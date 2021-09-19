use std::sync::{atomic::AtomicUsize, Arc};

use serenity::{async_trait, http::Http, model::prelude::ChannelId};

use songbird::{Event, EventContext, EventHandler as VoiceEventHandler};

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

pub struct SongAfter60 {
    pub chan_id: ChannelId,
    pub http: Arc<Http>,
}

#[async_trait]
#[allow(unused_variables)]
impl VoiceEventHandler for SongAfter60 {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        None
    }
}
