use std::sync::{
    atomic::{AtomicUsize, Ordering},
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
    input::AuxMetadata, tracks::PlayMode, Event, EventContext, EventHandler as VoiceEventHandler,
    Songbird,
};
use tracing::{error, info};

use super::utils::{metadata_to_embed, EmbedOperation};
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
}

#[async_trait]
impl VoiceEventHandler for BotInactiveCounter {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        // TODO: simplify
        if let Some(handler_lock) = self.manager.get(self.guild_id) {
            let handler = handler_lock.lock().await;
            // TODO: clarify reasons for leaving, not playing or no listener

            let alone_in_channel = {
                match self.guild_id.to_guild_cached(&self.ctx) {
                    Some(guild) => {
                        let voice_states = guild.clone().voice_states;
                        voice_states.len() == 1 && voice_states.contains_key(&self.bot_user_id)
                    }
                    None => true,
                }
            };

            let queue = handler.queue();
            match queue.current() {
                Some(track) => {
                    let track_state = track.get_info().await.unwrap();
                    if track_state.playing == PlayMode::End
                        || track_state.playing == PlayMode::Pause
                        || track_state.playing == PlayMode::Stop
                        || alone_in_channel
                    {
                        let counter_before = self.counter.fetch_add(1, Ordering::Relaxed);
                        info!(
                            "Counter for channel {} in guild {} is {}/5",
                            self.channel_id,
                            self.guild_id,
                            counter_before + 1
                        );
                    } else {
                        self.counter.store(0, Ordering::Relaxed);
                        info!(
                            "Counter for channel {} in guild {} is reset to {}/5",
                            self.channel_id,
                            self.guild_id,
                            self.counter.load(Ordering::Relaxed)
                        );
                    }
                }
                None => {
                    let counter_before = self.counter.fetch_add(1, Ordering::Relaxed);
                    info!(
                        "Counter for channel {} in guild {} is {}/5",
                        self.channel_id,
                        self.guild_id,
                        counter_before + 1
                    );
                }
            }
        } else {
            error!("Not in a voice channel?? TF????");
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
    pub metadata: AuxMetadata,
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
