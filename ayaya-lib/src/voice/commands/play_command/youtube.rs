//! This module is a custom implementation of the YoutubeSource

use reqwest::{
    Client,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serenity::async_trait;
use songbird::input::{
    AudioStream, AudioStreamError, AuxMetadata, Compose, HlsRequest, HttpRequest, Input,
};
use std::error::Error;
use symphonia::core::io::MediaSource;
use tracing::{info, warn};
use youtube_dl::{Protocol, YoutubeDlOutput};

use crate::{
    data::stats::StatsManager,
    error::BotError,
    utils::OptionExt,
    voice::{
        error::MusicCommandError,
        utils::{AsYoutubeMetadata, YoutubeMetadata},
    },
};

const YOUTUBE_DL_COMMAND: &str = "yt-dlp";

#[derive(Clone, Debug)]
enum QueryType {
    Url(String),
    Search(String),
}

impl std::fmt::Display for QueryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let desc = match self {
            QueryType::Url(url) => url,
            QueryType::Search(search) => search,
        };

        f.write_str(desc)
    }
}

/// A lazily instantiated call to download a file, finding its URL via youtube-dl.
///
/// By default, this uses yt-dlp and is backed by an [`HttpRequest`]. This handler
/// attempts to find the best audio-only source (typically `WebM`, enabling low-cost
/// Opus frame passthrough).
///
/// [`HttpRequest`]: super::HttpRequest
#[derive(Clone, Debug)]
pub struct YoutubeDl {
    program: &'static str,
    client: Client,
    aux_metadata: Option<AuxMetadata>,
    youtube_metadata: Option<YoutubeMetadata>,
    query: QueryType,
    update_query_db: Option<StatsManager>,
}

impl YoutubeDl {
    /// Creates a lazy request to select an audio stream from `url`, using "yt-dlp".
    ///
    /// This requires a reqwest client: ideally, one should be created and shared between
    /// all requests.
    #[must_use]
    pub fn new(client: Client, url: String, update_query_db: Option<StatsManager>) -> Self {
        Self::new_ytdl_like(YOUTUBE_DL_COMMAND, client, url, update_query_db)
    }

    /// Creates a lazy request to select an audio stream from `url` as in [`new`], using `program`.
    ///
    /// [`new`]: Self::new
    #[must_use]
    pub fn new_ytdl_like(
        program: &'static str,
        client: Client,
        url: String,
        update_query_db: Option<StatsManager>,
    ) -> Self {
        Self {
            program,
            client,
            aux_metadata: None,
            youtube_metadata: None,
            query: QueryType::Url(url),
            update_query_db,
        }
    }

    /// Creates a request to search youtube for an optionally specified number of videos matching `query`,
    /// using "yt-dlp".
    #[must_use]
    pub fn new_search(
        client: Client,
        query: String,
        update_query_db: Option<StatsManager>,
    ) -> Self {
        Self::new_search_ytdl_like(YOUTUBE_DL_COMMAND, client, query, update_query_db)
    }

    /// Creates a request to search youtube for an optionally specified number of videos matching `query`,
    /// using `program`.
    #[must_use]
    pub fn new_search_ytdl_like(
        program: &'static str,
        client: Client,
        query: String,
        update_query_db: Option<StatsManager>,
    ) -> Self {
        Self {
            program,
            client,
            aux_metadata: None,
            youtube_metadata: None,
            query: QueryType::Search(query),
            update_query_db,
        }
    }

    /// Creates a request to select an audio stream from `url` with a set metadata
    pub fn new_url_with_metadata(
        client: Client,
        url: String,
        youtube_metadata: YoutubeMetadata,
        aux_metadata: AuxMetadata,
    ) -> Self {
        Self {
            program: YOUTUBE_DL_COMMAND,
            client,
            aux_metadata: Some(aux_metadata),
            youtube_metadata: Some(youtube_metadata),
            query: QueryType::Url(url),
            update_query_db: None,
        }
    }

    pub async fn new_playlist(
        client: Client,
        url: String,
    ) -> Result<(Vec<Self>, Option<youtube_dl::Playlist>), BotError> {
        let youtube_playlist = youtube_dl::YoutubeDl::new(url.clone())
            .flat_playlist(true)
            .extra_arg("-f")
            .extra_arg("ba[abr>0][vcodec=none]/best")
            .run_async()
            .await
            .map_err(|e| MusicCommandError::YoutubeDlError {
                source: e,
                args: url.clone(),
            })?;

        // TODO: cleanup
        let videos = match youtube_playlist {
            YoutubeDlOutput::Playlist(ref playlist) => playlist
                .entries
                .clone()
                .ok_or(MusicCommandError::YoutubeDlEmptyPlaylist { args: url })?,
            YoutubeDlOutput::SingleVideo(ref video) => vec![*video.clone()],
        };

        let metadata = videos
            .iter()
            .map(|e| {
                let youtube_metadata = e.as_youtube_metadata();
                let aux = youtube_metadata.as_aux_metadata();
                Self::new_url_with_metadata(
                    client.clone(),
                    format!("https://www.youtube.com/watch?v={}", e.id),
                    youtube_metadata,
                    aux,
                )
            })
            .collect::<Vec<_>>();

        Ok((metadata, youtube_playlist.into_playlist()))
    }

