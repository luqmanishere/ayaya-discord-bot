use admin::*;
use play_command::*;
use playback_control::*;
use queue::*;

use crate::{error::BotError, Context};

mod admin;
mod play_command;
mod playback_control;
mod queue;

/// This command must be called with a subcommmand.
#[poise::command(
    slash_command,
    prefix_command,
    subcommands(
        "join",
        "play",
        "leave",
        "mute",
        "queue",
        "nowplaying",
        "unmute",
        "search",
        "skip",
        "pause",
        "resume",
        "stop",
        "undeafen",
        "seek",
        "deafen",
        "delete"
    ),
    aliases("m")
)]
pub async fn music(ctx: Context<'_>) -> Result<(), BotError> {
    let configuration = poise::builtins::HelpConfiguration {
        // [configure aspects about the help message here]
        ..Default::default()
    };
    poise::builtins::help(ctx, Some(&ctx.command().name), configuration).await?;
    Ok(())
}
