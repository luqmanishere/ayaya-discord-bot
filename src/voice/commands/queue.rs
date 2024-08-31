//! This module contains commands for queue manipulation (excluding addition)

use poise::serenity_prelude as serenity;
use tracing::error;

use crate::{
    error::BotError,
    utils::{get_guild_id, ChannelInfo, GuildInfo},
    voice::{
        error::MusicCommandError,
        utils::{self, error_embed, metadata_to_embed},
    },
    Context,
};

/// Shows the queue. The only kind of acceptable spoilers.
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, aliases("q"), guild_only)]
pub async fn queue(ctx: Context<'_>) -> Result<(), BotError> {
    // TODO Implement queue viewing
    let guild_id = get_guild_id(ctx)?;
    let guild_info = GuildInfo::from_ctx(ctx)?;

    let manager = &ctx.data().songbird;

    // Check if in channel
    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let tracks = queue.current_queue();
        let mut lines = serenity::MessageBuilder::new();
        {
            let data = ctx.data();
            let metadata_lock = data.track_metadata.lock().unwrap();

            // TODO: replace with embed
            if tracks.is_empty() {
                lines.push_line("# Nothing in queue");
            } else {
                lines.push_line("# Queue");
            }
            for (i, track) in tracks.iter().enumerate() {
                let track_uuid = track.uuid();
                let metadata = metadata_lock
                    .get(&track_uuid)
                    .ok_or(MusicCommandError::TrackMetadataNotFound { track_uuid })?;

                lines.push_quote_line(format!(
                    "{}. {} ({})",
                    i + 1,
                    metadata.title.as_ref().unwrap(),
                    metadata.channel.as_ref().unwrap()
                ));
            }
        }

        let embed = serenity::CreateEmbed::new()
            .colour(serenity::Colour::MEIBE_PINK)
            .description(lines.to_string());

        ctx.send(poise::CreateReply::default().embed(embed)).await?;
    } else {
        return Err(MusicCommandError::BotVoiceNotJoined { guild_info }.into());
    }
    //TODO check for

    Ok(())
}

/// Delete song from queue. Being able to make things go *poof* makes you feel like a Kami-sama, right?
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, guild_only, aliases("d"))]
pub async fn delete(ctx: Context<'_>, queue_position: usize) -> Result<(), BotError> {
    let guild_info = GuildInfo::from_ctx(ctx)?;

    let manager = ctx.data().songbird.clone();

    if let Some(handler_lock) = manager.get(guild_info.guild_id) {
        let voice_channel_info = {
            let handler = handler_lock.lock().await;
            ChannelInfo::from_songbird_current_channel(ctx, handler.current_channel(), &guild_info)
                .await?
        };
        // If not empty, remove the songs
        if queue_position != 0 {
            let handler = handler_lock.lock().await;
            let queue = handler.queue();
            if queue_position != 1 {
                let index = queue_position - 1;
                if let Some(track) = queue.current_queue().get(index) {
                    let track_uuid = track.uuid();
                    let track_metadata = ctx.data().track_metadata.clone();
                    let metadata = {
                        let lock = track_metadata.lock().unwrap();
                        lock.get(&track_uuid)
                            .ok_or(MusicCommandError::TrackMetadataNotFound { track_uuid })?
                            .clone()
                    };
                    if queue.dequeue(index).is_some() {
                        ctx.send(poise::CreateReply::default().embed(metadata_to_embed(
                            utils::EmbedOperation::DeleteFromQueue,
                            &metadata,
                            None,
                        )))
                        .await?;
                    } else {
                        // TODO: notify user of error
                        error!(
                            "Index {index} does not exist in queue for guild {}",
                            guild_info.guild_id
                        );
                        return Err(MusicCommandError::QueueOutOfBounds {
                            index,
                            guild_info,
                            voice_channel_info,
                        }
                        .into());
                    }
                } else {
                    return Err(MusicCommandError::QueueOutOfBounds {
                        index,
                        guild_info,
                        voice_channel_info,
                    }
                    .into());
                }
            } else {
                return Err(MusicCommandError::QueueDeleteNowPlaying {
                    guild_info,
                    voice_channel_info,
                }
                .into());
            }
        } else {
            // TODO: zero is an error
        }
    } else {
        return Err(MusicCommandError::BotVoiceNotJoined { guild_info }.into());
    }

    Ok(())
}

/// "Shows what song is currently playing. Ayaya really knows everything about herself."
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, aliases("np"), guild_only)]
pub async fn nowplaying(ctx: Context<'_>) -> Result<(), BotError> {
    let guild_id = get_guild_id(ctx)?;

    let manager = &ctx.data().songbird;

    if let Some(handler) = manager.get(guild_id) {
        let handler = handler.lock().await;
        match handler.queue().current() {
            Some(track) => {
                let data = ctx.data();
                let track_uuid = track.uuid();
                let track_state =
                    track
                        .get_info()
                        .await
                        .map_err(|e| MusicCommandError::TrackStateNotFound {
                            source: e,
                            track_uuid,
                        })?;
                let metadata = {
                    let lock = data.track_metadata.lock().unwrap();
                    lock.get(&track_uuid)
                        .ok_or(MusicCommandError::TrackMetadataNotFound { track_uuid })?
                        .clone()
                };

                ctx.send(poise::CreateReply::default().embed(metadata_to_embed(
                    utils::EmbedOperation::NowPlaying,
                    &metadata,
                    Some(&track_state),
                )))
                .await?;
            }
            None => {
                ctx.send(poise::CreateReply::default().embed(metadata_to_embed(
                    utils::EmbedOperation::NowPlaying,
                    &songbird::input::AuxMetadata::default(),
                    None,
                )))
                .await?;
            }
        };
    } else {
        ctx.send(
            poise::CreateReply::default()
                .embed(error_embed(utils::EmbedOperation::ErrorNotInVoiceChannel)),
        )
        .await?;
    }

    Ok(())
}
