//! Commands for stats
//!
use poise::serenity_prelude::{self as serenity, Mentionable};

use crate::{
    error::BotError,
    utils::{autocomplete_command_names, get_guild_name, GuildInfo},
    Commands, Context,
};

pub fn stats_commands() -> Commands {
    vec![user_all_time_single()]
}

/// Shows the total amount of specific command invocations for a user.
#[poise::command(
    slash_command,
    prefix_command,
    rename = "uats",
    category = "Statistics"
)]
pub async fn user_all_time_single(
    ctx: Context<'_>,
    user: serenity::User,
    #[autocomplete = "autocomplete_command_names"] command: String,
) -> Result<(), BotError> {
    ctx.defer().await?;

    let data_manager = ctx.data().data_manager.clone();
    let user_id = user.id.get();
    let guild_id = GuildInfo::guild_id_or_0(ctx);

    match data_manager
        .find_user_all_time_command_stats(guild_id, user_id, &command)
        .await?
    {
        Some(model) => {
            let msg = if guild_id == 0 {
                format!(
                    "All-time invocations for command `{}` in DMs: {}",
                    command, model.count
                )
            } else {
                let guild_name = get_guild_name(ctx)?;
                format!(
                    "All-time invocations for command `{}` in server {}: {}",
                    command, guild_name, model.count
                )
            };
            ctx.reply(msg).await?;
        }
        None => {
            ctx.reply(format!(
                "Data for user {}, command name `{}` is not found",
                user.mention(),
                command
            ))
            .await?;
        }
    }

    Ok(())
}
