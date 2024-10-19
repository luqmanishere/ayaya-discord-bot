//! This module contains commands used to manipulate playback

use crate::{
    error::BotError,
    utils::{check_msg, get_guild_id, ChannelInfo, GuildInfo, OptionExt as _},
    voice::{
        error::MusicCommandError,
        utils::{self, metadata_to_embed},
    },
    Context,
};

/// Pause the party. Time is frozen in this bubble universe."
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn pause(ctx: Context<'_>, _args: String) -> Result<(), BotError> {
    let guild_info = GuildInfo::from_ctx(ctx)?;

    let manager = &ctx.data().songbird;

    if let Some(handler_lock) = manager.get(guild_info.guild_id) {
        let handler = handler_lock.lock().await;
        let voice_channel_info =
            ChannelInfo::from_songbird_current_channel(ctx, handler.current_channel(), &guild_info)
                .await?;
        let queue = handler.queue();
        let track_uuid = queue.current().unwrap().uuid();
        let song_name = {
            let metadata_lock = ctx.data().track_metadata.lock().unwrap();
            metadata_lock
                .get(&track_uuid)
                .ok_or(MusicCommandError::TrackMetadataNotFound { track_uuid })?
                .title
                .clone()
                .unwrap_or_unknown()
        };
        queue
            .pause()
            .map_err(|e| MusicCommandError::FailedTrackPause {
                source: e,
                track_uuid,
                guild_info,
                voice_channel_info,
            })?;

        // TODO: replace these messages with embeds
        check_msg(
            ctx.channel_id()
                .say(ctx, format!("{} - paused", song_name))
                .await,
        );
    } else {
        // TODO: replace these messages with embeds
        check_msg(
            ctx.channel_id()
                .say(ctx, "Not in a voice channel to play in")
                .await,
        );
    }

    Ok(())
}

/// Resume the party. You hear a wind up sound as time speeds up.
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn resume(ctx: Context<'_>) -> Result<(), BotError> {
    let guild_info = GuildInfo::from_ctx(ctx)?;

    let manager = &ctx.data().songbird;

    if let Some(handler_lock) = manager.get(guild_info.guild_id) {
        let handler = handler_lock.lock().await;
        let voice_channel_info =
            ChannelInfo::from_songbird_current_channel(ctx, handler.current_channel(), &guild_info)
                .await?;
        let queue = handler.queue();
        let track_uuid = queue.current().unwrap().uuid();
        let song_name = {
            let metadata_lock = ctx.data().track_metadata.lock().unwrap();
            metadata_lock
                .get(&track_uuid)
                .ok_or(MusicCommandError::TrackMetadataNotFound { track_uuid })?
                .title
                .clone()
                .unwrap_or_unknown()
        };
        queue
            .resume()
            .map_err(|e| MusicCommandError::FailedTrackResume {
                source: e,
                track_uuid,
                guild_info,
                voice_channel_info,
            })?;

        check_msg(
            ctx.channel_id()
                .say(ctx, format!("{} - resumed", song_name))
                .await,
        );
    } else {
        return Err(MusicCommandError::BotVoiceNotJoined { guild_info }.into());
    }

    Ok(())
}

/// Stop all music and clear the queue. Will you stop by again?
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn stop(ctx: Context<'_>) -> Result<(), BotError> {
    let guild_info = GuildInfo::from_ctx(ctx)?;

    let manager = &ctx.data().songbird;

    if let Some(handler_lock) = manager.get(guild_info.guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        queue.stop();

        check_msg(ctx.channel_id().say(ctx, "queue cleared.").await);
    } else {
        return Err(MusicCommandError::BotVoiceNotJoined { guild_info }.into());
    }

    Ok(())
}

