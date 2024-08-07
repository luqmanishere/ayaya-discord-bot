use std::{
    collections::HashMap,
    sync::{atomic::AtomicUsize, Arc},
    time::Duration,
};

use ::serenity::futures::TryFutureExt;
use futures::{stream, StreamExt};
use poise::serenity_prelude as serenity;
use serenity::{model::id::ChannelId, prelude::*, Mentionable};
use songbird::{
    input::{Compose, YoutubeDl},
    Event,
};
use tracing::{debug, error, info, log::warn};

// Imports within the crate
use super::{
    events::*,
    utils::{
        self, create_search_interaction, error_embed, metadata_to_embed, resolve_yt_playlist,
        yt_search,
    },
};
use crate::{
    error::command_error_embed,
    utils::{check_msg, get_guild, get_guild_id, ChannelInfo, GuildInfo, OptionExt},
    voice::error::MusicCommandError,
    BotError, Context,
};

/// This command must be called with a subcommmand.
#[poise::command(
    slash_command,
    prefix_command,
    subcommands(
        "join",
        "play",
        "leave",
        "mute",
        "queue",
        "nowplaying",
        "unmute",
        "search",
        "skip",
        "pause",
        "resume",
        "stop",
        "undeafen",
        "seek",
        "deafen",
        "delete"
    ),
    aliases("m")
)]
pub async fn music(ctx: Context<'_>) -> Result<(), BotError> {
    let configuration = poise::builtins::HelpConfiguration {
        // [configure aspects about the help message here]
        ..Default::default()
    };
    poise::builtins::help(ctx, Some(&ctx.command().name), configuration).await?;
    Ok(())
}

// TODO: reply to slash commands properly

/// Deafens Ayaya. She knows how to read lips, you know.
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, guild_only)]
async fn deafen(ctx: Context<'_>) -> Result<(), BotError> {
    let guild_id = get_guild_id(ctx)?;

    let manager = &ctx.data().songbird;

    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            ctx.reply("Not in a voice channel").await?;

            return Ok(());
        }
    };

    let mut handler = handler_lock.lock().await;

    if handler.is_deaf() {
        ctx.reply("Already deafened.").await?;
    } else {
        if let Err(e) = handler.deafen(true).await {
            error!("Failed to deafen: {e}");
            ctx.say(format!("Failed to deafen: {:?}", e)).await?;
        }

        ctx.say("Deafened").await?;
    }

    Ok(())
}

/// Joins the voice channel the user is currently in. PARTY TIME!
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, guild_only, aliases("j"))]
async fn join(ctx: Context<'_>) -> Result<(), BotError> {
    join_inner(ctx, true).await
}

