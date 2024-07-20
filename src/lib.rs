use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use anyhow::{Context as _, Result};
use poise::{serenity_prelude as serenity, FrameworkError};
use reqwest::Client as HttpClient;
use songbird::input::AuxMetadata;
use time::UtcOffset;
use tracing::{error, info, level_filters::LevelFilter, subscriber::set_global_default};
use tracing_error::ErrorLayer;
use tracing_subscriber::{fmt::time::OffsetTime, layer::SubscriberExt, EnvFilter};
use uuid::Uuid;

use crate::voice::commands::music;

pub mod utils;
pub mod voice;

pub type Context<'a> = poise::Context<'a, Data, anyhow::Error>;

// User data, which is stored and accessible in all command invocations
#[derive(Debug)]
pub struct Data {
    http: HttpClient,
    songbird: Arc<songbird::Songbird>,
    track_metadata: Arc<Mutex<HashMap<Uuid, AuxMetadata>>>,
}

pub async fn client(token: String) -> Result<serenity::Client> {
    // color_eyre::install()?;

    setup_logging()?;

    #[cfg(debug_assertions)]
    let prefix = "~";

    #[cfg(not(debug_assertions))]
    let prefix = "aya";

    let manager = songbird::Songbird::serenity();

    let manager_clone = manager.clone();
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
            pre_command: |ctx: Context<'_>| {
                Box::pin(async move {
                    let command_name = ctx.command().qualified_name.clone();
                    let author = ctx.author();
                    let channel_id = ctx.channel_id();
                    let guild_id = ctx.guild_id();
                    info!("Command \"{command_name}\" called from channel {channel_id} in guild {guild_id:?} by {} ({})", author.name, author);
                })
            },
            on_error: |error: FrameworkError<'_, Data, anyhow::Error>| {
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
                    songbird: manager_clone,
                    track_metadata: Default::default(),
                })
            })
        })
        .build();

    let intents = serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::MESSAGE_CONTENT
        | serenity::GatewayIntents::GUILD_VOICE_STATES;

    serenity::Client::builder(&token, intents)
        .voice_manager_arc(manager)
        .framework(framework)
        .await
        .context("Error creating client")
}

fn setup_logging() -> Result<()> {
    // Init tracing with malaysian offset cause thats where i live and read timestamps
    let offset = UtcOffset::current_local_offset()
        .unwrap_or(UtcOffset::from_hms(8, 0, 0).unwrap_or(UtcOffset::UTC));
    let registry = tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_timer(OffsetTime::new(
                    offset,
                    time::format_description::well_known::Rfc3339,
                ))
                .with_thread_ids(true),
        )
        .with(ErrorLayer::default())
        .with(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        );
    set_global_default(registry)?;

    info!("log initialized with time offset {offset}");
    Ok(())
}

async fn event_handler(
    _ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, anyhow::Error>,
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
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"] command: Option<String>,
) -> Result<()> {
    let configuration = poise::builtins::HelpConfiguration {
        // [configure aspects about the help message here]
        ..Default::default()
    };
    poise::builtins::help(ctx, command.as_deref(), configuration).await?;
    Ok(())
}
