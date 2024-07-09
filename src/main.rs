use std::env;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use eyre::{Context as EyreContext, Result};
use poise::{serenity_prelude as serenity, FrameworkError};
use reqwest::Client as HttpClient;
use songbird::{input::AuxMetadata, typemap::TypeMapKey, SerenityInit};
use tracing::{error, info, instrument, level_filters::LevelFilter};
use tracing_error::ErrorLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use uuid::Uuid;

use crate::voice::music;

// use crate::utils::check_msg;
// use crate::voice::*;

mod utils;
mod voice;
mod voice_events;

async fn event_handler(
    _ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, eyre::ErrReport>,
    _data: &Data,
) -> Result<()> {
    match event {
        serenity::FullEvent::Ready { data_about_bot, .. } => {
            let bot_user_name = &data_about_bot.user.name;
            let session_id = &data_about_bot.session_id;
            info!(
                "Logged in as {} with session id {}.",
                bot_user_name, session_id
            );
        }
        serenity::FullEvent::CacheReady { guilds } => {
            info!("Cached guild info is ready for {} guilds.", guilds.len());
        }
        _ => {}
    }
    Ok(())
}

// #[hook]
// #[instrument]
// async fn before(_: &Context, msg: &Message, command_name: &str) -> bool {
//     info!(
//         "Got command '{}' by user '{}'",
//         command_name, msg.author.name
//     );

//     true
// }

// #[help]
// async fn my_help(
//     context: &Context,
//     msg: &Message,
//     args: Args,
//     help_options: &'static HelpOptions,
//     groups: &[&'static CommandGroup],
//     owners: HashSet<UserId>,
// ) -> CommandResult {
//     let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
//     Ok(())
// }

// #[group]
// #[commands(ping, about)]
// struct General;

// #[group]
// #[commands(
//     deafen, join, leave, mute, play, search, resume, pause, queue, delete, nowplaying, skip, stop,
//     undeafen, unmute
// )]
// #[summary("Music controls")]
// struct Music;

pub type Context<'a> = poise::Context<'a, Data, eyre::ErrReport>;
// User data, which is stored and accessible in all command invocations
pub struct Data {
    http: HttpClient,
    track_metadata: Arc<Mutex<HashMap<Uuid, AuxMetadata>>>,
}

#[tokio::main]
#[instrument]
async fn main() -> Result<()> {
    // Init tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(ErrorLayer::default())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    info!("log initialized");

    // Configure the client with your Discord bot token in the environment.
    // DISCORD_TOKEN_FILE is searched first, then DISCORD_TOKEN.
    // IF DISCORD_TOKEN_FILE is found, the token is read from the file.
    let token = {
        if let Ok(token_file) = env::var("DISCORD_TOKEN_FILE") {
            std::fs::read_to_string(token_file)?
        } else {
            #[cfg(debug_assertions)]
            dotenv::dotenv().expect("works");
            env::var("DISCORD_TOKEN").wrap_err("Expected a token in the environment")?
        }
    };

    #[cfg(debug_assertions)]
    let prefix = "~";

    #[cfg(not(debug_assertions))]
    let prefix = "aya";

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![about(), help(), ping(), music()],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some(prefix.into()),
                /* non_command_message: Some(|_, _, msg| {
                    Box::pin(async move  {
                        println!("non command message!: {}", msg.content);
                        Ok(())
                    })
                }),*/
                mention_as_prefix: true,
                case_insensitive_commands: true,
                ..Default::default()
            },
            on_error: |error: FrameworkError<'_, Data, eyre::ErrReport>| {
                Box::pin(async move {
                    error!("error error error {}", error);
                    match error {
                        poise::FrameworkError::ArgumentParse { error, .. } => {
                            if let Some(error) = error.downcast_ref::<serenity::RoleParseError>() {
                                error!("Found a RoleParseError: {:?}", error);
                            } else {
                                error!("Not a RoleParseError :(");
                            }
                        }
                        poise::FrameworkError::UnknownCommand {
                            ctx,
                            msg,
                            msg_content,
                            ..
                        } => {
                            error!("unrecognized command: {}", msg_content);
                            msg.reply(ctx, format!("unrecognized command: {}", msg_content))
                                .await
                                .expect("no errors");
                        }
                        poise::FrameworkError::Command { error, ctx, .. } => {
                            let cmd = ctx.command().name.clone();
                            error!("Error in command ({}): {}", cmd, error);
                            // TODO: flesh out user facing error message
                            ctx.channel_id()
                                .say(ctx, "Error running whatever you did")
                                .await
                                .expect("works");
                        }
                        other => {
                            if let Err(e) = poise::builtins::on_error(other).await {
                                error!("Error sending error message: {}", e);
                            }
                        }
                    }
                })
            },
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                info!("Setup...");
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    http: HttpClient::new(),
                    track_metadata: Default::default(),
                })
            })
        })
        .build();

    let intents = serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::MESSAGE_CONTENT
        | serenity::GatewayIntents::GUILD_VOICE_STATES;

    let mut client = serenity::Client::builder(&token, intents)
        .framework(framework)
        .register_songbird()
        .type_map_insert::<HttpKey>(HttpClient::new())
        .await
        .expect("Err creating client");

    client.start().await.wrap_err("client ended")
}

struct HttpKey;

impl TypeMapKey for HttpKey {
    type Value = HttpClient;
}

#[poise::command(prefix_command, slash_command)]
async fn ping(ctx: Context<'_>) -> Result<()> {
    ctx.reply("Pong!").await?;

    Ok(())
}

/// Ayaya likes to talk about herself...
#[poise::command(slash_command, prefix_command)]
async fn about(ctx: Context<'_>) -> Result<()> {
    let about = poise::CreateReply::default()
        .content(
            r"
_*Ayaya*_, a random bot
Author: SolemnAttic#9269
Github: https://github.com/luqmanishere/ayaya-discord-bot

Consider leaving a star on the Github page!
    ",
        )
        .reply(true);

    ctx.send(about).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command)]
pub async fn help(ctx: Context<'_>, command: Option<String>) -> Result<()> {
    let configuration = poise::builtins::HelpConfiguration {
        // [configure aspects about the help message here]
        ..Default::default()
    };
    poise::builtins::help(ctx, command.as_deref(), configuration).await?;
    Ok(())
}
