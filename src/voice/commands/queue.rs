//! This module contains commands for queue manipulation (excluding addition)

use poise::serenity_prelude as serenity;
use tracing::error;

use crate::{
    error::BotError,
    utils::{get_guild_id, ChannelInfo, GuildInfo, OptionExt},
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
        let queue_vec = {
            let data = ctx.data();
            let metadata_lock = data.track_metadata.lock().unwrap();
            let mut queue_vec = vec![];

            // TODO: replace with embed
            for (index, track) in tracks.iter().enumerate() {
                let track_uuid = track.uuid();
                let metadata = metadata_lock
                    .get(&track_uuid)
                    .ok_or(MusicCommandError::TrackMetadataNotFound { track_uuid })?;
                let rendered = format!(
                    "{}. {} | Channel: {}",
                    index,
                    metadata.title.clone().unwrap_or_unknown(),
                    metadata.channel.clone().unwrap_or_unknown()
                );
                queue_vec.push(rendered);
            }
            queue_vec
        };
        // TODO: pagination
        if let Err(BotError::MusicCommandError(MusicCommandError::SearchTimeout)) =
            queue_pagination_interaction(ctx, queue_vec).await
        {
            return Ok(());
        };
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

async fn queue_pagination_interaction(
    ctx: Context<'_>,
    queued_metadata: Vec<String>,
) -> Result<(), BotError> {
    // define unique identifiers
    let ctx_id = ctx.id();
    let prev_button_id = format!("{}prev", ctx_id);
    let next_button_id = format!("{}next", ctx_id);

    let mut current_page = 0;

    // cut the metadata into chunks
    let queued_metadata_chunks = queued_metadata.chunks(3).collect::<Vec<_>>();

    // create the first reply
    let reply = {
        let mut buttons = vec![serenity::CreateButton::new(&prev_button_id).emoji('◀')];
        let mut reply = poise::CreateReply::default();
        let mut message = serenity::MessageBuilder::default();
        let mut embed = serenity::CreateEmbed::new()
            .author(serenity::CreateEmbedAuthor::new(format!("Queue | Page: {}", current_page)).icon_url(
                "https://cliply.co/wp-content/uploads/2019/04/371903520_SOCIAL_ICONS_YOUTUBE.png",
            ))
            .timestamp(serenity::Timestamp::now())
            .footer(serenity::CreateEmbedFooter::new("Ayaya Discord Bot"));

        for (i, rendered) in queued_metadata_chunks[0].iter().enumerate() {
            message.push_line(rendered);
        }

        // set the description
        embed = embed.description(message.to_string());
        reply = reply.embed(embed.to_owned());
        buttons.push(serenity::CreateButton::new(&next_button_id).emoji('▶'));

        let components = serenity::CreateActionRow::Buttons(buttons);
        reply.components(vec![components])
    };
    ctx.send(reply).await?;

    // Loop through incoming interactions with the navigation buttons
    while let Some(press) = serenity::collector::ComponentInteractionCollector::new(ctx)
        // We defined our button IDs to start with `ctx_id`. If they don't, some other command's
        // button was pressed
        .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
        // Timeout when no navigation button has been pressed for 1 minute
        .timeout(std::time::Duration::from_secs(60))
        .await
    {
        // Depending on which button was pressed, go to next or previous page
        if press.data.custom_id == next_button_id {
            current_page += 1;
            if current_page >= queued_metadata_chunks.len() {
                current_page = 0;
            }
        } else if press.data.custom_id == prev_button_id {
            current_page = current_page
                .checked_sub(1)
                .unwrap_or(queued_metadata_chunks.len() - 1);
        } else {
            // This is an unrelated button interaction
            continue;
        }

        let response = {
            let mut buttons = vec![serenity::CreateButton::new(&prev_button_id).emoji('◀')];
            let mut response = serenity::CreateInteractionResponseMessage::new();
            let mut message = serenity::MessageBuilder::default();
            let mut embed = serenity::CreateEmbed::new()
                .author(serenity::CreateEmbedAuthor::new(format!("Queue | Page: {}", current_page)).icon_url(
                    "https://cliply.co/wp-content/uploads/2019/04/371903520_SOCIAL_ICONS_YOUTUBE.png",
                ))
                .timestamp(serenity::Timestamp::now())
                .footer(serenity::CreateEmbedFooter::new(
                    "Ayaya Discord Bot"
                ));

            for (i, rendered) in queued_metadata_chunks[0].iter().enumerate() {
                message.push_line(rendered);
            }

            // set the description
            embed = embed.description(message.to_string());
            response = response.embed(embed.to_owned());
            buttons.push(serenity::CreateButton::new(&next_button_id).emoji('▶'));

            let components = serenity::CreateActionRow::Buttons(buttons);
            response.components(vec![components])
        };

        // Update the message with the new page contents
        press
            .create_response(
                ctx.serenity_context(),
                serenity::CreateInteractionResponse::UpdateMessage(response),
            )
            .await?;
    }
    // TODO: its own error
    Err(MusicCommandError::SearchTimeout.into())
}
