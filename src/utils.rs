use poise::serenity_prelude as serenity;
use serenity::{model::channel::Message, Result as SerenityResult};
use tracing::error;

use crate::{voice::error::MusicCommandError, BotError, Context};

/// Checks that a message successfully sent; if not, then logs why to stdout.
pub fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        error!("Error sending message: {:?}", why);
    }
}

pub trait OptionExt<String> {
    fn unwrap_or_unknown(self) -> String;
}

impl OptionExt<String> for Option<String> {
    fn unwrap_or_unknown(self) -> String {
        self.unwrap_or_else(|| "Unknown".to_string())
    }
}

pub fn get_guild_id(ctx: Context<'_>) -> Result<serenity::GuildId, BotError> {
    ctx.guild_id().ok_or(BotError::NoGuildId)
}

pub fn get_guild(ctx: Context<'_>) -> Result<serenity::Guild, BotError> {
    Ok(ctx.guild().ok_or(BotError::NoGuild)?.clone())
}

pub fn get_guild_name(ctx: Context<'_>) -> Result<String, BotError> {
    Ok(get_guild_id(ctx)?
        .name(ctx)
        .unwrap_or("Unknown Guild".to_string()))
}

pub async fn get_channel_name_id(
    ctx: Context<'_>,
    channel_id: serenity::ChannelId,
) -> Result<String, BotError> {
    Ok(channel_id
        .name(ctx)
        .await
        .unwrap_or("Unknown Channel".to_string()))
}

pub fn songbird_channel_to_serenity_channel(
    songbird_voice_channel_id: songbird::id::ChannelId,
) -> serenity::ChannelId {
    let channel_id: u64 = songbird_voice_channel_id.0.into();
    serenity::ChannelId::from(channel_id)
}

#[derive(Debug, Clone)]
pub struct GuildInfo {
    pub guild_name: String,
    pub guild_id: serenity::GuildId,
}

impl GuildInfo {
    pub fn from_ctx(ctx: Context<'_>) -> Result<Self, BotError> {
        let guild_id = get_guild_id(ctx)?;
        let guild_name = get_guild_name(ctx)?;
        Ok(Self {
            guild_name,
            guild_id,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ChannelInfo {
    pub channel_name: String,
    pub channel_id: serenity::ChannelId,
    pub is_voice: bool,
}

impl ChannelInfo {
    pub async fn from_ctx(ctx: Context<'_>, is_voice: bool) -> Result<Self, BotError> {
        let channel_id = ctx.channel_id();
        let channel_name = channel_id.name(ctx).await?;
        Ok(Self {
            channel_name,
            channel_id,
            is_voice,
        })
    }

    pub async fn from_songbird_current_channel(
        ctx: Context<'_>,
        songbird_voice_channel: Option<songbird::id::ChannelId>,
        guild_info: &GuildInfo,
    ) -> Result<Self, BotError> {
        let channel_id = songbird_channel_to_serenity_channel(songbird_voice_channel.ok_or(
            MusicCommandError::BotVoiceNotJoined {
                guild_info: guild_info.clone(),
            },
        )?);
        let channel_name = channel_id.name(ctx).await?;
        Ok(Self {
            channel_name,
            channel_id,
            is_voice: true,
        })
    }

    pub async fn from_serenity_id(
        ctx: Context<'_>,
        channel_id: serenity::ChannelId,
        is_voice: bool,
    ) -> Result<Self, BotError> {
        let channel_name = channel_id.name(ctx).await?;
        Ok(Self {
            channel_name,
            channel_id,
            is_voice,
        })
    }
}
