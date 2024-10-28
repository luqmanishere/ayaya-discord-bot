//! Contains commands reserved for the bot's owner: ie me.

use entity::prelude::*;
use sea_orm::{prelude::*, QuerySelect};

use crate::{error::BotError, Commands, Context};

pub fn owner_commands() -> Commands {
    vec![command_log_raw()]
}

/// Prints command logs raw. Owner only.
#[poise::command(
    slash_command,
    prefix_command,
    owners_only,
    ephemeral,
    hide_in_help,
    category = "Owner Commands"
)]
pub async fn command_log_raw(ctx: Context<'_>) -> Result<(), BotError> {
    ctx.defer().await?;

    let db = ctx.data().db.clone();

    let logs = CommandCallLog::find().limit(5).all(&db).await?;

    ctx.reply(format!("{:#?}", logs)).await?;
    Ok(())
}
