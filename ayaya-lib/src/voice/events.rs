use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};

use poise::serenity_prelude::{self as serenity, UserId};
use serenity::{
    async_trait,
    http::Http,
    model::{id::GuildId, prelude::ChannelId},
    Context as SerenityContext,
};
use songbird::{
    tracks::PlayMode, Event, EventContext, EventHandler as VoiceEventHandler, Songbird,
};
use tracing::{error, info};

use super::utils::{metadata_to_embed, EmbedOperation, YoutubeMetadata};
use crate::utils::check_msg;

pub struct SongFader {
    pub chan_id: ChannelId,
    pub http: Arc<Http>,
}

#[async_trait]
impl VoiceEventHandler for SongFader {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(&[(state, track)]) = ctx {
            let _ = track.set_volume(state.volume / 2.0);

            if state.volume < 1e-2 {
                let _ = track.stop();
                check_msg(self.chan_id.say(&self.http, "Stopping song...").await);
                Some(Event::Cancel)
            } else {
                check_msg(self.chan_id.say(&self.http, "Volume reduced.").await);
                None
            }
        } else {
            None
        }
    }
}

/// Bot inactive counter. Will start counting when song ends, is stopped or paused.
/// The check is ran every 60 seconds, so the 5 minutes actually has a margin
/// of 1 min. Also starts counting when the bot is alone in the voice channel
pub struct BotInactiveCounter {
    pub channel_id: ChannelId,
    pub guild_id: GuildId,
    pub bot_user_id: UserId,
    pub ctx: SerenityContext,
    pub manager: Arc<Songbird>,
    pub counter: Arc<AtomicUsize>,
    pub linger: Arc<AtomicBool>,
}

#[derive(Debug)]
enum Status {
    Alone,
    PlaybackFinished,
    Playback,
    Inactive,
    Linger,
    Other(String),
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            " Status: {}",
            match self {
                Status::Alone => "Bot is alone",
                Status::PlaybackFinished => "Playback is finished",
                Status::Playback => "Bot is playing music",
                Status::Inactive => "Bot is inactive",
                Status::Linger => "Linger is active.",
                Status::Other(ref other) => other,
            }
        )
    }
}

impl BotInactiveCounter {
    async fn check_inactive(&self) -> (bool, Option<Status>) {
        if let Some(handler) = self.manager.get(self.guild_id) {
            // check if we are alone in the channel
            let alone_in_channel = {
                match self.guild_id.to_guild_cached(&self.ctx) {
                    Some(guild) => {
                        let voice_states = guild.clone().voice_states;
                        voice_states.len() == 1 && voice_states.contains_key(&self.bot_user_id)
                    }
                    None => true,
                }
            };
            let linger = self.linger.load(Ordering::Relaxed);

            // skip queue checks if linger is on
            if linger {
                (alone_in_channel, Some(Status::Linger))
            } else {
                tracing::info!("linger is off");

                // first check if we are alone
                if alone_in_channel {
                    return (true, Some(Status::Alone));
                }

                // then check the queue
                let queue = handler.lock().await.queue().clone();
                let play_mode = match queue.current() {
                    Some(current) => match current.get_info().await {
                        Ok(state) => Some(state.playing),
                        Err(_) => None,
                    },
                    None => None,
                };

                // if linger is not on, we leave when the bot stops playing, or is inactive
                match play_mode {
                    Some(PlayMode::Pause | PlayMode::Stop | PlayMode::End) => {
                        (true, Some(Status::PlaybackFinished))
                    }
                    Some(PlayMode::Play) => (false, Some(Status::Playback)),
                    other => (
                        true,
                        Some(other.map_or(Status::Inactive, |e| Status::Other(format!("{e:?}")))),
                    ),
                }
            }
        } else {
            // if not in channel, leave
            (true, None)
        }
    }
}

#[async_trait]
impl VoiceEventHandler for BotInactiveCounter {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let check_inactive = self.check_inactive().await;
        match check_inactive {
            (true, status) => {
                let status = status.map_or(String::default(), |e| e.to_string());

                let counter_before = self.counter.fetch_add(1, Ordering::Relaxed);
                info!(
                    "Counter for channel {} in guild {} is {}/5.{status}.",
                    self.channel_id,
                    self.guild_id,
                    counter_before + 1
                );
            }
            (false, status) => {
                let status = status.map_or(String::default(), |e| e.to_string());

                self.counter.store(0, Ordering::Relaxed);
                info!(
                    "Counter for channel {} in guild {} is reset to {}/5.{status}",
                    self.channel_id,
                    self.guild_id,
                    self.counter.load(Ordering::Relaxed)
                );
            }
        }

        let counter = self.counter.load(Ordering::Relaxed);
        if counter >= 5 {
            // Leave the voice channel
            let manager = &self.manager;

            if let Err(e) = manager.remove(self.guild_id).await {
                check_msg(
                    self.channel_id
                        .say(&self.ctx.http, format!("Failed: {:?}", e))
                        .await,
                );
                error!("Failed: {:?}", e);
            }

            check_msg(
                self.channel_id
                    .say(&self.ctx.http, "Left voice channel after 5 minutes of inactivity. Ayaya got bored without you, you know")
                    .await,
            );
            info!(
                "Left voice channel {} in guild {} for 5 minutes of inactivity",
                self.channel_id, self.guild_id
            );
            return None;
        }

        None
    }
}

/// Notify the calling channel when a track starts to play
pub struct TrackPlayNotifier {
    pub channel_id: ChannelId,
    pub metadata: YoutubeMetadata,
    pub http: Arc<Http>,
}

#[async_trait]
impl VoiceEventHandler for TrackPlayNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(tracks) = ctx {
            let tracks = tracks.to_vec();
            for (track_state, _) in tracks {
                if track_state.playing == PlayMode::Play {
                    self.channel_id
                        .send_message(
                            self.http.clone(),
                            serenity::CreateMessage::default().embed(metadata_to_embed(
                                EmbedOperation::NowPlayingNotification,
                                &self.metadata,
                                None,
                            )),
                        )
                        .await
                        .expect("message sent");
                    return None;
                }
            }
        }
        Some(Event::Track(songbird::TrackEvent::Play))
    }
}

// pub struct VoiceLeaveCleanup {
//     pub channel_id: ChannelId,
//     pub guild_id: GuildId,
//     pub ctx: SerenityContext,
//     pub manager: Arc<Songbird>,
//     pub counter: Arc<AtomicUsize>,
//     pub linger_map: Arc<Mutex<HashMap<GuildId, Arc<AtomicBool>>>>,
//     pub bot_user_id: UserId,
// }

// #[async_trait]
// impl VoiceEventHandler for VoiceLeaveCleanup {
//     async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
//         if let EventContext::(ev) = ctx {
//             if let Some(dis) = ev.reason {
//                 match dis {
//                     songbird::events::context_data::DisconnectReason::AttemptDiscarded => todo!(),
//                     songbird::events::context_data::DisconnectReason::Internal => todo!(),
//                     songbird::events::context_data::DisconnectReason::Io => todo!(),
//                     songbird::events::context_data::DisconnectReason::ProtocolViolation => todo!(),
//                     songbird::events::context_data::DisconnectReason::TimedOut => todo!(),
//                     songbird::events::context_data::DisconnectReason::Requested => todo!(),
//                     songbird::events::context_data::DisconnectReason::WsClosed(close_code) => {
//                         todo!()
//                     }
//                     _ => todo!(),
//                 }
//             }
//             tracing::info!("removed from voice channel");
//             self.manager.remove(self.guild_id).await;
//         }
//         None
//     }
// }
