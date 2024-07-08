use std::fmt::{self as fmt, Write};
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use ::serenity::all::Mentionable;
use eyre::{eyre, Result, WrapErr};
use poise::serenity_prelude as serenity;
use serde_json::Value;
use serenity::client::Context as SerenityContext;
use serenity::model::channel::Message;
use serenity::Result as SerenityResult;
use songbird::Songbird;
use tokio::{process::Command as TokioCommand, task};
use tracing::{debug, error, info};

use crate::Context;

/// Checks that a message successfully sent; if not, then logs why to stdout.
pub fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        error!("Error sending message: {:?}", why);
    }
}

pub async fn get_manager(ctx: &SerenityContext) -> Arc<Songbird> {
    songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
}

#[allow(unused_variables)]
#[allow(dead_code)]
pub async fn yt_9search(term: &str) -> Result<Vec<String>> {
    let ytdl_args = ["--get-title", "--ignore-config", "--skip-download"];

    let youtube_dl = TokioCommand::new("youtube-dl")
        .args(&ytdl_args)
        .arg(format!("ytsearch9:{}", term))
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .output()
        .await?;

    info!("Done searching!");

    let o_vec = std::str::from_utf8(&youtube_dl.stdout)?;

    let out: Vec<String> = o_vec
        .split_terminator('\n')
        .map(|line| line.to_owned())
        .collect();

    debug!("{} items: {:?}", &out.len(), &out);
    Ok(out)
}

#[allow(dead_code)]
#[allow(unused_variables)]
pub async fn yt_search(term: &str) -> Result<songbird::input::AuxMetadata> {
    let ytdl_args = ["--print-json", "--ignore-config", "--skip-download"];

    let mut youtube_dl = Command::new("youtube-dl")
        .args(&ytdl_args)
        .arg(term)
        .stdin(Stdio::null())
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let stdout = youtube_dl.stdout.take();
    let (returned_stdout, value) = task::spawn_blocking(move || {
        let mut s = stdout.unwrap();

        let out: Result<Value> = {
            let mut o_vec = vec![];
            let mut serde_read = BufReader::new(s.by_ref());
            // Newline...
            if let Ok(len) = serde_read.read_until(0xA, &mut o_vec) {
                serde_json::from_slice(&o_vec[..len])
                    .wrap_err_with(|| std::str::from_utf8(&o_vec).unwrap_or_default().to_string())
            } else {
                Result::Err(eyre!("Metadata error (1)"))
            }
        };

        (s, out)
    })
    .await
    .map_err(|_| eyre!("Metadata error (2)"))?;

    let value = value?;
    let obj = value.as_object();

    let track = obj
        .and_then(|m| m.get("track"))
        .and_then(Value::as_str)
        .map(str::to_string);

    let true_artist = obj
        .and_then(|m| m.get("artist"))
        .and_then(Value::as_str)
        .map(str::to_string);

    let artist = true_artist.or_else(|| {
        obj.and_then(|m| m.get("uploader"))
            .and_then(Value::as_str)
            .map(str::to_string)
    });

    let r_date = obj
        .and_then(|m| m.get("release_date"))
        .and_then(Value::as_str)
        .map(str::to_string);

    let date = r_date.or_else(|| {
        obj.and_then(|m| m.get("upload_date"))
            .and_then(Value::as_str)
            .map(str::to_string)
    });

    let channel = obj
        .and_then(|m| m.get("channel"))
        .and_then(Value::as_str)
        .map(str::to_string);

    let duration = obj
        .and_then(|m| m.get("duration"))
        .and_then(Value::as_f64)
        .map(Duration::from_secs_f64);

    let source_url = obj
        .and_then(|m| m.get("webpage_url"))
        .and_then(Value::as_str)
        .map(str::to_string);

    let title = obj
        .and_then(|m| m.get("title"))
        .and_then(Value::as_str)
        .map(str::to_string);

    let thumbnail = obj
        .and_then(|m| m.get("thumbnail"))
        .and_then(Value::as_str)
        .map(str::to_string);

    Ok(songbird::input::AuxMetadata {
        track,
        artist,
        date,

        channel,
        duration,
        source_url,
        title,
        thumbnail,

        ..Default::default()
    })
}

