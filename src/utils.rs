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
