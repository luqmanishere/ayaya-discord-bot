//! Module for trolling and meme commands
use poise::serenity_prelude::{self as serenity, Mentionable};
use rand::seq::SliceRandom;
use snafu::ResultExt as _;

use crate::{
    Context,
    error::{BotError, GeneralSerenitySnafu},
};

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
    ctx.reply(random_gay(user))
        .await
        .context(GeneralSerenitySnafu)?;
    Ok(())
}

fn random_gay(user: serenity::User) -> String {
    const GAY: [(&str, u32); 4] = [
        ("<user> is gay", 33),
        ("<@508510863108472858> raeesgay", 1),
        ("<user> gay balls", 33),
        ("why are u gay <user>", 33),
    ];

    let mut rng = rand::thread_rng();
    match GAY.choose_weighted(&mut rng, |item| item.1) {
        Ok((choosen, _)) => choosen
            .replace("<user>", user.mention().to_string().as_str())
            .to_string(),
        Err(_e) => GAY[0].0.to_string(),
    }
}
