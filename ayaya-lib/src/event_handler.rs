use std::{
    io::{BufRead, BufReader},
    sync::Arc,
};

use ayaya_db::data::voice::{VoiceSessionEndReason, VoiceStateUpdateInput};
use serenity::all::{ActivityData, CacheHttp, Context, EventHandler, FullEvent};
use time::OffsetDateTime;

use crate::{Data, setup_cookies};

pub struct StartupHandler;

#[serenity::async_trait]
impl EventHandler for StartupHandler {
    async fn dispatch(&self, context: &Context, event: &FullEvent) {
        match event {
            FullEvent::Ready { data_about_bot, .. } => {
                println!("Ready is called!");
                // TODO: migrate setup function
                tracing::info!("Setup is running after Ready Event");
                let data: Arc<Data> = context.data();
                let commands = data.commands.iter().collect::<Vec<_>>();
                poise::builtins::register_globally(context.http(), commands)
                    .await
                    .expect("Error registering commands");

                let bot_user_name = &data_about_bot.user.name;
                let session_id = &data_about_bot.session_id;
                let bot_user_id = data_about_bot.user.id;
                tracing::info!(
                    "Logged in as {} with session id {}.",
                    bot_user_name,
                    session_id
                );

                {
                    let mut user_id_lock = data.user_id.write().await;
                    *user_id_lock = bot_user_id;
                }

                // TODO: handle this error
                setup_cookies(
                    &data.data_manager,
                    &data.ytdlp_config_path,
                    &data.secret_key,
                )
                .await
                .expect("handle this error somehow");

                // test yt-dlp
                #[expect(clippy::zombie_processes)]
                let child = std::process::Command::new("yt-dlp")
                    .arg("-v")
                    // .arg("--extractor-args")
                    // .arg("youtube:player_client=web_creator,mweb")
                    .arg("-O")
                    .arg("title,channel")
                    .arg("https://www.youtube.com/watch?v=1aPOj0ERTEc")
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .expect("yt-dlp runs");
                let stderr = child
                    .stderr
                    .ok_or_else(|| std::io::Error::other("Could not capture stdout"))
                    .expect("cant get yt-dlp stdout");

                let reader = BufReader::new(stderr);

                reader
                    .lines()
                    .map_while(Result::ok)
                    .for_each(|line| tracing::info!("yt-dlp setup: {}", line));
                tracing::info!("yt-dlp checks done");
                context.set_activity(Some(ActivityData::watching("Hoshimachi Suichan")));
            }
            FullEvent::CacheReady { guilds, .. } => {
                tracing::info!("Cached guild info is ready for {} guilds.", guilds.len());
                reconcile_cached_voice_states(
                    context,
                    guilds.as_slice(),
                    OffsetDateTime::now_utc(),
                )
                .await;
            }
            FullEvent::VoiceStateUpdate { old, new, .. } => {
                persist_voice_state_update(context, old.as_ref(), new).await;
            }
            _ => {}
        }
    }
}

async fn persist_voice_state_update(
    context: &Context,
    old: Option<&serenity::all::VoiceState>,
    new: &serenity::all::VoiceState,
) {
    let Some(guild_id) = new
        .guild_id
        .or_else(|| old.and_then(|state| state.guild_id))
    else {
        tracing::debug!("Skipping voice state update without guild id");
        return;
    };

    let input = VoiceStateUpdateInput {
        guild_id: guild_id.get() as i64,
        user_id: new.user_id.get() as i64,
        from_channel_id: old
            .and_then(|state| state.channel_id)
            .map(|channel_id| channel_id.get() as i64),
        to_channel_id: new.channel_id.map(|channel_id| channel_id.get() as i64),
        occurred_at: OffsetDateTime::now_utc(),
        self_mute: new.self_mute(),
        self_deaf: new.self_deaf(),
        mute: new.mute(),
        deaf: new.deaf(),
        self_stream: new.self_stream().unwrap_or(false),
        self_video: new.self_video(),
        suppress: new.suppress(),
        request_to_speak_at: new
            .request_to_speak_timestamp
            .and_then(timestamp_to_offset_datetime),
        raw_state_json: serde_json::to_string(new).ok(),
        start_is_estimated: old.is_none(),
    };

    let data: Arc<Data> = context.data();
    if let Err(error) = data
        .data_manager
        .voice()
        .apply_voice_state_update(input)
        .await
    {
        tracing::error!("Failed to persist voice state update: {error}");
    }
}

async fn reconcile_cached_voice_states(
    context: &Context,
    guild_ids: &[serenity::all::GuildId],
    startup_time: OffsetDateTime,
) {
    let data: Arc<Data> = context.data();
    let voice_manager = data.data_manager.voice();

    if let Err(error) = voice_manager
        .close_all_open_voice_sessions(startup_time, VoiceSessionEndReason::BotRestart)
        .await
    {
        tracing::error!("Failed to close open voice sessions during reconciliation: {error}");
        return;
    }

    for guild_id in guild_ids {
        let active_voice_states = {
            let Some(guild) = guild_id.to_guild_cached(&context.cache) else {
                continue;
            };

            guild
                .voice_states
                .iter()
                .filter_map(|state| {
                    let channel_id = state.channel_id?;
                    Some((
                        state.user_id.get() as i64,
                        channel_id.get() as i64,
                        serde_json::to_string(state).ok(),
                    ))
                })
                .collect::<Vec<_>>()
        };

        for (user_id, channel_id, raw_state_json) in active_voice_states {
            if let Err(error) = voice_manager
                .ensure_open_voice_session(
                    guild_id.get() as i64,
                    user_id,
                    channel_id,
                    startup_time,
                    raw_state_json,
                )
                .await
            {
                tracing::error!(
                    "Failed to reconcile voice session for guild {} user {}: {}",
                    guild_id,
                    user_id,
                    error
                );
            }
        }
    }
}

fn timestamp_to_offset_datetime(timestamp: serenity::all::Timestamp) -> Option<OffsetDateTime> {
    OffsetDateTime::from_unix_timestamp(timestamp.unix_timestamp()).ok()
}