/// Skips the currently playing song. Ayaya wonders why you abandoned your summon so easily.
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn skip(ctx: Context<'_>) -> Result<(), BotError> {
    let guild_info = GuildInfo::from_ctx(ctx)?;

    let manager = &ctx.data().songbird;

    if let Some(handler_lock) = manager.get(guild_info.guild_id) {
        let handler = handler_lock.lock().await;
        let voice_channel_info =
            ChannelInfo::from_songbird_current_channel(ctx, handler.current_channel(), &guild_info)
                .await?;
        let queue = handler.queue();
        let track_uuid = queue.current().unwrap().uuid();
        let song_metadata = {
            let metadata_lock = ctx.data().track_metadata.lock().unwrap();
            metadata_lock
                .get(&track_uuid)
                .ok_or(MusicCommandError::TrackMetadataNotFound { track_uuid })?
                .clone()
        };
        queue
            .skip()
            .map_err(|e| MusicCommandError::FailedTrackSkip {
                source: e,
                track_uuid,
                guild_info,
                voice_channel_info,
            })?;

        let embed = metadata_to_embed(utils::EmbedOperation::SkipSong, &song_metadata, None);

        ctx.send(poise::CreateReply::default().embed(embed)).await?;
    } else {
        return Err(MusicCommandError::BotVoiceNotJoined { guild_info }.into());
    }

    Ok(())
}

/// Seeks the track to a position given in seconds
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn seek(ctx: Context<'_>, secs: u64) -> Result<(), BotError> {
    let guild_info = GuildInfo::from_ctx(ctx)?;

    let manager = &ctx.data().songbird;

    // TODO: polish and user error handling
    if let Some(handler) = manager.get(guild_info.guild_id) {
        let handler = handler.lock().await;
        let voice_channel_info =
            ChannelInfo::from_songbird_current_channel(ctx, handler.current_channel(), &guild_info)
                .await?;
        match handler.queue().current() {
            Some(track) => {
                let data = ctx.data();
                let track_uuid = track.uuid();
                let metadata = {
                    let lock = data.track_metadata.lock().unwrap();
                    lock.get(&track_uuid)
                        .ok_or(MusicCommandError::TrackMetadataNotFound { track_uuid })?
                        .clone()
                };
                track
                    .seek(std::time::Duration::from_secs(secs))
                    .result()
                    .map_err(|e| MusicCommandError::FailedTrackSeek {
                        source: e,
                        track_uuid,
                        guild_info,
                        voice_channel_info,
                        position: secs,
                    })?;
                let song_name = metadata.title.clone().unwrap();
                let channel_name = metadata.channel.clone().unwrap();

                // TODO: express in embed
                check_msg(
                    ctx.channel_id()
                        .say(
                            ctx,
                            format!(
                                "Seek track: `{} ({})` to {} seconds",
                                song_name, channel_name, secs
                            ),
                        )
                        .await,
                );
            }
            None => {
                let voice_channel_info = ChannelInfo::from_songbird_current_channel(
                    ctx,
                    handler.current_channel(),
                    &guild_info,
                )
                .await?;
                return Err(MusicCommandError::NoTrackToSeek {
                    guild_info,
                    voice_channel_info,
                }
                .into());
            }
        };
    } else {
        return Err(MusicCommandError::BotVoiceNotJoined { guild_info }.into());
    }

    Ok(())
}

