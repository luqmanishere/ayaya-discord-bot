use admin::*;
use play_command::*;
use playback_control::*;
use queue::*;
use snafu::ResultExt;
use soundboard::*;

use crate::{
    Commands, Context,
    error::{BotError, GeneralSerenitySnafu},
};

mod admin;
mod play_command;
mod playback_control;
mod queue;
pub(crate) mod soundboard;

pub fn voice_commands() -> Commands {
    vec![
        join(),
        play(),
        leave(),
        queue(),
        nowplaying(),
        search(),
        skip(),
        pause(),
        resume(),
        stop(),
        seek(),
        delete(),
        loop_track(),
        stop_loop(),
        ting(),
        shuffle(),
        shuffle_play(),
        queue_move(),
        play_next(),
        upload_sound(),
        play_sound(),
        rename_sound(),
    ]
}

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
        "delete",
        "loop_track",
        "stop_loop",
        "play_next"
    ),
    aliases("m")
)]
pub async fn music(ctx: Context<'_>) -> Result<(), BotError> {
    let configuration = poise::builtins::HelpConfiguration {
        // [configure aspects about the help message here]
        ..Default::default()
    };
    poise::builtins::help(ctx, Some(&ctx.command().name), configuration)
        .await
        .context(GeneralSerenitySnafu)?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, hide_in_help, ephemeral)]
pub async fn ting(ctx: Context<'_>) -> Result<(), BotError> {
    join::join_inner(ctx, false, true).await?;

    let manager = ctx.data().songbird.clone();
    let guild_id = crate::utils::get_guild_id(ctx)?;
    let call = manager.get(guild_id).expect("exists");
    let input = songbird::input::File::new("ting.wav");

    {
        let mut lock = call.lock().await;
        lock.play(input.into());
    }
    Ok(())
}
