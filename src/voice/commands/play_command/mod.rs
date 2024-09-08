//! This module contains the playback insert commands

use join::*;
use play::*;
use tracing::error;

use crate::{
    error::BotError,
    utils::get_guild_id,
    voice::{
        error::MusicCommandError,
        utils::{create_search_interaction, yt_search},
    },
    Context,
};

mod join;
mod play;
mod youtube;

/// Joins the voice channel the user is currently in. PARTY TIME!
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, guild_only, aliases("j"))]
pub async fn join(ctx: Context<'_>) -> Result<(), BotError> {
    join_inner(ctx, true).await
}

/// Plays music from YT url or search term. We are getting help from a higher being...
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command, aliases("p"), guild_only)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "A url or a search term for youtube"]
    #[min_length = 1]
    url: Vec<String>,
) -> Result<(), BotError> {
    // convert vec to a string
    let url = url.join(" ").trim().to_string();

    ctx.defer().await?;

    play_inner(ctx, url).await?;
    Ok(())
}

/// Search YT and get metadata
#[tracing::instrument(skip(ctx), fields(user_id = %ctx.author().id, guild_id = get_guild_id(ctx)?.get()))]
#[poise::command(slash_command, prefix_command)]
// #[usage("<search term>")]
// #[example("ayaya intensifies")]
pub async fn search(ctx: Context<'_>, search_term: Vec<String>) -> Result<(), BotError> {
    let term = search_term.join(" ");

    // reply or say in channel depending on command type
    match ctx {
        poise::Context::Application(ctx) => {
            ctx.reply(format!("Searching youtube for: {term}")).await?;
        }
        poise::Context::Prefix(ctx) => {
            ctx.channel_id()
                .say(ctx, format!("Searching youtube for: {term}"))
                .await?;
        }
    }
    ctx.defer().await?;

    // let songbird do the searching
    let search = yt_search(&term, Some(10)).await?;

    // TODO: return errors here
    match create_search_interaction(ctx, search).await {
        Ok(youtube_id) => {
            play_inner(ctx, youtube_id).await?;
        }
        Err(e) => {
            if let BotError::MusicCommandError(MusicCommandError::SearchTimeout) = e {
                return Ok(());
            }
            error!("Error from interaction: {e}");
            return Err(e);
        }
    };

    Ok(())
}
