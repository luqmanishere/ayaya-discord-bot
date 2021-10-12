use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;
use serenity::client::Context;
use serenity::framework::standard::CommandResult;
use serenity::model::channel::Message;
use serenity::Result as SerenityResult;
use songbird::Songbird;

use tokio::{process::Command as TokioCommand, task};

use eyre::{eyre, Result, WrapErr};
use tracing::{debug, info};

/// Checks that a message successfully sent; if not, then logs why to stdout.
pub fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}

pub async fn get_manager(ctx: &Context) -> Arc<Songbird> {
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

    let o_vec = String::from_utf8(youtube_dl.stdout)?;

    let out: Vec<String> = o_vec
        .split_terminator('\n')
        .map(|line| line.to_string())
        .collect();

    debug!("{} items: {:?}", &out.len(), &out);
    Ok(out)
}

#[allow(dead_code)]
#[allow(unused_variables)]
pub async fn yt_search(term: &str) -> Result<songbird::input::Metadata> {
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

    Ok(songbird::input::Metadata {
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
