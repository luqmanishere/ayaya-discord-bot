use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use serenity::{
    async_trait,
    http::Http,
    model::{id::GuildId, prelude::ChannelId},
    prelude::*,
};

use songbird::{
    tracks::PlayMode, Event, EventContext, EventHandler as VoiceEventHandler, Songbird,
};
use tracing::{error, info};

use crate::utils::check_msg;

pub struct ChannelDurationNotifier {
    pub chan_id: ChannelId,
    pub count: Arc<AtomicUsize>,
    pub http: Arc<Http>,
}

#[async_trait]
impl VoiceEventHandler for ChannelDurationNotifier {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        /*
        let count_before = self.count.fetch_add(1, Ordering::Relaxed);
        check_msg(
            self.chan_id
                .say(
                    &self.http,
                    &format!(
                        "I've been in this channel for {} minutes!",
                        count_before + 1
                    ),
                )
                .await,
        );
        */

        None
    }
}

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
/// of 1 min.
pub struct BotInactiveCounter {
    pub channel_id: ChannelId,
    pub guild_id: GuildId,
    pub ctx: Context,
    pub manager: Arc<Songbird>,
    pub counter: Arc<AtomicUsize>,
}

#[async_trait]
#[allow(unused_variables)]
impl VoiceEventHandler for BotInactiveCounter {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let Some(handler_lock) = self.manager.get(self.guild_id) {
            let handler = handler_lock.lock().await;
            let queue = handler.queue();
            match queue.current() {
                Some(track) => {
                    let track_state = track.get_info().await.unwrap();
                    if track_state.playing == PlayMode::End
                        || track_state.playing == PlayMode::Pause
                        || track_state.playing == PlayMode::Stop
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
            let manager = songbird::get(&self.ctx)
                .await
                .expect("Songbird Voice client placed in at initialisation.")
                .clone();

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
