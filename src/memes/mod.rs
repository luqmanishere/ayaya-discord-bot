//! Module for trolling and meme commands
use poise::serenity_prelude::{self as serenity, Mentionable};
use rand::seq::SliceRandom;

use crate::{error::BotError, Context};

/// Unleash the gay memes on a user.
///
/// U are gay.
#[poise::command(guild_only, slash_command, prefix_command, category = "Memes")]
pub async fn gay(
    ctx: Context<'_>,
    #[description = "Selected user"] user: Option<serenity::User>,
) -> Result<(), BotError> {
    let user = if let Some(user) = user {
        user
    } else {
        ctx.author().clone()
    };
    ctx.reply(random_gay(user)).await?;
    Ok(())
}

fn random_gay(user: serenity::User) -> String {
    const GAY: [&str; 4] = [
        "<user> is gay",
        "<@508510863108472858> raeesgay",
        "<user> gay balls",
        "why are u gay <user>",
    ];

    let mut rng = rand::thread_rng();
    match GAY.choose(&mut rng) {
        Some(choosen) => choosen
            .replace("<user>", user.mention().to_string().as_str())
            .to_string(),
        None => GAY[0].to_string(),
    }
}