#[tracing::instrument(skip(ctx))]
async fn join_inner(ctx: Context<'_>, play_notify_flag: bool) -> Result<(), BotError> {
    let guild: serenity::Guild = get_guild(ctx)?;
    let guild_id = get_guild_id(ctx)?;
    let guild_info = GuildInfo::from_ctx(ctx)?;
    let chat_channel_id = ctx.channel_id();
    let user_voice_state_option: Option<&serenity::VoiceState> =
        guild.voice_states.get(&ctx.author().id);

    let manager = ctx.data().songbird.clone();

    // check if we are already in a call
    match manager.get(guild_id) {
        // if already in call
        Some(call) => {
            let (voice_channel_name, voice_channel_id) = {
                let call = call.lock().await;
                let chan: u64 = call.current_channel().expect("bruh").0.into();
                let channel_id = serenity::ChannelId::from(chan);
                (channel_id.name(ctx).await?, channel_id)
            };

            warn!("Already in a channel {}, not joining", voice_channel_name);

            if play_notify_flag {
                // TODO: replace with embed
                ctx.reply(format!(
                    "Already in voice channel \"{}\"",
                    voice_channel_id.mention()
                ))
                .await
                .map_err(BotError::GeneralSerenityError)?;
            }
        }
        None => {
            let user_voice_state =
                user_voice_state_option.ok_or(MusicCommandError::UserVoiceNotJoined {
                    user: ctx.author().clone(),
                    guild_info: guild_info.clone(),
                })?;

            // the voice channel id to join
            let voice_channel_id = if let Some(voice_state_guild_id) = user_voice_state.guild_id {
                // check if data is consistent
                if voice_state_guild_id == guild_id {
                    user_voice_state
                        .channel_id
                        .ok_or(MusicCommandError::UserVoiceNotJoined {
                            user: ctx.author().clone(),
                            guild_info: guild_info.clone(),
                        })?
                } else {
                    return Err(BotError::GuildMismatch);
                }
            } else {
                warn!("Not in a guild, expected guild id {}", get_guild_id(ctx)?);
                // TODO: replace with embed
                // TODO: centrailize
                ctx.reply("Cache error. Please rejoin the channel")
                    .await
                    .map_err(BotError::GeneralSerenityError)?;

                return Err(BotError::GuildCacheStale);
            };

            // join the given voice channel
            match manager.join(guild_id, voice_channel_id).await {
                Ok(call) => {
                    let mut call = call.lock().await;
                    let voice_channel_info = ChannelInfo::from_songbird_current_channel(
                        ctx,
                        call.current_channel(),
                        &guild_info,
                    )
                    .await?;
                    info!("joined channel id: {voice_channel_id} in guild {guild_id}",);
                    if play_notify_flag {
                        // TODO: replace with embed
                        ctx.reply(format!("Joined {}", voice_channel_id.mention()))
                            .await?;
                    }

                    // bot should be unmuted and deafened
                    call.mute(false)
                        .map_err(|e| MusicCommandError::FailedUnmuteCall {
                            source: e,
                            guild_info: guild_info.clone(),
                            voice_channel_info: voice_channel_info.clone(),
                        })
                        .await?;
                    call.deafen(true)
                        .map_err(|e| MusicCommandError::FailedDeafenCall {
                            source: e,
                            guild_info,
                            voice_channel_info,
                        })
                        .await?;

                    // TODO: Add event to detect inactivity
                    let bot_user_id = { *ctx.data().user_id.read().await };

                    // inactive counter bot
                    call.add_global_event(
                        Event::Periodic(Duration::from_secs(60), None),
                        BotInactiveCounter {
                            channel_id: chat_channel_id,
                            counter: Arc::new(AtomicUsize::new(0)),
                            guild_id,
                            bot_user_id,
                            manager: ctx.data().songbird.clone(),
                            ctx: ctx.serenity_context().to_owned(),
                        },
                    );
                }
                Err(e) => {
                    let voice_channel_info =
                        ChannelInfo::from_serenity_id(ctx, voice_channel_id, true).await?;
                    error!("Error joining channel: {}", e);
                    // TODO: centralize
                    ctx.say("Unable to join voice channel").await?;
                    return Err(MusicCommandError::FailedJoinCall {
                        source: e,
                        guild_info,
                        voice_channel_info,
                    }
                    .into());
                }
            }
        }
    }

    Ok(())
}

/// Leaves the current voice channel. Ever wonder what happens to Ayaya then?
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, guild_only)]
async fn leave(ctx: Context<'_>) -> Result<(), BotError> {
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

/// Mutes Ayaya. Mmmhh mmhh mmmhhh????
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, guild_only)]
async fn mute(ctx: Context<'_>) -> Result<(), BotError> {
    let guild_id = get_guild_id(ctx)?;

    let manager = &ctx.data().songbird;

    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            ctx.reply("Not in a voice channel").await?;

            return Ok(());
        }
    };

    let mut handler = handler_lock.lock().await;

    if handler.is_mute() {
        check_msg(ctx.channel_id().say(ctx, "Already muted").await);
    } else {
        if let Err(e) = handler.mute(true).await {
            check_msg(ctx.channel_id().say(ctx, format!("Failed: {:?}", e)).await);
        }

        ctx.say("Now muted").await?;
    }

    Ok(())
}

