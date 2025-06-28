//! This module contains commands that should only be available to select roles or admins

use tracing::error;

use crate::{
    error::BotError,
    utils::{check_msg, get_guild_id, ChannelInfo, GuildInfo},
    voice::error::MusicCommandError,
    Context,
};

/// Undeafens the bot. Finally, Ayaya pulls out her earplugs.
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, guild_only, category = "Music")]
pub async fn undeafen(ctx: Context<'_>) -> Result<(), BotError> {
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
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    aliases("um"),
    category = "Music"
)]
pub async fn unmute(ctx: Context<'_>) -> Result<(), BotError> {
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

/// Mutes Ayaya. Mmmhh mmhh mmmhhh????
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, guild_only, category = "Music")]
pub async fn mute(ctx: Context<'_>) -> Result<(), BotError> {
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
            check_msg(ctx.channel_id().say(ctx, format!("Failed: {e:?}")).await);
        }

        ctx.say("Now muted").await?;
    }

    Ok(())
}

/// Deafens Ayaya. She knows how to read lips, you know.
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, guild_only, category = "Music")]
pub async fn deafen(ctx: Context<'_>) -> Result<(), BotError> {
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
            ctx.say(format!("Failed to deafen: {e:?}")).await?;
        }

        ctx.say("Deafened").await?;
    }

    Ok(())
}
