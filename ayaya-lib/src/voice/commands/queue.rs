//! This module contains commands for queue manipulation (excluding addition)

use poise::serenity_prelude as serenity;
use rand::seq::SliceRandom;
use tracing::{error, warn};

use crate::{
    error::BotError,
    utils::{get_guild_id, ChannelInfo, GuildInfo, OptionExt},
    voice::{
        error::MusicCommandError,
        utils::{self, metadata_to_embed, YoutubeMetadata},
    },
    CommandResult, Context,
};

/// Shows the queue. The only kind of acceptable spoilers.
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(
    slash_command,
    prefix_command,
    aliases("q"),
    guild_only,
    category = "Music"
)]
pub async fn queue(ctx: Context<'_>) -> Result<(), BotError> {
    let guild_id = get_guild_id(ctx)?;
    let guild_info = GuildInfo::from_ctx(ctx)?;

    let manager = &ctx.data().songbird;

    // Check if in channel
    if let Some(handler_lock) = manager.get(guild_id) {
        // dont hold the lock, we only need the track metadatas
        let tracks = {
            let handler = handler_lock.lock().await;
            let queue = handler.queue();
            queue.current_queue()
        };
        let queue_vec = if !tracks.is_empty() {
            let mut queue_vec = vec![];

            for (index, track) in tracks.iter().enumerate() {
                let metadata = track.data::<YoutubeMetadata>().as_aux_metadata();
                let rendered = format!(
                    "{}. {} | Channel: {}",
                    index + 1,
                    metadata.title.clone().unwrap_or_unknown(),
                    metadata.channel.clone().unwrap_or_unknown()
                );
                queue_vec.push(rendered);
            }
            queue_vec
        } else {
            ctx.reply("Queue is empty, add some music to see something")
                .await?;
            return Ok(());
        };

        if let Err(BotError::MusicCommandError(MusicCommandError::SearchTimeout)) =
            queue_pagination_interaction(ctx, queue_vec).await
        {
            return Ok(());
        } else {
            warn!("Waited too long");
        };
    } else {
        return Err(MusicCommandError::BotVoiceNotJoined { guild_info }.into());
    }
    //TODO check for

    Ok(())
}