/// Plays music from YT url or search term. We are getting help from a higher being...
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, aliases("p"), guild_only)]
async fn play(
    ctx: Context<'_>,
    #[description = "A url or a search term for youtube"]
    #[min_length = 1]
    url: Vec<String>,
) -> Result<(), BotError> {
    // convert vec to a string
    let url = url.join(" ").trim().to_string();

    ctx.defer().await?;

    play_inner(ctx, url).await?;
    Ok(())
}

async fn play_inner(ctx: Context<'_>, input: String) -> Result<(), BotError> {
    let input_type = PlayParse::parse(&input);

    // join a channel first
    join_inner(ctx, false).await?;

    // TODO: check if youtube url

    input_type.run(ctx).await
}

/// Search YT and get metadata
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command)]
// #[usage("<search term>")]
// #[example("ayaya intensifies")]
async fn search(ctx: Context<'_>, search_term: Vec<String>) -> Result<(), BotError> {
    let term = search_term.join(" ");

    // reply or say in channel depending on command type
    match ctx {
        poise::Context::Application(ctx) => {
            ctx.reply(format!("Searching youtube for: {term}")).await?;
        }
        poise::Context::Prefix(ctx) => {
            ctx.channel_id()
                .say(ctx, format!("Searching youtube for: {term}"))
                .await?;
        }
    }
    ctx.defer().await?;

    // let songbird do the searching
    let search = yt_search(&term, Some(10)).await?;

    // TODO: return errors here
    match create_search_interaction(ctx, search).await {
        Ok(youtube_id) => {
            play_inner(ctx, youtube_id).await?;
        }
        Err(e) => {
            if let BotError::MusicCommandError(MusicCommandError::SearchTimeout) = e {
                return Ok(());
            }
            error!("Error from interaction: {e}");
            return Err(e);
        }
    };

    Ok(())
}

/// Skips the currently playing song. Ayaya wonders why you abandoned your summon so easily.
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, guild_only)]
async fn skip(ctx: Context<'_>) -> Result<(), BotError> {
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

/// Shows the queue. The only kind of acceptable spoilers.
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, aliases("q"), guild_only)]
async fn queue(ctx: Context<'_>) -> Result<(), BotError> {
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

/// Pause the party. Time is frozen in this bubble universe."
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, guild_only)]
async fn pause(ctx: Context<'_>, _args: String) -> Result<(), BotError> {
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

        check_msg(
            ctx.channel_id()
                .say(ctx, format!("{} - paused", song_name))
                .await,
        );
    } else {
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
async fn resume(ctx: Context<'_>) -> Result<(), BotError> {
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
async fn stop(ctx: Context<'_>) -> Result<(), BotError> {
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

/// Delete song from queue. Being able to make things go *poof* makes you feel like a Kami-sama, right?
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, guild_only, aliases("d"))]
async fn delete(ctx: Context<'_>, queue_position: usize) -> Result<(), BotError> {
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

/// Undeafens the bot. Finally, Ayaya pulls out her earplugs.
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, guild_only)]
async fn undeafen(ctx: Context<'_>) -> Result<(), BotError> {
    let guild_info = GuildInfo::from_ctx(ctx)?;

    let manager = &ctx.data().songbird;

    if let Some(handler_lock) = manager.get(guild_info.guild_id) {
        let mut handler = handler_lock.lock().await;
        let voice_channel_info =
            ChannelInfo::from_songbird_current_channel(ctx, handler.current_channel(), &guild_info)
                .await?;
        handler
            .deafen(false)
            .await
            .map_err(|e| MusicCommandError::FailedUndeafenCall {
                source: e,
                guild_info,
                voice_channel_info,
            })?;

        ctx.reply("Undeafened").await?;
    } else {
        return Err(MusicCommandError::BotVoiceNotJoined { guild_info }.into());
    }

    Ok(())
}

