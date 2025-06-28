//! This module contains commands used to manipulate playback

use poise::serenity_prelude as serenity;

use crate::{
    error::BotError,
    utils::{check_msg, get_guild_id, ChannelInfo, GuildInfo, OptionExt},
    voice::{
        error::MusicCommandError,
        utils::{self, metadata_to_embed, YoutubeMetadata},
    },
    Context,
};

/// Pause the party. Time is frozen in this bubble universe."
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, guild_only, category = "Music")]
pub async fn pause(ctx: Context<'_>, _args: String) -> Result<(), BotError> {
    let guild_info = GuildInfo::from_ctx(ctx)?;

    let manager = &ctx.data().songbird;

    if let Some(handler_lock) = manager.get(guild_info.guild_id) {
        let handler = handler_lock.lock().await;
        let voice_channel_info =
            ChannelInfo::from_songbird_current_channel(ctx, handler.current_channel(), &guild_info)
                .await?;
        let queue = handler.queue();
        let track_uuid = queue.current().expect("unable to get current track").uuid();
        let song_name = queue
            .current()
            .expect("unable to get current track")
            .data::<YoutubeMetadata>()
            .title
            .clone()
            .unwrap_or_unknown();
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
                .say(ctx, format!("{song_name} - paused"))
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
#[poise::command(slash_command, prefix_command, guild_only, category = "Music")]
pub async fn resume(ctx: Context<'_>) -> Result<(), BotError> {
    let guild_info = GuildInfo::from_ctx(ctx)?;

    let manager = &ctx.data().songbird;

    if let Some(handler_lock) = manager.get(guild_info.guild_id) {
        let handler = handler_lock.lock().await;
        let voice_channel_info =
            ChannelInfo::from_songbird_current_channel(ctx, handler.current_channel(), &guild_info)
                .await?;
        let queue = handler.queue();
        let track_uuid = queue.current().expect("unable to get current track").uuid();
        let song_name = queue
            .current()
            .expect("unable to get current track")
            .data::<YoutubeMetadata>()
            .title
            .clone()
            .unwrap_or_unknown();

        queue
            .resume()
            .map_err(|e| MusicCommandError::FailedTrackResume {
                source: e,
                track_uuid,
                guild_info,
                voice_channel_info,
            })?;

        // TODO: embed
        check_msg(
            ctx.channel_id()
                .say(ctx, format!("{song_name} - resumed"))
                .await,
        );
    } else {
        return Err(MusicCommandError::BotVoiceNotJoined { guild_info }.into());
    }

    Ok(())
}

/// Stop all music and clear the queue. Will you stop by again?
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, guild_only, category = "Music")]
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
#[poise::command(slash_command, prefix_command, guild_only, category = "Music")]
pub async fn skip(ctx: Context<'_>) -> Result<(), BotError> {
    let guild_info = GuildInfo::from_ctx(ctx)?;

    let manager = &ctx.data().songbird;

    if let Some(handler_lock) = manager.get(guild_info.guild_id) {
        let handler = handler_lock.lock().await;
        let voice_channel_info =
            ChannelInfo::from_songbird_current_channel(ctx, handler.current_channel(), &guild_info)
                .await?;
        let queue = handler.queue();
        let track_uuid = queue.current().expect("unable to get current track").uuid();
        let song_metadata = queue
            .current()
            .expect("unable to get current track")
            .data::<YoutubeMetadata>();

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
#[poise::command(slash_command, prefix_command, guild_only, category = "Music")]
pub async fn seek(
    ctx: Context<'_>,
    #[description = "Time in seconds to seek to. Can only seek forwards! Max is -5s from end."]
    #[autocomplete = "autocomplete_seek"]
    secs: u64,
) -> Result<(), BotError> {
    let guild_info = GuildInfo::from_ctx(ctx)?;

    let manager = &ctx.data().songbird;

    if let Some(handler) = manager.get(guild_info.guild_id) {
        let handler = handler.lock().await;
        let voice_channel_info =
            ChannelInfo::from_songbird_current_channel(ctx, handler.current_channel(), &guild_info)
                .await?;
        match handler.queue().current() {
            Some(track) => {
                let track_uuid = track.uuid();
                let metadata = track.data::<YoutubeMetadata>();

                if let Some(max_track_duration) = metadata.duration() {
                    let max_track_position = max_track_duration.as_secs() - 5;
                    let track_state = track.get_info().await.map_err(|e| {
                        MusicCommandError::TrackStateNotFound {
                            source: e,
                            track_uuid,
                        }
                    })?;
                    let current_position = track_state.position.as_secs();

                    // check if less than current pos, or more than max
                    if secs < current_position {
                        return Err(MusicCommandError::NoSeekBackwards {
                            guild_info,
                            voice_channel_info,
                            requested_position: secs,
                            current_position,
                        }
                        .into());
                    } else if secs > max_track_position {
                        return Err(MusicCommandError::SeekOutOfBounds {
                            guild_info,
                            voice_channel_info,
                            requested_position: secs,
                            max_position: max_track_position,
                        }
                        .into());
                    }

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

                    let new_track_info = track.get_info().await.ok();

                    let embed = metadata_to_embed(
                        utils::EmbedOperation::Seek(secs),
                        &metadata,
                        new_track_info.as_ref(),
                    );
                    ctx.send(poise::CreateReply::default().embed(embed)).await?;
                } else {
                    return Err(MusicCommandError::NoDurationNoSeek {
                        guild_info,
                        voice_channel_info,
                        track_uuid,
                    }
                    .into());
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

async fn autocomplete_seek(
    ctx: Context<'_>,
    partial: &str,
) -> impl Iterator<Item = serenity::AutocompleteChoice> {
    fn template(partial: u64) -> Vec<serenity::AutocompleteChoice> {
        vec![serenity::AutocompleteChoice::new(
            format!("Selection: {partial}s"),
            partial,
        )]
    }

    let partial = partial.parse::<u64>().unwrap_or_default();

    let manager = ctx.data().songbird.clone();
    let Ok(guild_info) = GuildInfo::from_ctx(ctx) else {
        return template(partial).into_iter();
    };
    let current = manager.get(guild_info.guild_id);
    if let Some(handler) = current {
        let queue = handler.lock().await.queue().clone();
        drop(handler);
        let Some(current) = queue.current() else {
            return template(partial).into_iter();
        };

        let Some(duration) = current.data::<YoutubeMetadata>().duration() else {
            return vec![].into_iter();
        };
        let current_duration = {
            let Ok(track_state) = current.get_info().await else {
                return template(partial).into_iter();
            };

            track_state.position.as_secs()
        };
        let max_duration = duration.as_secs() - 5;

        let mut complete = template(partial);
        complete.push(serenity::AutocompleteChoice::new(
            format!("Max: {max_duration}s | Current: {current_duration}s"),
            max_duration,
        ));

        // indicate if less
        if partial <= current_duration {
            complete.push(serenity::AutocompleteChoice::new(
                format!("ERROR: {partial}s is <= current position"),
                partial,
            ));
        }

        // indicate if more
        if partial > max_duration {
            complete.push(serenity::AutocompleteChoice::new(
                format!("ERROR: {partial}s is > max position"),
                partial,
            ));
        }

        complete.into_iter()
    } else {
        template(partial).into_iter()
    }
}

/// Loops the current track. Leave empty for an indefinite loop.
///
/// Round and round the Ayaya goes...
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(
    rename = "loop",
    slash_command,
    prefix_command,
    guild_only,
    category = "Music"
)]
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
                let metadata = track.data::<YoutubeMetadata>();

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

/// Stops the current track from any loops.
///
/// Ayaya is already dizzy...
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(
    rename = "stoploop",
    slash_command,
    prefix_command,
    guild_only,
    category = "Music"
)]
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
                let metadata = track.data::<YoutubeMetadata>();

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
#[poise::command(slash_command, prefix_command, guild_only, category = "Music")]
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
