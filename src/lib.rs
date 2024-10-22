use std::{
    collections::HashMap,
    io::{BufRead, BufReader, ErrorKind},
    sync::{Arc, Mutex},
};

use anyhow::{Context as _, Result};
use base64::Engine as _;
use error::{error_handler, BotError};
use memes::gay;
use poise::{
    serenity_prelude::{self as serenity},
    FrameworkError,
};
use reqwest::Client as HttpClient;
use songbird::input::AuxMetadata;
use time::UtcOffset;
use tokio::sync::RwLock;
use tracing::{debug, info, level_filters::LevelFilter, subscriber::set_global_default};
use tracing_error::ErrorLayer;
use tracing_subscriber::{fmt::time::OffsetTime, layer::SubscriberExt, EnvFilter};
use uuid::Uuid;
use voice::voice_commands;

use crate::voice::commands::music;

pub(crate) mod error;
pub(crate) mod memes;
pub(crate) mod utils;
pub(crate) mod voice;

pub type Context<'a> = poise::Context<'a, Data, BotError>;
pub type Commands = Vec<poise::Command<Data, BotError>>;

// User data, which is stored and accessible in all command invocations
#[derive(Debug)]
pub struct Data {
    http: HttpClient,
    songbird: Arc<songbird::Songbird>,
    track_metadata: Arc<Mutex<HashMap<Uuid, AuxMetadata>>>,
    user_id: RwLock<serenity::UserId>,
}

pub async fn client(token: String, loki: Option<LokiOpts>) -> Result<serenity::Client> {
    // color_eyre::install()?;

    setup_logging(loki).await?;

    #[cfg(debug_assertions)]
    let prefix = "~";

    #[cfg(not(debug_assertions))]
    let prefix = "aya";

    let manager = songbird::Songbird::serenity();

    // we do this for
    let mut commands = vec![about(), help(), ping(), music(), gay()];
    commands.append(&mut voice_commands());

    let manager_clone = manager.clone();
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands,
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some(prefix.into()),
                mention_as_prefix: true,
                case_insensitive_commands: true,
                execute_untracked_edits:true,
                edit_tracker: Some(Arc::new(poise::EditTracker::for_timespan(std::time::Duration::from_secs(20)))),
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
            on_error: |error: FrameworkError<'_, Data, BotError>| {
                Box::pin(
                    error_handler(error)
                )
            },
            event_handler: |ctx, event, framework,  data| {
                Box::pin(event_handler(ctx, event, framework,   data))
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
                    user_id: Default::default(),
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

async fn setup_logging(loki: Option<LokiOpts>) -> Result<()> {
    // Init tracing with malaysian offset cause thats where i live and read timestamps
    let offset = UtcOffset::current_local_offset()
        .unwrap_or(UtcOffset::from_hms(8, 0, 0).unwrap_or(UtcOffset::UTC));

    // TODO: simplify
    match loki {
        Some(loki) => {
            let url = url::Url::parse("https://logs-prod-020.grafana.net")?;

            let builder = tracing_loki::builder()
                .label("application", loki.application_log_label.clone())?
                .extra_field("pid", format!("{}", std::process::id()))?
                .http_header("Authorization", format!("Basic {}", loki.get_basic_auth()))?;

            let (layer, task) = builder.build_url(url)?;
            let registry = tracing_subscriber::registry()
                .with(
                    EnvFilter::builder()
                        .with_default_directive(LevelFilter::INFO.into())
                        .from_env_lossy()
                        .add_directive("ayaya_discord_bot=debug".parse()?),
                )
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_timer(OffsetTime::new(
                            offset,
                            time::format_description::well_known::Rfc3339,
                        ))
                        .with_thread_ids(true),
                )
                .with(ErrorLayer::default())
                .with(layer);
            set_global_default(registry)?;
            tokio::spawn(task);
        }
        None => {
            println!("Not sending logs to Grafana Loki");
            let registry = tracing_subscriber::registry()
                .with(
                    EnvFilter::builder()
                        .with_default_directive(LevelFilter::INFO.into())
                        .from_env_lossy()
                        .add_directive("ayaya_discord_bot=debug".parse()?),
                )
                .with(
                    tracing_subscriber::fmt::layer()
                        .with_timer(OffsetTime::new(
                            offset,
                            time::format_description::well_known::Rfc3339,
                        ))
                        .with_thread_ids(true),
                )
                .with(ErrorLayer::default());
            set_global_default(registry)?;
        }
    };

    info!("log initialized with time offset {offset}");
    debug!("debug logging is enabled for ayaya_discord_bot");
    Ok(())
}

async fn event_handler(
    _ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, BotError>,
    _data: &Data,
) -> Result<(), BotError> {
    match event {
        serenity::FullEvent::Ready { data_about_bot, .. } => {
            let bot_user_name = &data_about_bot.user.name;
            let session_id = &data_about_bot.session_id;
            let bot_user_id = data_about_bot.user.id;
            info!(
                "Logged in as {} with session id {}.",
                bot_user_name, session_id
            );
            {
                let mut user_id_lock = _data.user_id.write().await;
                *user_id_lock = bot_user_id;
            }

            // test yt-dlp
            let stdout = std::process::Command::new("yt-dlp")
                .arg("-J")
                .arg("-v")
                .arg("https://www.youtube.com/watch?v=KId6eunoiWk")
                .stderr(std::process::Stdio::piped())
                .spawn()
                .expect("yt-dlp runs")
                .stderr
                .ok_or_else(|| std::io::Error::new(ErrorKind::Other, "Could not capture stdout"))
                .expect("cant get yt-dlp stdout");

            let reader = BufReader::new(stdout);

            reader
                .lines()
                .map_while(Result::ok)
                .for_each(|line| info!("yt-dlp setup: {}", line));
        }
        serenity::FullEvent::CacheReady { guilds } => {
            info!("Cached guild info is ready for {} guilds.", guilds.len());
        }
        _ => {}
    }
    Ok(())
}

#[poise::command(prefix_command, slash_command)]
async fn ping(ctx: Context<'_>) -> Result<(), BotError> {
    ctx.reply("Pong!").await?;

    Ok(())
}

/// Ayaya likes to talk about herself...
#[poise::command(slash_command, prefix_command)]
async fn about(ctx: Context<'_>) -> Result<(), BotError> {
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
) -> Result<(), BotError> {
    let configuration = poise::builtins::HelpConfiguration {
        // [configure aspects about the help message here]
        ..Default::default()
    };
    poise::builtins::help(ctx, command.as_deref(), configuration).await?;
    Ok(())
}

pub struct LokiOpts {
    pub grafana_user: String,
    pub grafana_api_key: String,
    pub application_log_label: String,
}
impl LokiOpts {
    pub fn get_basic_auth(&self) -> String {
        let basic_auth = format!("{}:{}", self.grafana_user, self.grafana_api_key);
        base64::engine::general_purpose::STANDARD.encode(basic_auth.as_bytes())
    }
}
