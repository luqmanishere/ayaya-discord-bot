//! This module contains functions supporting the play command

use std::sync::Arc;

use poise::serenity_prelude as serenity;
use rand::seq::SliceRandom;
use songbird::{input::Compose, tracks::Track, Event};
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::{
    data::stats::StatsManager,
    error::BotError,
    utils::{get_guild_id, GuildInfo, OptionExt},
    voice::{
        commands::play_command::youtube,
        error::MusicCommandError,
        events::TrackPlayNotifier,
        utils::{self, metadata_to_embed, playlist_to_embed, YoutubeMetadata},
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
    pub fn parse(ctx: Context<'_>, input: &str) -> Self {
        let mut data_manager = ctx.data().data_manager.clone();
        let new_input = if let Some(value) = data_manager.get_autocomplete(input.to_string()) {
            value
        } else {
            input.to_string()
        };

        if new_input.starts_with("http") {
            let url = url::Url::parse(&new_input).unwrap();
            let pairs = url.query_pairs().filter(|(name, _)| !name.eq("si"));
            let mut url = url.clone();
            url.query_pairs_mut().clear().extend_pairs(pairs);

            if new_input.contains("playlist") {
                return Self::PlaylistUrl(url.to_string());
            }

            Self::Url(url.to_string())
        } else {
            Self::Search(new_input.to_string())
        }
    }

    /// Handle the parsed input for play. Takes the poise context to facilitate communication
    pub async fn run(self, ctx: Context<'_>, shuffle: bool) -> Result<(), BotError> {
        let manager = ctx.data().songbird.clone();
        let guild_id = get_guild_id(ctx)?;
        let calling_channel_id = ctx.channel_id();
        let call = manager.get(guild_id);
        let sources = match self {
            PlayParse::Search(ref search) => {
                info!("searching youtube for: {}", search);

                ctx.data()
                    .data_manager
                    .stats()
                    .add_user_play_query(
                        guild_id.get(),
                        ctx.author(),
                        search.to_string(),
                        self.to_string(),
                        "".to_string(),
                    )
                    .await?;
                let source = youtube::YoutubeDl::new_search(
                    ctx.data().http.clone(),
                    search.clone(),
                    Some(ctx.data().data_manager.stats()),
                );

                vec![source]
            }
            PlayParse::Url(ref url) => {
                info!("using provided link: {}", url);
                ctx.data()
                    .data_manager
                    .stats()
                    .add_user_play_query(
                        guild_id.get(),
                        ctx.author(),
                        url.to_string(),
                        self.to_string(),
                        "".to_string(),
                    )
                    .await?;

                let source = youtube::YoutubeDl::new(
                    ctx.data().http.clone(),
                    url.clone(),
                    Some(ctx.data().data_manager.stats()),
                );

                vec![source]
            }
            PlayParse::PlaylistUrl(ref playlist_url) => {
                info!("using provided playlist link: {playlist_url}");

                let (mut playlist, playlist_info) =
                    youtube::YoutubeDl::new_playlist(ctx.data().http.clone(), playlist_url.clone())
                        .await?;

                if let Some(playlist_info) = playlist_info {
                    tracing::warn!("adding playlist info");
                    ctx.data()
                        .data_manager
                        .stats()
                        .add_user_play_query(
                            guild_id.get(),
                            ctx.author(),
                            playlist_url.to_string(),
                            self.to_string(),
                            playlist_info.title.clone().unwrap_or_default(),
                        )
                        .await?;

                    // broadcast playlist info
                    let embed = playlist_to_embed(
                        &utils::EmbedOperation::NewPlaylist,
                        &playlist_info,
                        Some(ctx.author()),
                    );

                    ctx.send(poise::CreateReply::default().embed(embed)).await?;
                } else {
                    tracing::error!("no playlist info");
                }

                if shuffle {
                    let mut rng = rand::thread_rng();
                    playlist.shuffle(&mut rng);
                    playlist
                } else {
                    playlist
                }
            }
        };
        handle_sources(call, calling_channel_id, sources, ctx).await?;
        Ok(())
    }
}

impl std::fmt::Display for PlayParse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let desc = match self {
            PlayParse::Search(_) => "Search",
            PlayParse::Url(_) => "Url",
            PlayParse::PlaylistUrl(_) => "Playlist",
        };
        f.write_str(desc)
    }
}

/// Parses the input string and adds the result to the trackqueue
pub async fn play_inner(ctx: Context<'_>, input: String, shuffle: bool) -> Result<(), BotError> {
    let input_type = PlayParse::parse(ctx, &input);

    // join a channel first
    join_inner(ctx, false).await?;

    input_type.run(ctx, shuffle).await
}