/// Delete song from queue. Being able to make things go POOF makes you feel like a Kami-sama, right?
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    aliases("d"),
    category = "Music"
)]
pub async fn delete(ctx: Context<'_>, queue_position: usize) -> Result<(), BotError> {
    let guild_info = GuildInfo::from_ctx(ctx)?;

    let manager = ctx.data().songbird.clone();

    if let Some(handler_lock) = manager.get(guild_info.guild_id) {
        let voice_channel_info = {
            let handler = handler_lock.lock().await;
            ChannelInfo::from_songbird_current_channel(ctx, handler.current_channel(), &guild_info)
                .await?
        };

        // BUG: something here is holding locks longer than it should
        // If not empty, remove the songs
        if queue_position != 0 {
            let handler = handler_lock.lock().await;
            let queue = handler.queue();
            if queue_position != 1 {
                let index = queue_position - 1;
                if let Some(track) = queue.current_queue().get(index) {
                    let metadata = track.data::<YoutubeMetadata>();
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
#[poise::command(
    slash_command,
    prefix_command,
    aliases("np"),
    guild_only,
    category = "Music"
)]
pub async fn nowplaying(ctx: Context<'_>) -> Result<(), BotError> {
    let guild_info = GuildInfo::from_ctx(ctx)?;

    let manager = &ctx.data().songbird;

    if let Some(handler) = manager.get(guild_info.guild_id) {
        let handler = handler.lock().await;
        match handler.queue().current() {
            Some(track) => {
                let track_uuid = track.uuid();
                let track_state =
                    track
                        .get_info()
                        .await
                        .map_err(|e| MusicCommandError::TrackStateNotFound {
                            source: e,
                            track_uuid,
                        })?;

                let metadata = track.data::<YoutubeMetadata>();

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
                    &YoutubeMetadata::default(),
                    None,
                )))
                .await?;
            }
        };
    } else {
        return Err(MusicCommandError::BotVoiceNotJoined { guild_info }.into());
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command, guild_only, category = "Music")]
pub async fn shuffle(ctx: Context<'_>) -> CommandResult {
    ctx.defer().await?;

    let manager = &ctx.data().songbird;
    let guild_info = GuildInfo::from_ctx(ctx)?;

    if let Some(handler_lock) = manager.get(guild_info.guild_id) {
        {
            let handler = handler_lock.lock().await;
            let queue = { handler.queue() };
            queue.modify_queue(|queued| {
                let mut rng = rand::thread_rng();
                // it is required to preserve the first element
                queued.make_contiguous()[1..].shuffle(&mut rng);
            });
            // TODO: pretty embeds
            ctx.say("Ayaya shuffled the queue!").await?;
        }
    } else {
        return Err(MusicCommandError::BotVoiceNotJoined { guild_info }.into());
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
    let queued_metadata_chunks = queued_metadata.chunks(10).collect::<Vec<_>>();

    // create the first reply
    let reply = {
        let mut buttons = vec![serenity::CreateButton::new(&prev_button_id).emoji('◀')];
        let mut reply = poise::CreateReply::default();
        let mut message = serenity::MessageBuilder::default();
        let mut embed = serenity::CreateEmbed::new()
            .author(serenity::CreateEmbedAuthor::new(format!("Queue | Page: {}", current_page  +1)).icon_url(
                "https://cliply.co/wp-content/uploads/2019/04/371903520_SOCIAL_ICONS_YOUTUBE.png",
            ))
            .timestamp(serenity::Timestamp::now())
            .footer(serenity::CreateEmbedFooter::new("Ayaya Discord Bot"));

        for rendered in queued_metadata_chunks[0].iter() {
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
                .author(serenity::CreateEmbedAuthor::new(format!("Queue | Page: {}", current_page + 1)).icon_url(
                    "https://cliply.co/wp-content/uploads/2019/04/371903520_SOCIAL_ICONS_YOUTUBE.png",
                ))
                .timestamp(serenity::Timestamp::now())
                .footer(serenity::CreateEmbedFooter::new(
                    "Ayaya Discord Bot"
                ));

            for rendered in queued_metadata_chunks[current_page].iter() {
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

/// Swap item positions in queue. Use the queue command to view the queue
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, guild_only, category = "Music")]
pub async fn queue_move(
    ctx: Context<'_>,
    #[description = "The item that is going to be moved. Position 1 is unchangable."]
    original_position: u64,
    #[description = "The target position. Leave to move to next position. Position 1 is unchangeable."]
    target_position: Option<u64>,
) -> CommandResult {
    ctx.defer_or_broadcast().await?;
    let guild_info = GuildInfo::from_ctx(ctx)?;

    let manager = &ctx.data().songbird;

    if let Some(handler) = manager.get(guild_info.guild_id) {
        let handler = handler.lock().await;
        let voice_channel_info =
            ChannelInfo::from_songbird_current_channel(ctx, handler.current_channel(), &guild_info)
                .await?;

        let queue = handler.queue();
        let queue_len = queue.len();
        tracing::warn!("Queue length: {queue_len}");

        // calculate the indexes
        let index = (original_position - 1) as usize;
        let target_position = target_position.unwrap_or(2);
        let target_index = (target_position - 1) as usize;

        // error out if any index is 1, number 1 cannot be changed in any way
        if index == 0 || target_index == 0 {
            return Err(MusicCommandError::QueueMoveNoPos1 {
                guild_info,
                voice_channel_info,
            }
            .into());
        }

        // error out if selection is out of bounds
        if index > queue_len || original_position == 0 {
            return Err(MusicCommandError::QueueOutOfBounds {
                index,
                guild_info,
                voice_channel_info,
            }
            .into());
        }

        // we have checked the index earlier, so this should not panic
        let data = queue
            .modify_queue(|queue_mut| -> Option<std::sync::Arc<YoutubeMetadata>> {
                if let Some(item) = queue_mut.remove(index) {
                    let data = item.data::<YoutubeMetadata>();

                    queue_mut.insert(target_index, item);
                    tracing::info!("moved index {index} to {target_index}");

                    Some(data)
                } else {
                    None
                }
            })
            .expect("index was valid");

        // notify
        let embed = metadata_to_embed(
            utils::EmbedOperation::MoveInQueue {
                source: original_position as usize,
                target: target_position as usize,
            },
            &data,
            None,
        );

        ctx.send(poise::CreateReply::default().embed(embed)).await?;
    } else {
        return Err(MusicCommandError::BotVoiceNotJoined { guild_info }.into());
    }
    Ok(())
}
