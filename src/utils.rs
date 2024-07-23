use poise::serenity_prelude as serenity;
use serenity::{model::channel::Message, Result as SerenityResult};
use tracing::error;

use crate::{BotError, Context};

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
