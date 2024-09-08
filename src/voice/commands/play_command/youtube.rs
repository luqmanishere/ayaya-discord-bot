//! This module is a custom implementation of the YoutubeSource

use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client,
};
use serenity::async_trait;
use songbird::input::{
    AudioStream, AudioStreamError, AuxMetadata, Compose, HlsRequest, HttpRequest, Input,
};
use std::error::Error;
use symphonia::core::io::MediaSource;
use youtube_dl::{Protocol, YoutubeDlOutput};

use crate::{
    error::BotError,
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
    metadata: Option<AuxMetadata>,
    query: QueryType,
}

impl YoutubeDl {
    /// Creates a lazy request to select an audio stream from `url`, using "yt-dlp".
    ///
    /// This requires a reqwest client: ideally, one should be created and shared between
    /// all requests.
    #[must_use]
    pub fn new(client: Client, url: String) -> Self {
        Self::new_ytdl_like(YOUTUBE_DL_COMMAND, client, url)
    }

    /// Creates a lazy request to select an audio stream from `url` as in [`new`], using `program`.
    ///
    /// [`new`]: Self::new
    #[must_use]
    pub fn new_ytdl_like(program: &'static str, client: Client, url: String) -> Self {
        Self {
            program,
            client,
            metadata: None,
            query: QueryType::Url(url),
        }
    }

    /// Creates a request to search youtube for an optionally specified number of videos matching `query`,
    /// using "yt-dlp".
    #[must_use]
    pub fn new_search(client: Client, query: String) -> Self {
        Self::new_search_ytdl_like(YOUTUBE_DL_COMMAND, client, query)
    }

    /// Creates a request to search youtube for an optionally specified number of videos matching `query`,
    /// using `program`.
    #[must_use]
    pub fn new_search_ytdl_like(program: &'static str, client: Client, query: String) -> Self {
        Self {
            program,
            client,
            metadata: None,
            query: QueryType::Search(query),
        }
    }

    pub async fn new_playlist(client: Client, url: String) -> Result<Vec<Self>, BotError> {
        let youtube_playlist = youtube_dl::YoutubeDl::new(url.clone())
            .flat_playlist(true)
            .run_async()
            .await
            .map_err(|e| MusicCommandError::YoutubeDlError {
                source: e,
                args: url.clone(),
            })?;

        let videos = match youtube_playlist {
            YoutubeDlOutput::Playlist(playlist) => playlist
                .entries
                .ok_or(MusicCommandError::YoutubeDlEmptyPlaylist { args: url })?,
            YoutubeDlOutput::SingleVideo(video) => vec![*video],
        };

        let urls = videos
            .iter()
            .map(|e| format!("https://www.youtube.com/watch?v={}", e.id))
            .collect::<Vec<_>>();

        Ok(urls
            .into_iter()
            .map(|e| Self::new(client.clone(), e))
            .collect())
    }

    /// Runs a search for the given query, returning a list of up to `n_results`
    /// possible matches which are `AuxMetadata` objects containing a valid URL.
    ///
    /// Returns up to 5 matches by default.
    pub async fn search(
        &mut self,
        n_results: Option<usize>,
    ) -> Result<Vec<AuxMetadata>, AudioStreamError> {
        let n_results = n_results.unwrap_or(5);

        Ok(match &self.query {
            // Safer to just return the metadata for the pointee if possible
            QueryType::Url(_) => vec![self.aux_metadata().await?],
            QueryType::Search(_) => self
                .query(n_results)
                .await?
                .into_iter()
                .map(|v| v.as_aux_metadata())
                .collect(),
        })
    }

    async fn query(&mut self, n_results: usize) -> Result<Vec<YoutubeMetadata>, AudioStreamError> {
        let new_query;
        let query_str = match &self.query {
            QueryType::Url(url) => url,
            QueryType::Search(query) => {
                new_query = format!("ytsearch{n_results}:{query}");
                &new_query
            }
        };

        let out = youtube_dl::YoutubeDl::new(query_str)
            .youtube_dl_path(self.program)
            .extra_arg("--no-playlist")
            .extra_arg("-f")
            .extra_arg("ba[abr>0][vcodec=none]/best")
            .run_async()
            .await
            .map_err(|e| AudioStreamError::Fail(Box::new(e)))?;

        let videos = match out {
            YoutubeDlOutput::Playlist(playlist) => playlist.entries.expect("playlist not empty"),
            YoutubeDlOutput::SingleVideo(video) => vec![*video],
        };

        let out = videos
            .into_iter()
            .map(|e| e.as_youtube_metadata())
            .collect::<Vec<_>>();

        let meta = out
            .first()
            .ok_or_else(|| {
                AudioStreamError::Fail(format!("no results found for '{query_str}'").into())
            })?
            .as_aux_metadata();

        self.metadata = Some(meta);

        Ok(out)
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
        let mut results = self.query(1).await?;
        let result = results.swap_remove(0);

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

        #[allow(clippy::single_match_else)]
        match result.protocol {
            Some(Protocol::M3U8Native) => {
                let mut req =
                    HlsRequest::new_with_headers(self.client.clone(), result.url, headers);
                // TODO: monitor if making this async breaks anything
                req.create_async().await
            }
            _ => {
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
        if let Some(meta) = self.metadata.as_ref() {
            return Ok(meta.clone());
        }

        self.query(1).await?;

        self.metadata.clone().ok_or_else(|| {
            let msg: Box<dyn Error + Send + Sync + 'static> =
                "Failed to instantiate any metadata... Should be unreachable.".into();
            AudioStreamError::Fail(msg)
        })
    }
}
