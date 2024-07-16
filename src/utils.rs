use std::fmt::{self as fmt};
use std::ops::Sub;
use std::sync::Arc;
use std::time::Duration;

use eyre::{eyre, ContextCompat, Result};
use poise::serenity_prelude as serenity;
use serenity::client::Context as SerenityContext;
use serenity::model::channel::Message;
use serenity::Result as SerenityResult;
use songbird::Songbird;
use tracing::error;
use youtube_dl::{SearchOptions, SingleVideo, YoutubeDlOutput};

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct YoutubeMetadata {
    pub track: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub date: Option<String>,
    pub channels: Option<u8>,
    pub channel: Option<String>,
    pub start_time: Option<Duration>,
    pub duration: Option<Duration>,
    pub sample_rate: Option<u32>,
    pub source_url: Option<String>,
    pub title: Option<String>,
    pub thumbnail: Option<String>,
    pub youtube_id: String,
}

impl YoutubeMetadata {
    pub fn aux_metadata(&self) -> songbird::input::AuxMetadata {
        Into::<AuxMetadataWrap>::into(self.clone()).0
    }
}

pub struct AuxMetadataWrap(songbird::input::AuxMetadata);

impl From<YoutubeMetadata> for AuxMetadataWrap {
    fn from(value: YoutubeMetadata) -> Self {
        AuxMetadataWrap(songbird::input::AuxMetadata {
            track: value.track,
            artist: value.artist,
            album: value.album,
            date: value.date,
            channels: value.channels,
            channel: value.channel,
            start_time: value.start_time,
            duration: value.duration,
            sample_rate: value.sample_rate,
            source_url: value.source_url,
            title: value.title,
            thumbnail: value.thumbnail,
        })
    }
}

impl From<SingleVideo> for YoutubeMetadata {
    fn from(value: SingleVideo) -> Self {
        Self {
            track: value.track,
            artist: value.artist,
            album: value.album,
            date: value.upload_date,
            channels: None,
            channel: value.channel,
            start_time: None,
            duration: value
                .duration
                .map(|e| std::time::Duration::from_secs(e.as_u64().expect("duration is integer"))),
            sample_rate: None,
            source_url: value.url,
            title: value.title,
            thumbnail: value.thumbnail,
            youtube_id: value.id,
        }
    }
}

pub async fn yt_search(term: &str, count: Option<usize>) -> Result<Vec<YoutubeMetadata>> {
    let search_options = SearchOptions::youtube(term).with_count(count.unwrap_or(10));
    let youtube_search = youtube_dl::YoutubeDl::search_for(&search_options)
        .run_async()
        .await?;

    let videos = match youtube_search {
        YoutubeDlOutput::Playlist(playlist) => {
            playlist.entries.wrap_err("expect playlist has entries")?
        }
        YoutubeDlOutput::SingleVideo(video) => vec![*video],
    };

    let metadata_vec = videos
        .iter()
        .map(|e| Into::<YoutubeMetadata>::into(e.clone()))
        .collect::<Vec<_>>();

    Ok(metadata_vec)
}

pub trait OptionExt<String> {
    fn unwrap_or_unknown(self) -> String;
}

impl OptionExt<String> for Option<String> {
    fn unwrap_or_unknown(self) -> String {
        self.unwrap_or_else(|| "Unknown".to_string())
    }
}

#[allow(dead_code)]
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

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum EmbedOperation {
    YoutubeSearch,
    AddToQueue,
    NowPlayingNotification,
    NowPlaying,
    SkipSong,
    ErrorNotPlaying,
    ErrorNotInVoiceChannel,
}

impl std::fmt::Display for EmbedOperation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let out = match self {
            EmbedOperation::YoutubeSearch => "Search Result",
            EmbedOperation::AddToQueue => "Added to Queue",
            EmbedOperation::NowPlayingNotification | EmbedOperation::NowPlaying => "Now Playing",
            EmbedOperation::SkipSong => "Skipping Song",
            EmbedOperation::ErrorNotPlaying => "No Songs Playing",
            Self::ErrorNotInVoiceChannel => "Not in Voice Channel",
        };
        write!(f, "{out}")
    }
}

// TODO: extract static pictures out to somewhere

/// Converts AuxMetadata to a pretty embed
pub fn metadata_to_embed(
    operation: EmbedOperation,
    metadata: &songbird::input::AuxMetadata,
    track_state: Option<&songbird::tracks::TrackState>,
) -> serenity::CreateEmbed {
    let mut embed = serenity::CreateEmbed::default()
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
        );

    embed = embed.fields([
        (
            "Channel",
            metadata.channel.clone().unwrap_or_unknown(),
            true,
        ),
        (
            "Duration",
            // TODO: decide what to do with this unwrap
            humantime::format_duration(metadata.duration.unwrap_or_default()).to_string(),
            true,
        ),
    ]);

    // extra conditional fields
    if let Some(track_state) = track_state {
        match operation {
            EmbedOperation::SkipSong => {
                let current_pos = track_state.position;
                let duration = metadata.duration.unwrap_or_default();
                let time_remaining = duration.sub(current_pos);

                embed = embed.field(
                    "Time Remaining",
                    humantime::format_duration(time_remaining).to_string(),
                    true,
                );
            }
            EmbedOperation::NowPlaying => {
                let current_pos = track_state.position;
                let duration = metadata.duration.unwrap_or_default();
                let time_remaining = duration.sub(current_pos);

                embed = embed.fields([
                    (
                        "Current Time",
                        humantime::format_duration(current_pos).to_string(),
                        true,
                    ),
                    (
                        "Time Remaining",
                        humantime::format_duration(time_remaining).to_string(),
                        true,
                    ),
                ]);
            }
            _ => {}
        }
    }

    embed = embed
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
            EmbedOperation::NowPlayingNotification | EmbedOperation::NowPlaying => {
                serenity::Color::DARK_GREEN
            }
            EmbedOperation::SkipSong => serenity::Color::ORANGE,
            _ => serenity::Color::MEIBE_PINK,
        });

    embed
}