#[derive(Default, Clone, PartialEq, Eq, Debug)]
pub struct YTMetadata {
    pub track: Option<String>,
    pub artist: Option<String>,
    pub date: Option<String>,
    pub channel: Option<String>,
    pub start_time: Option<Duration>,
    pub duration: Option<Duration>,
    pub source_url: Option<String>,
    pub title: Option<String>,
    pub thumbnail: Option<String>,
}

pub trait OptionExt<String> {
    fn unwrap_or_unknown(self) -> String;
}

impl OptionExt<String> for Option<String> {
    fn unwrap_or_unknown(self) -> String {
        self.unwrap_or_else(|| "Unknown".to_string())
    }
}

pub async fn paginate(ctx: Context<'_>, pages: &[&str]) -> Result<(), serenity::Error> {
    // Define some unique identifiers for the navigation buttons
    let ctx_id = ctx.id();
    let prev_button_id = format!("{}prev", ctx_id);
    let next_button_id = format!("{}next", ctx_id);

    // Send the embed with the first page as content
    let reply = {
        let components = serenity::CreateActionRow::Buttons(vec![
            serenity::CreateButton::new(&prev_button_id).emoji('◀'),
            serenity::CreateButton::new(&next_button_id).emoji('▶'),
        ]);

        poise::CreateReply::default()
            .embed(serenity::CreateEmbed::default().description(pages[0]))
            .components(vec![components])
    };

    ctx.send(reply).await?;

    // Loop through incoming interactions with the navigation buttons
    let mut current_page = 0;
    while let Some(press) = serenity::collector::ComponentInteractionCollector::new(ctx)
        // We defined our button IDs to start with `ctx_id`. If they don't, some other command's
        // button was pressed
        .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
        // Timeout when no navigation button has been pressed for 24 hours
        .timeout(std::time::Duration::from_secs(3600 * 24))
        .await
    {
        // Depending on which button was pressed, go to next or previous page
        if press.data.custom_id == next_button_id {
            current_page += 1;
            if current_page >= pages.len() {
                current_page = 0;
            }
        } else if press.data.custom_id == prev_button_id {
            current_page = current_page.checked_sub(1).unwrap_or(pages.len() - 1);
        } else {
            // This is an unrelated button interaction
            continue;
        }

        // Update the message with the new page contents
        press
            .create_response(
                ctx.serenity_context(),
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .embed(serenity::CreateEmbed::new().description(pages[current_page])),
                ),
            )
            .await?;
    }

    Ok(())
}

pub enum EmbedOperation {
    YoutubeSearch,
    AddToQueue,
}

impl std::fmt::Display for EmbedOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let out = match self {
            EmbedOperation::YoutubeSearch => "Search Result",
            EmbedOperation::AddToQueue => "Added to Queue",
        };
        write!(f, "{out}")
    }
}

// TODO: extract static pictures out to somewhere

/// Converts AuxMetadata to a pretty embed
pub fn metadata_to_embed(
    operation: EmbedOperation,
    metadata: &songbird::input::AuxMetadata,
) -> serenity::CreateEmbed {
    // TODO: decide what to do with this unwrap
    serenity::CreateEmbed::default()
        .author(
            serenity::CreateEmbedAuthor::new(format!("{} | Youtube Video", operation)).icon_url(
                "https://cliply.co/wp-content/uploads/2019/04/371903520_SOCIAL_ICONS_YOUTUBE.png",
            ),
        )
        .description(
            serenity::MessageBuilder::default()
                .push_line(format!(
                    "### {}",
                    metadata.title.clone().unwrap_or_unknown()
                ))
                .to_string(),
        )
        .fields([
            (
                "Channel",
                metadata.channel.clone().unwrap_or_unknown(),
                true,
            ),
            (
                "Duration",
                humantime::format_duration(metadata.duration.unwrap()).to_string(),
                true,
            ),
        ])
        .thumbnail(
            metadata
                .thumbnail
                .clone()
                // thumbnail or broken
                .unwrap_or("https://cdn-icons-png.freepik.com/512/107/107817.png".to_string()),
        )
        .timestamp(serenity::Timestamp::now())
        .footer(serenity::CreateEmbedFooter::new("Ayaya Discord Bot"))
        .color(match operation {
            EmbedOperation::YoutubeSearch => serenity::Color::RED,
            EmbedOperation::AddToQueue => serenity::Color::MEIBE_PINK,
        })
}
