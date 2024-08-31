//! This module contains functions supporting the play command

use std::{collections::HashMap, sync::Arc};

use futures::{stream, StreamExt};
use poise::serenity_prelude as serenity;
use songbird::{
    input::{self, Compose},
    Event,
};
use tokio::sync::Mutex;
use tracing::{debug, error, info};

use crate::{
    error::{command_error_embed, BotError},
    utils::{get_guild_id, OptionExt},
    voice::{
        error::MusicCommandError,
        events::TrackPlayNotifier,
        utils::{self, metadata_to_embed, resolve_yt_playlist},
    },
    Context,
};

use super::join_inner;

/// This enum parses the given string and runs the appropriate process for the input
pub enum PlayParse {
    Search(String),
    Url(String),
    PlaylistUrl(String),
}

impl PlayParse {
    pub fn parse(input: &str) -> Self {
        if input.starts_with("http") {
            if input.contains("playlist") {
                return Self::PlaylistUrl(input.to_string());
            }

            Self::Url(input.to_string())
        } else {
            Self::Search(input.to_string())
        }
    }

    /// Handle the parsed input for play. Takes the poise context to facilitate communication
    pub async fn run(self, ctx: Context<'_>) -> Result<(), BotError> {
        let manager = ctx.data().songbird.clone();
        let guild_id = get_guild_id(ctx)?;
        let calling_channel_id = ctx.channel_id();
        let call = manager.get(guild_id);
        match self {
            PlayParse::Search(search) => {
                info!("searching youtube for: {}", search);
                let source = input::YoutubeDl::new_search(ctx.data().http.clone(), search);
                handle_single_play(call, calling_channel_id, source, ctx).await?;
            }
            PlayParse::Url(url) => {
                info!("using provided link: {}", url);
                let source = input::YoutubeDl::new(ctx.data().http.clone(), url);
                handle_single_play(call, calling_channel_id, source, ctx).await?;
            }
            PlayParse::PlaylistUrl(playlist_url) => {
                info!("using provided playlist link: {playlist_url}");
                ctx.reply("Handling playlist....").await?;

                let metadata_vec = resolve_yt_playlist(playlist_url).await?;

                let channel_id = ctx.channel_id();
                let call = manager.get(guild_id);

                // TODO: make it ordered
                let fut = stream::iter(metadata_vec).for_each_concurrent(20, |metadata| async {
                    if let Err(error) = handle_from_playlist(
                        metadata,
                        ctx.data().http.clone(),
                        ctx.data().track_metadata.clone(),
                        call.clone(),
                        ctx.serenity_context().http.clone(),
                        channel_id,
                    )
                    .await
                    {
                        let cmd = ctx.command().name.clone();
                        error!("Error executing command ({}): {}", cmd, error);

                        if let Err(e) = ctx
                            .send(
                                poise::CreateReply::default()
                                    .embed(command_error_embed(cmd, error)),
                            )
                            .await
                        {
                            error!("Error sending error message: {}", e);
                        }
                    };
                });

                fut.await;
            }
        }
        Ok(())
    }
}

/// Parses the input string and adds the result to the trackqueue
pub async fn play_inner(ctx: Context<'_>, input: String) -> Result<(), BotError> {
    let input_type = PlayParse::parse(&input);

    // join a channel first
    join_inner(ctx, false).await?;

    // TODO: check if youtube url

    input_type.run(ctx).await
}

/// Inserts a youtube source, sets events and notifies the calling channel
#[tracing::instrument(skip(ctx, call, source))]
async fn handle_single_play(
    call: Option<Arc<Mutex<songbird::Call>>>,
    calling_channel_id: serenity::ChannelId,
    source: input::YoutubeDl,
    ctx: Context<'_>,
) -> Result<(), BotError> {
    let metadata = insert_source(
        source,
        ctx.data().track_metadata.clone(),
        call,
        ctx.serenity_context().http.clone(),
        calling_channel_id,
    )
    .await?;

    let embed = metadata_to_embed(utils::EmbedOperation::AddToQueue, &metadata, None);
    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

// TODO: decide if this needs to be instrumented
async fn handle_from_playlist(
    metadata: utils::YoutubeMetadata,
    http: reqwest::Client,
    track_metadata: Arc<std::sync::Mutex<HashMap<uuid::Uuid, songbird::input::AuxMetadata>>>,
    call: Option<Arc<Mutex<songbird::Call>>>,
    serenity_http: Arc<serenity::Http>,
    calling_channel_id: serenity::ChannelId,
) -> Result<songbird::input::AuxMetadata, BotError> {
    // our ids are formatted into youtube links to prevent command line errors
    let youtube_link = format!("https://www.youtube.com/watch?v={}", metadata.youtube_id);

    debug!("Handling youtube link {youtube_link}");
    let source = input::YoutubeDl::new(http.clone(), youtube_link);
    insert_source(
        source,
        track_metadata,
        call,
        serenity_http,
        calling_channel_id,
    )
    .await
}

/// Process the given source, obtain its metadata and handle track insertion with events. This
/// function is made to be used with tokio::spawn
#[tracing::instrument(skip(track_metadata, call, serenity_http, calling_channel_id, source))]
async fn insert_source(
    mut source: input::YoutubeDl,
    track_metadata: Arc<std::sync::Mutex<HashMap<uuid::Uuid, songbird::input::AuxMetadata>>>,
    call: Option<Arc<Mutex<songbird::Call>>>,
    serenity_http: Arc<serenity::Http>,
    calling_channel_id: serenity::ChannelId,
) -> Result<songbird::input::AuxMetadata, BotError> {
    // TODO: rework this entire thing
    info!("Gathering metadata for source");
    match source.aux_metadata().await {
        Ok(metadata) => {
            let track: songbird::tracks::Track = source.into();
            let track_uuid = track.uuid;

            {
                let mut metadata_lock = track_metadata.lock().unwrap();
                metadata_lock.insert(track_uuid, metadata.clone());
            }

            // queue the next song few seconds before current song ends
            let preload_time = if let Some(duration) = metadata.duration {
                duration.checked_sub(std::time::Duration::from_secs(8))
            } else {
                None
            };

            if let Some(handler_lock) = &call {
                let mut handler = handler_lock.lock().await;
                let track_handle = handler.enqueue_with_preload(track, preload_time);

                let serenity_http_clone = serenity_http.clone();
                track_handle
                    .add_event(
                        Event::Track(songbird::TrackEvent::Play),
                        TrackPlayNotifier {
                            channel_id: calling_channel_id,
                            metadata: metadata.clone(),
                            http: serenity_http_clone,
                        },
                    )
                    .unwrap();
                info!(
                    "Added track {} ({}) to channel {calling_channel_id}",
                    metadata.title.clone().unwrap_or_unknown(),
                    metadata.channel.clone().unwrap_or_unknown()
                );
                // Logging added playlist in discord will be annoyying af
                Ok(metadata)
            } else {
                error!("Call does not exist...");
                return Err(MusicCommandError::CallDoesNotExist.into());
            }
        }
        Err(e) => {
            let err = format!("Unable to get metadata from youtube {e}");
            error!(err);
            return Err(MusicCommandError::TrackMetadataRetrieveFailed(e).into());
        }
    }
}