pub fn error_embed(operation: EmbedOperation) -> serenity::CreateEmbed {
    serenity::CreateEmbed::default()
        .color(serenity::Color::RED)
        .author(
            serenity::CreateEmbedAuthor::new(format!("Error | {}", operation)).icon_url(
                "https://cliply.co/wp-content/uploads/2019/04/371903520_SOCIAL_ICONS_YOUTUBE.png",
            ),
        )
        .description(
            serenity::MessageBuilder::default()
                .push_line(format!("### {}: {}", "Error", operation))
                .push_line(match operation {
                    EmbedOperation::ErrorNotInVoiceChannel => {
                        "Please join a voice channel to run this command."
                    }
                    EmbedOperation::ErrorNotPlaying => "No songs are being played in the server.",
                    _ => "Undefined error",
                })
                .to_string(),
        )
        .timestamp(serenity::Timestamp::now())
        .footer(serenity::CreateEmbedFooter::new("Ayaya Discord Bot"))
}

/// Create an interaction for the search command. Returns the selected video id if any
pub async fn create_search_interaction(
    ctx: Context<'_>,
    metadata_vec: Vec<YoutubeMetadata>,
) -> Result<String> {
    // Define some unique identifiers for the navigation buttons
    let ctx_id = ctx.id();
    let prev_button_id = format!("{}prev", ctx_id);
    let next_button_id = format!("{}next", ctx_id);

    let button_id_gen = |count: usize| format!("{ctx_id}-search-{count}");

    // TODO: optimize?
    let metadata_embeds = metadata_vec
        .iter()
        .map(|e| metadata_to_embed(EmbedOperation::YoutubeSearch, &e.aux_metadata(), None))
        .collect::<Vec<_>>();
    let metadata_embed_chunks = metadata_embeds.chunks(3).collect::<Vec<_>>();

    let metadata_chunks = metadata_vec.chunks(3).collect::<Vec<_>>();

    // Send the embed with the first page as content
    let reply = {
        let mut buttons = vec![serenity::CreateButton::new(&prev_button_id).emoji('◀')];
        let mut reply = poise::CreateReply::default();

        for (i, embed) in metadata_embed_chunks[0].iter().enumerate() {
            reply = reply.embed(embed.to_owned());
            buttons.push(
                serenity::CreateButton::new(button_id_gen(i + 1)).label(format!("{}", i + 1)),
            );
        }
        buttons.push(serenity::CreateButton::new(&next_button_id).emoji('▶'));

        let components = serenity::CreateActionRow::Buttons(buttons);
        reply.components(vec![components])
    };

    ctx.send(reply).await?;

    // Loop through incoming interactions with the navigation buttons
    let mut current_page = 0;
    while let Some(press) = serenity::collector::ComponentInteractionCollector::new(ctx)
        // We defined our button IDs to start with `ctx_id`. If they don't, some other command's
        // button was pressed
        .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
        // Timeout when no navigation button has been pressed for 1 minute
        .timeout(std::time::Duration::from_secs(60))
        .await
    {
        // Depending on which button was pressed, go to next or previous page
        if press.data.custom_id == next_button_id {
            current_page += 1;
            if current_page >= metadata_embed_chunks.len() {
                current_page = 0;
            }
        } else if press.data.custom_id == prev_button_id {
            current_page = current_page
                .checked_sub(1)
                .unwrap_or(metadata_embed_chunks.len() - 1);
        } else if press.data.custom_id == button_id_gen(1) {
            // TODO: simplify
            let metadata = metadata_chunks[current_page][0].clone();
            return Ok(metadata.youtube_id);
        } else if press.data.custom_id == button_id_gen(2) {
            let metadata = metadata_chunks[current_page][1].clone();
            return Ok(metadata.youtube_id);
        } else if press.data.custom_id == button_id_gen(3) {
            let metadata = metadata_chunks[current_page][2].clone();
            return Ok(metadata.youtube_id);
        } else {
            // This is an unrelated button interaction
            continue;
        }

        let response = {
            let mut buttons = vec![serenity::CreateButton::new(&prev_button_id).emoji('◀')];
            let mut response = serenity::CreateInteractionResponseMessage::new();

            for (i, embed) in metadata_embed_chunks[current_page].iter().enumerate() {
                response = response.add_embed(embed.to_owned());
                buttons.push(
                    serenity::CreateButton::new(button_id_gen(i + 1)).label(format!("{}", i + 1)),
                );
            }
            buttons.push(serenity::CreateButton::new(&next_button_id).emoji('▶'));

            let components = serenity::CreateActionRow::Buttons(buttons);
            response.components(vec![components])
        };

        // Update the message with the new page contents
        press
            .create_response(
                ctx.serenity_context(),
                serenity::CreateInteractionResponse::UpdateMessage(response),
            )
            .await?;
    }
    Err(eyre!("No selection made before timeout"))
}