    pub fn youtube_metadata(&self) -> Option<YoutubeMetadata> {
        self.youtube_metadata.clone()
    }

    /// Query for single metadata
    #[tracing::instrument(skip_all, fields(self.query))]
    async fn query(&mut self) -> Result<YoutubeMetadata, AudioStreamError> {
        let new_query;
        let query_str = match &self.query {
            QueryType::Url(url) => url,
            QueryType::Search(query) => {
                new_query = format!("ytsearch1:{query}");
                &new_query
            }
        };
        info!("Querying for: {query_str}");

        let out = youtube_dl::YoutubeDl::new(query_str)
            .youtube_dl_path(self.program)
            .extra_arg("--no-playlist")
            .extra_arg("-f")
            .extra_arg("ba[abr>0][vcodec=none][protocol!=m3u8][protocol!=m3u8_native]")
            // .extra_arg("-4")
            // commentd out to try fix dropped args
            // .extra_arg("--extractor-args")
            // .extra_arg("youtube:player_client=tv,ios")
            .process_timeout(std::time::Duration::from_secs(45)) // the length of a youtube ad
            .run_async()
            .await
            .map_err(|e| AudioStreamError::Fail(Box::new(e)))?;

        let videos = match out {
            YoutubeDlOutput::Playlist(playlist) => playlist.entries.expect("playlist not empty"),
            YoutubeDlOutput::SingleVideo(video) => vec![*video],
        };

        let video = videos.into_iter().next().ok_or_else(|| {
            AudioStreamError::Fail(format!("no results found for '{query_str}'").into())
        })?;

        let youtube_metadata = video.as_youtube_metadata();

        let meta = youtube_metadata.as_aux_metadata();

        if let Some(stats) = &mut self.update_query_db {
            let text = format!(
                "{} ({})",
                video.title.clone().unwrap_or_unknown(),
                video.channel.clone().unwrap_or_unknown()
            );
            if let Err(e) = stats
                .update_user_play_queries_description(self.query.to_string(), text)
                .await
            {
                tracing::error!("Unable to update query descriptions from source: {e}");
            };
        };

        // set the query results
        self.youtube_metadata = Some(youtube_metadata.clone());
        self.aux_metadata = Some(meta);

        Ok(youtube_metadata)
    }
}

impl From<YoutubeDl> for Input {
    fn from(val: YoutubeDl) -> Self {
        Input::Lazy(Box::new(val))
    }
}

#[async_trait]
impl Compose for YoutubeDl {
    fn create(&mut self) -> Result<AudioStream<Box<dyn MediaSource>>, AudioStreamError> {
        Err(AudioStreamError::Unsupported)
    }

    async fn create_async(
        &mut self,
    ) -> Result<AudioStream<Box<dyn MediaSource>>, AudioStreamError> {
        // panic safety: `query` should have ensured > 0 results if `Ok`
        let result = self.query().await?;

        let mut headers = HeaderMap::default();

        if let Some(map) = result.http_headers {
            headers.extend(map.iter().filter_map(|(k, v)| {
                let header_value = v.clone().unwrap_or_default();
                Some((
                    HeaderName::from_bytes(k.as_bytes()).ok()?,
                    HeaderValue::from_str(header_value.as_str()).ok()?,
                ))
            }));
        }

        #[expect(clippy::single_match_else)]
        match result.protocol {
            Some(Protocol::M3U8Native) => {
                tracing::debug!("Using HLS, url: {}", result.url);
                let mut req =
                    HlsRequest::new_with_headers(self.client.clone(), result.url, headers);

                req.create_async().await
            }
            _ => {
                tracing::debug!("Using HTTP, url: {}", result.url);
                let mut req = HttpRequest {
                    client: self.client.clone(),
                    request: result.url,
                    headers,
                    content_length: result.filesize,
                };

                req.create_async().await
            }
        }
    }

    fn should_create_async(&self) -> bool {
        true
    }

    async fn aux_metadata(&mut self) -> Result<AuxMetadata, AudioStreamError> {
        if let Some(meta) = self.aux_metadata.as_ref() {
            return Ok(meta.clone());
        }

        warn!("no metadata found");
        self.query().await?;

        self.aux_metadata.clone().ok_or_else(|| {
            let msg: Box<dyn Error + Send + Sync + 'static> =
                "Failed to instantiate any metadata... Should be unreachable.".into();
            AudioStreamError::Fail(msg)
        })
    }
}
