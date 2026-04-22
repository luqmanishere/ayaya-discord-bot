//! Commands for stats
//!
use std::collections::HashMap;

use poise::serenity_prelude::{self as serenity, Mentionable};
use snafu::ResultExt;

use crate::{
    CommandResult, Commands, Context,
    error::{BotError, DataManagerSnafu, GeneralSerenitySnafu},
    utils::{GuildInfo, autocomplete_command_names, get_guild_name},
};

pub fn stats_commands() -> Commands {
    vec![
        user_all_time_single(),
        server_all_time_single(),
        server_voice_stats(),
    ]
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
    ctx.defer().await.context(GeneralSerenitySnafu)?;

    let data_manager = ctx.data().data_manager.clone();
    let user_id = user.id.get();
    let guild_id = GuildInfo::guild_id_or_0(ctx);

    match data_manager
        .find_single_user_single_all_time_command_stats(guild_id, user_id, &command)
        .await
        .context(DataManagerSnafu)?
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
            ctx.reply(msg).await.context(GeneralSerenitySnafu)?;
        }
        None => {
            ctx.reply(format!(
                "Data for user {}, command name `{}` is not found",
                user.mention(),
                command
            ))
            .await
            .context(GeneralSerenitySnafu)?;
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
    ctx.defer().await.context(GeneralSerenitySnafu)?;
    let http = ctx.http();

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
            let mut lines = Vec::new();
            for (i, e) in model.iter().enumerate() {
                let line = format!(
                    "{}. {}: {}",
                    i + 1,
                    match serenity::UserId::new(e.user_id as u64).to_user(http).await {
                        Ok(res) => res.display_name().to_string(),
                        Err(_) => format!("Unknown({})", e.user_id),
                    },
                    e.count
                );
                lines.push(line);
            }

            let desc = lines.join("\n");

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
            ctx.send(reply).await.context(GeneralSerenitySnafu)?;
        }
        Err(_) => {
            ctx.reply(format!("Data for command name `{command}` is not found"))
                .await
                .context(GeneralSerenitySnafu)?;
        }
    }

    Ok(())
}

/// A measure of no life
#[poise::command(slash_command, prefix_command, category = "Statistics")]
pub async fn server_voice_stats(
    ctx: Context<'_>,
    server_id: Option<serenity::GuildId>,
) -> CommandResult {
    ctx.defer().await.context(GeneralSerenitySnafu)?;

    let data_manager = ctx.data().data_manager.clone();
    let guild_id = if let Some(guild_id) = server_id {
        guild_id.get()
    } else {
        GuildInfo::guild_id_or_0(ctx)
    };

    match data_manager
        .voice()
        .get_server_voice_sessions(guild_id)
        .await
    {
        Ok(models) => {
            // calculate length for each user
            let mut map: std::collections::HashMap<u64, time::Duration> = HashMap::default();

            for model in models {
                if let Some(left_at) = model.left_at {
                    let dur = left_at - model.joined_at;
                    map.entry(model.user_id as u64)
                        .and_modify(|e| {
                            *e = e.saturating_add(dur);
                        })
                        .or_insert(dur);
                } else {
                    tracing::warn!("no left at time found, skipping");
                }
            }

            let mut msg = serenity::MessageBuilder::default()
                .push_line(format!("# Voicechat Stats: {guild_id}").as_str());

            for (i, (user_id, dur)) in map.iter().enumerate() {
                let i = i + 1;
                let user = serenity::UserId::new(*user_id)
                    .to_user(ctx)
                    .await
                    .context(GeneralSerenitySnafu)?;
                let dur = humantime::format_duration(dur.unsigned_abs()).to_string();
                let line = format!("{i}. {}: {}", user.name, dur);
                msg = msg.push_line(line.as_str());
            }

            ctx.reply(msg.to_string())
                .await
                .context(GeneralSerenitySnafu)?;
        }
        Err(_) => {
            ctx.say("No data found")
                .await
                .context(GeneralSerenitySnafu)?;
        }
    }

    Ok(())
}