#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(rename = "loop", slash_command, prefix_command, guild_only)]
pub async fn loop_track(ctx: Context<'_>, count: Option<usize>) -> Result<(), BotError> {
    let guild_info = GuildInfo::from_ctx(ctx)?;

    let manager = &ctx.data().songbird;

    if let Some(handler) = manager.get(guild_info.guild_id) {
        let handler = handler.lock().await;
        let voice_channel_info =
            ChannelInfo::from_songbird_current_channel(ctx, handler.current_channel(), &guild_info)
                .await?;
        match handler.queue().current() {
            Some(track) => {
                let data = ctx.data();
                let track_uuid = track.uuid();
                let metadata = {
                    let lock = data.track_metadata.lock().unwrap();
                    lock.get(&track_uuid)
                        .ok_or(MusicCommandError::TrackMetadataNotFound { track_uuid })?
                        .clone()
                };

                match count {
                    Some(count) => {
                        track
                            .loop_for(count)
                            .map_err(|e| MusicCommandError::FailedTrackLoop {
                                source: e,
                                guild_info,
                                voice_channel_info,
                                count: Some(count),
                            })?;

                        let embed = metadata_to_embed(
                            utils::EmbedOperation::LoopCount(count),
                            &metadata,
                            None,
                        );
                        ctx.send(poise::CreateReply::default().embed(embed)).await?;
                    }
                    None => {
                        track
                            .enable_loop()
                            .map_err(|e| MusicCommandError::FailedTrackLoop {
                                source: e,
                                guild_info,
                                voice_channel_info,
                                count: None,
                            })?;

                        let embed = metadata_to_embed(
                            utils::EmbedOperation::LoopIndefinite,
                            &metadata,
                            None,
                        );
                        ctx.send(poise::CreateReply::default().embed(embed)).await?;
                    }
                }
            }
            None => {
                let voice_channel_info = ChannelInfo::from_songbird_current_channel(
                    ctx,
                    handler.current_channel(),
                    &guild_info,
                )
                .await?;
                return Err(MusicCommandError::NoTrackToSeek {
                    guild_info,
                    voice_channel_info,
                }
                .into());
            }
        };
    } else {
        return Err(MusicCommandError::BotVoiceNotJoined { guild_info }.into());
    }
    Ok(())
}

#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(rename = "stoploop", slash_command, prefix_command, guild_only)]
pub async fn stop_loop(ctx: Context<'_>) -> Result<(), BotError> {
    let guild_info = GuildInfo::from_ctx(ctx)?;

    let manager = &ctx.data().songbird;

    if let Some(handler) = manager.get(guild_info.guild_id) {
        let handler = handler.lock().await;
        let voice_channel_info =
            ChannelInfo::from_songbird_current_channel(ctx, handler.current_channel(), &guild_info)
                .await?;
        match handler.queue().current() {
            Some(track) => {
                let data = ctx.data();
                let track_uuid = track.uuid();
                let metadata = {
                    let lock = data.track_metadata.lock().unwrap();
                    lock.get(&track_uuid)
                        .ok_or(MusicCommandError::TrackMetadataNotFound { track_uuid })?
                        .clone()
                };

                track
                    .disable_loop()
                    .map_err(|e| MusicCommandError::FailedTrackLoop {
                        source: e,
                        guild_info,
                        voice_channel_info,
                        count: None,
                    })?;

                let embed = metadata_to_embed(utils::EmbedOperation::StopLoop, &metadata, None);
                ctx.send(poise::CreateReply::default().embed(embed)).await?;
            }
            None => {
                let voice_channel_info = ChannelInfo::from_songbird_current_channel(
                    ctx,
                    handler.current_channel(),
                    &guild_info,
                )
                .await?;
                return Err(MusicCommandError::NoTrackToSeek {
                    guild_info,
                    voice_channel_info,
                }
                .into());
            }
        };
    } else {
        return Err(MusicCommandError::BotVoiceNotJoined { guild_info }.into());
    }
    Ok(())
}

/// Leaves the current voice channel. Ever wonder what happens to Ayaya then?
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn leave(ctx: Context<'_>) -> Result<(), BotError> {
    let guild_info = GuildInfo::from_ctx(ctx)?;

    let manager = &ctx.data().songbird;

    if let Some(handler_lock) = manager.get(guild_info.guild_id) {
        let voice_channel_info = {
            let handler = handler_lock.lock().await;
            ChannelInfo::from_songbird_current_channel(ctx, handler.current_channel(), &guild_info)
                .await?
        };

        if let Err(e) = manager.remove(guild_info.guild_id).await {
            return Err(MusicCommandError::FailedLeaveCall {
                source: e,
                guild_info,
                voice_channel_info,
            }
            .into());
        }

        // TODO: replace with embeds
        check_msg(ctx.channel_id().say(ctx, "Left voice channel").await);
    } else {
        return Err(MusicCommandError::BotVoiceNotJoined { guild_info }.into());
    }

    Ok(())
}