/// Inserts a youtube source, sets events and notifies the calling channel
#[tracing::instrument(skip(ctx, call, sources))]
async fn handle_sources(
    call: Option<Arc<Mutex<songbird::Call>>>,
    calling_channel_id: serenity::ChannelId,
    sources: Vec<youtube::YoutubeDl>,
    ctx: Context<'_>,
) -> Result<(), BotError> {
    let stats = ctx.data().data_manager.stats();
    let guild_info = GuildInfo::from_ctx(ctx)?;
    let user = ctx.author();
    // do not announce if more than 1 track is added
    match sources.len() {
        1 => {
            let metadata = insert_source(
                sources.first().expect("length should be 1").clone(),
                call,
                ctx.serenity_context().http.clone(),
                calling_channel_id,
                stats,
                user.clone(),
                guild_info.guild_id,
            )
            .await?;

            let embed = metadata_to_embed(utils::EmbedOperation::AddToQueue, &metadata, None);
            ctx.send(poise::CreateReply::default().embed(embed)).await?;
        }
        _num if _num > 1 => {
            for source in sources {
                insert_source(
                    source,
                    call.clone(),
                    ctx.serenity_context().http.clone(),
                    calling_channel_id,
                    stats.clone(),
                    user.clone(),
                    guild_info.guild_id,
                )
                .await?;
            }
        }
        _ => {
            return Err(BotError::MusicCommandError(MusicCommandError::EmptySource));
        }
    };

    Ok(())
}

/// Process the given source, obtain its metadata and handle track insertion with events. This
/// function is made to be used with tokio::spawn
#[tracing::instrument(skip(
    call,
    serenity_http,
    calling_channel_id,
    source,
    stats,
    user,
    guild_id
))]
async fn insert_source(
    mut source: youtube::YoutubeDl,
    call: Option<Arc<Mutex<songbird::Call>>>,
    serenity_http: Arc<serenity::Http>,
    calling_channel_id: serenity::ChannelId,
    stats: StatsManager,
    user: serenity::User,
    guild_id: serenity::GuildId,
) -> Result<YoutubeMetadata, BotError> {
    // TODO: rework this entire thing
    info!("Gathering metadata for source");
    match source.aux_metadata().await {
        Ok(metadata) => {
            // TODO: store this in the hashmap
            let mut youtube = source
                .youtube_metadata()
                .expect("youtube metadata initialized");
            // the user context is still the same, so we can directly add it here
            youtube.requester = Some(user.clone());

            let desc = format!(
                "{} Ch: {}",
                youtube.title.clone().unwrap_or_unknown(),
                youtube.channel.clone().unwrap_or_unknown()
            );
            stats
                .add_song_queue_count(
                    guild_id.get(),
                    &user,
                    youtube.youtube_id.clone(),
                    Some(desc),
                )
                .await?;

            let track = Track::new_with_data(source.into(), std::sync::Arc::new(youtube.clone()));

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
                            metadata: youtube.clone(),
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
                Ok(youtube)
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

/// Autocomplete for the play commands
pub async fn autocomplete_play(
    ctx: Context<'_>,
    partial: &str,
) -> impl Iterator<Item = serenity::AutocompleteChoice> {
    let guild_id = GuildInfo::from_ctx(ctx).unwrap().guild_id;
    let user = ctx.author();
    // make all comparisons lowercase == easier comparisons
    let partial = partial.to_lowercase();
    let mut data_manager = ctx.data().data_manager.clone();

    let mut completions = ctx
        .data()
        .data_manager
        .stats()
        .get_user_play_queries(guild_id.get(), user)
        .await
        .unwrap();

    // sort then reverse, so the most common sits above
    completions.sort_by_key(|e| e.count);
    completions.reverse();

    completions
        .into_iter()
        .filter(move |s| {
            s.query.to_lowercase().contains(&partial)
                || s.description.to_lowercase().contains(&partial)
                || s.query_type.to_lowercase().contains(&partial)
        })
        .map(move |e| {
            let query = if e.query.len() > 90 {
                // discord max length for value is 100, so we cache the value and sub in a uuid
                let uuid = uuid::Uuid::new_v4();
                data_manager.add_autocomplete(uuid.to_string(), e.query.clone());
                uuid.to_string()
            } else {
                // else just show the query
                e.query.clone()
            };

            let name = match e.query_type.as_str() {
                "Playlist" => {
                    // show the name of the playlist
                    format!("Playlist: {}", e.description)
                }
                _ => {
                    format!(
                        "Query:{} | LastResult:{}[{}]",
                        e.query, e.query_type, e.description
                    )
                }
            };

            serenity::AutocompleteChoice::new(first_n_chars(name.as_str(), 100), query)
        })
}

/// Truncate chars to a provided index
fn first_n_chars(s: &str, n: usize) -> &str {
    if let Some((x, _)) = s.char_indices().nth(n) {
        &s[..x]
    } else {
        s
    }
}
