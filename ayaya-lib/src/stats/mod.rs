//! Commands for stats
//!
use poise::serenity_prelude::{self as serenity, Mentionable};

use crate::{
    error::BotError,
    utils::{autocomplete_command_names, get_guild_name, GuildInfo},
    CommandResult, Commands, Context,
};

pub fn stats_commands() -> Commands {
    vec![user_all_time_single(), server_all_time_single()]
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
        .find_single_user_single_all_time_command_stats(guild_id, user_id, &command)
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

/// Shows the total amount of specific command invocations for all users in a server.
#[poise::command(
    slash_command,
    prefix_command,
    rename = "sats",
    category = "Statistics"
)]
pub async fn server_all_time_single(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_command_names"] command: String,
) -> CommandResult {
    ctx.defer().await?;

    let data_manager = ctx.data().data_manager.clone();
    let guild_id = GuildInfo::guild_id_or_0(ctx);

    match data_manager
        .find_all_users_single_command_call(guild_id, command.clone())
        .await
    {
        Ok(mut model) => {
            model.sort_by_key(|e| e.count);
            model.reverse();

            // numbering
            let desc = model
                .iter()
                .enumerate()
                .map(|(i, e)| {
                    format!(
                        "{}. {}: {}",
                        i + 1,
                        match ctx.cache().user(e.user_id) {
                            Some(res) => res.display_name().to_string(),
                            None => format!("Unknown({})", e.user_id),
                        },
                        e.count
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");

            let embed = serenity::CreateEmbed::new()
                .title(format!("All users calls for command: {command}"))
                .description(desc)
                .color(serenity::Color::FABLED_PINK);
            // let msg = if guild_id == 0 {
            //     format!(
            //         "All-time invocations for command `{}` in DMs: {}",
            //         command, model.count
            //     )
            // } else {
            //     let guild_name = get_guild_name(ctx)?;
            //     format!(
            //         "All-time invocations for command `{}` in server {}: {}",
            //         command, guild_name, model.count
            //     )
            // };
            let reply = poise::CreateReply::default().reply(true).embed(embed);
            ctx.send(reply).await?;
        }
        Err(_) => {
            ctx.reply(format!("Data for command name `{}` is not found", command))
                .await?;
        }
    }

    Ok(())
}
