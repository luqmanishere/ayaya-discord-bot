//! Commands for stats
//!
use entity::{prelude::*, user_command_all_time_statistics};
use poise::serenity_prelude::{self as serenity, Mentionable};
use sea_orm::{
    prelude::*,
    strum::{Display, VariantNames},
};

use crate::{
    error::BotError,
    utils::{get_guild_name, GuildInfo},
    Commands, Context,
};

pub fn stats_commands() -> Commands {
    vec![user_all_time_single()]
}

/// Shows the total amount of specific command invocations for a user.
#[poise::command(slash_command, prefix_command, category = "Statistics")]
pub async fn user_all_time_single(
    ctx: Context<'_>,
    user: serenity::User,
    #[autocomplete = "autocomplete_command_names"] command: String,
) -> Result<(), BotError> {
    ctx.defer().await?;

    let db = ctx.data().db.clone();
    let user_id = user.id.get();
    let guild_id = GuildInfo::guild_id_or_0(ctx);

    match UserCommandAllTimeStatistics::find()
        .filter(user_command_all_time_statistics::Column::ServerId.eq(guild_id))
        .filter(user_command_all_time_statistics::Column::UserId.eq(user_id))
        .filter(user_command_all_time_statistics::Column::Command.eq(command.clone()))
        .one(&db)
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

async fn autocomplete_command_names<'a>(_ctx: Context<'_>, partial: &'a str) -> Vec<String> {
    let partial = partial.to_lowercase();
    CommandList::VARIANTS
        .iter()
        .filter(|s| s.starts_with(&partial))
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
}

#[derive(Display, EnumIter, VariantNames)]
#[non_exhaustive]
#[strum(serialize_all = "snake_case")]
enum CommandList {
    Play,
    Join,
    Skip,
    Delete,
    Queue,
    Search,
    Help,
    UserAllTimeSingle,
    UserAllTime,
}
