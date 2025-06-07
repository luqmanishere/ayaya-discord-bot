//! This module contains functions supporting the join command
use std::{
    sync::{atomic::AtomicUsize, Arc},
    time::Duration,
};

use ::serenity::futures::TryFutureExt as _;
use poise::serenity_prelude as serenity;
use serenity::Mentionable;
use songbird::Event;
use tracing::{error, info, warn};

use crate::{
    error::BotError,
    utils::{get_guild, get_guild_id, ChannelInfo, GuildInfo},
    voice::{error::MusicCommandError, events::BotInactiveCounter},
    Context,
};

#[tracing::instrument(skip(ctx))]
pub async fn join_inner(
    ctx: Context<'_>,
    play_notify_flag: bool,
    linger: bool,
) -> Result<(), BotError> {
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
                            only_alone: linger,
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
