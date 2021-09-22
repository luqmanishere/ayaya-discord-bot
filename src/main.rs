use std::{collections::HashSet, env};

use serenity::{
    async_trait,
    client::{Client, Context, EventHandler},
    framework::{
        standard::{
            help_commands,
            macros::{command, group, help, hook},
            Args, CommandGroup, CommandResult, HelpOptions, WithWhiteSpace,
        },
        StandardFramework,
    },
    model::{channel::Message, gateway::Ready, id::UserId},
    utils::MessageBuilder,
};

use songbird::SerenityInit;

use tracing::{error, info, instrument};

use crate::utils::check_msg;
use crate::voice::*;

mod utils;
mod voice;
mod voice_events;

struct Handler;
#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
    }
}

#[hook]
#[instrument]
async fn before(_: &Context, msg: &Message, command_name: &str) -> bool {
    info!(
        "Got command '{}' by user '{}'",
        command_name, msg.author.name
    );

    true
}

#[help]
async fn my_help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}

#[group]
#[commands(ping, about)]
struct General;

#[group]
#[commands(
    deafen, join, leave, mute, play, resume, pause, queue, delete, nowplaying, skip, stop,
    undeafen, unmute
)]
#[summary("Music controls")]
struct Music;

#[tokio::main]
#[instrument]
async fn main() {
    // Init tracing
    tracing_subscriber::fmt::init();

    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    #[cfg(debug_assertions)]
    let prefix = "~";

    #[cfg(not(debug_assertions))]
    let prefix = "aya";

    let framework = StandardFramework::new()
        .configure(|c| {
            c.prefix(prefix)
                .delimiter("$")
                .with_whitespace(WithWhiteSpace {
                    prefixes: true,
                    groups: true,
                    commands: true,
                })
        })
        .before(before)
        .help(&MY_HELP)
        .group(&GENERAL_GROUP)
        .group(&MUSIC_GROUP);

    let mut client = Client::builder(&token)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird()
        .await
        .expect("Err creating client");

    let _ = client
        .start()
        .await
        .map_err(|why| error!("Client ended: {:?}", why));
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    check_msg(msg.channel_id.say(&ctx.http, "Pong!").await);

    Ok(())
}

#[command]
#[description("Ayaya likes to talk about herself...")]
async fn about(ctx: &Context, msg: &Message) -> CommandResult {
    let about = MessageBuilder::new()
        .push_bold_line("Ayaya")
        .push_line("Author: SolemnAttic#9269")
        .push_line("Github: https://github.com/luqmanishere/ayaya-discord-bot")
        .push_line("\nConsider leaving a star on the Github page!")
        .build();

    check_msg(msg.channel_id.say(&ctx.http, about).await);
    Ok(())
}