/// Unmutes Ayaya. Poor Ayaya has been talking to herself unnoticed.
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, guild_only, aliases("um"))]
async fn unmute(ctx: Context<'_>) -> Result<(), BotError> {
    let guild_info = GuildInfo::from_ctx(ctx)?;

    let manager = &ctx.data().songbird;

    if let Some(handler_lock) = manager.get(guild_info.guild_id) {
        let mut handler = handler_lock.lock().await;
        let voice_channel_info =
            ChannelInfo::from_songbird_current_channel(ctx, handler.current_channel(), &guild_info)
                .await?;
        handler
            .mute(false)
            .await
            .map_err(|e| MusicCommandError::FailedUnmuteCall {
                source: e,
                guild_info,
                voice_channel_info,
            })?;
        // TODO: embed
        ctx.reply("Unmuted").await?;
    } else {
        return Err(MusicCommandError::BotVoiceNotJoined { guild_info }.into());
    }

    Ok(())
}

/// "Shows what song is currently playing. Ayaya really knows everything about herself."
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, aliases("np"), guild_only)]
async fn nowplaying(ctx: Context<'_>) -> Result<(), BotError> {
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

/// Seeks the track to a position given in seconds
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, guild_only)]
async fn seek(ctx: Context<'_>, secs: u64) -> Result<(), BotError> {
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

pub enum PlayParse {
    Search(String),
    Url(String),
    PlaylistUrl(String),
}

impl PlayParse {
    pub fn parse(input: &str) -> Self {
        if input.starts_with("http") {
            if input.contains("playlist") {
                return Self::PlaylistUrl(input.to_string());
            }

            Self::Url(input.to_string())
        } else {
            Self::Search(input.to_string())
        }
    }

    /// Handle the parsed input for play. Takes the poise context to facilitate communication
    pub async fn run(self, ctx: Context<'_>) -> Result<(), BotError> {
        let manager = ctx.data().songbird.clone();
        let guild_id = get_guild_id(ctx)?;
        let calling_channel_id = ctx.channel_id();
        let call = manager.get(guild_id);
        match self {
            PlayParse::Search(search) => {
                info!("searching youtube for: {}", search);
                let source = YoutubeDl::new_search(ctx.data().http.clone(), search);
                handle_single_play(call, calling_channel_id, source, ctx).await?;
            }
            PlayParse::Url(url) => {
                info!("using provided link: {}", url);
                let source = YoutubeDl::new(ctx.data().http.clone(), url);
                handle_single_play(call, calling_channel_id, source, ctx).await?;
            }
            PlayParse::PlaylistUrl(playlist_url) => {
                info!("using provided playlist link: {playlist_url}");
                ctx.reply("Handling playlist....").await?;

                let metadata_vec = resolve_yt_playlist(playlist_url).await?;

                let channel_id = ctx.channel_id();
                let call = manager.get(guild_id);

                // TODO: make it ordered
                let fut = stream::iter(metadata_vec).for_each_concurrent(20, |metadata| async {
                    if let Err(error) = handle_from_playlist(
                        metadata,
                        ctx.data().http.clone(),
                        ctx.data().track_metadata.clone(),
                        call.clone(),
                        ctx.serenity_context().http.clone(),
                        channel_id,
                    )
                    .await
                    {
                        let cmd = ctx.command().name.clone();
                        error!("Error executing command ({}): {}", cmd, error);

                        if let Err(e) = ctx
                            .send(
                                poise::CreateReply::default()
                                    .embed(command_error_embed(cmd, error)),
                            )
                            .await
                        {
                            error!("Error sending error message: {}", e);
                        }
                    };
                });

                fut.await;
            }
        }
        Ok(())
    }
}

/// Inserts a youtube source, sets events and notifies the calling channel
#[tracing::instrument(skip(ctx, call, source))]
async fn handle_single_play(
    call: Option<Arc<Mutex<songbird::Call>>>,
    calling_channel_id: ChannelId,
    source: YoutubeDl,
    ctx: Context<'_>,
) -> Result<(), BotError> {
    let metadata = insert_source(
        source,
        ctx.data().track_metadata.clone(),
        call,
        ctx.serenity_context().http.clone(),
        calling_channel_id,
    )
    .await?;

    let embed = metadata_to_embed(utils::EmbedOperation::AddToQueue, &metadata, None);
    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

// TODO: decide if this needs to be instrumented
async fn handle_from_playlist(
    metadata: utils::YoutubeMetadata,
    http: reqwest::Client,
    track_metadata: Arc<std::sync::Mutex<HashMap<uuid::Uuid, songbird::input::AuxMetadata>>>,
    call: Option<Arc<Mutex<songbird::Call>>>,
    serenity_http: Arc<serenity::Http>,
    calling_channel_id: ChannelId,
) -> Result<songbird::input::AuxMetadata, BotError> {
    // our ids are formatted into youtube links to prevent command line errors
    let youtube_link = format!("https://www.youtube.com/watch?v={}", metadata.youtube_id);

    // TODO: polis
    debug!("Handling youtube link {youtube_link}");
    let source = YoutubeDl::new(http.clone(), youtube_link);
    insert_source(
        source,
        track_metadata,
        call,
        serenity_http,
        calling_channel_id,
    )
    .await
}

/// Process the given source, obtain its metadata and handle track insertion with events. This
/// function is made to be used with tokio::spawn
#[tracing::instrument(skip(track_metadata, call, serenity_http, calling_channel_id, source))]
async fn insert_source(
    mut source: YoutubeDl,
    track_metadata: Arc<std::sync::Mutex<HashMap<uuid::Uuid, songbird::input::AuxMetadata>>>,
    call: Option<Arc<Mutex<songbird::Call>>>,
    serenity_http: Arc<serenity::Http>,
    calling_channel_id: ChannelId,
) -> Result<songbird::input::AuxMetadata, BotError> {
    // TODO: rework this entire thing
    info!("Gathering metadata for source");
    match source.aux_metadata().await {
        Ok(metadata) => {
            let track: songbird::tracks::Track = source.into();
            let track_uuid = track.uuid;

            {
                let mut metadata_lock = track_metadata.lock().unwrap();
                metadata_lock.insert(track_uuid, metadata.clone());
            }

            // queue the next song few seconds before current song ends
            let preload_time = if let Some(duration) = metadata.duration {
                duration.checked_sub(std::time::Duration::from_secs(8))
            } else {
                None
            };

            if let Some(handler_lock) = &call {
                let mut handler = handler_lock.lock().await;
                let track_handle = handler.enqueue_with_preload(track, preload_time);

                let serenity_http_clone = serenity_http.clone();
                track_handle
                    .add_event(
                        Event::Track(songbird::TrackEvent::Play),
                        TrackPlayNotifier {
                            channel_id: calling_channel_id,
                            metadata: metadata.clone(),
                            http: serenity_http_clone,
                        },
                    )
                    .unwrap();
                info!(
                    "Added track {} ({}) to channel {calling_channel_id}",
                    metadata.title.clone().unwrap_or_unknown(),
                    metadata.channel.clone().unwrap_or_unknown()
                );
                // Logging added playlist in discord will be annoyying af
                Ok(metadata)
            } else {
                error!("Call does not exist...");
                return Err(MusicCommandError::CallDoesNotExist.into());
            }
        }
        Err(e) => {
            let err = format!("Unable to get metadata from youtube {e}");
            error!(err);
            return Err(MusicCommandError::TrackMetadataRetrieveFailed(e).into());
        }
    }
}
