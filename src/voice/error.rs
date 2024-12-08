use miette::Diagnostic;
use poise::serenity_prelude::{self as serenity};
use thiserror::Error;

use crate::{
    error::ErrorName,
    utils::{ChannelInfo, GuildInfo},
};

#[derive(Error, Diagnostic, Debug)]
pub enum MusicCommandError {
    #[error("Ayaya has not joined any voice channels in the guild {} ({})", guild_info.guild_id, guild_info.guild_name)]
    #[diagnostic(help("Try joining Ayaya to a voice channel with the join command."))]
    BotVoiceNotJoined { guild_info: GuildInfo },

    #[error(
        "Ayaya can't find user {} ({}) in any voice channel in the guild {} ({})",
        user.name, user.id, guild_info.guild_name, guild_info.guild_id
    )]
    #[diagnostic(help("Try joining a voice channel before running the command."))]
    UserVoiceNotJoined {
        user: serenity::User,
        guild_info: GuildInfo,
    },

    #[error(
        "Failed to join voice channel {} ({}) in guild {} ({}) with error {source}",
        voice_channel_info.channel_name,
        voice_channel_info.channel_id,
        guild_info.guild_name,
        guild_info.guild_id
    )]
    #[diagnostic(help("Contact @solemnattic for assistance"))]
    FailedJoinCall {
        source: songbird::error::JoinError,
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
    },

    #[error(
        "Failed to leave voice channel {} ({}) in guild {} ({}) with error: {source}",
        voice_channel_info.channel_name,
        voice_channel_info.channel_id,
        guild_info.guild_name,
        guild_info.guild_id
    )]
    #[diagnostic(help("Contact @solemnattic for assistance or just yeet her."))]
    FailedLeaveCall {
        source: songbird::error::JoinError,
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
    },

    #[error("Failed to unmute voice channel {} ({}) in guild {} ({}) due to {source}", voice_channel_info.channel_name, voice_channel_info.channel_id, guild_info.guild_name, guild_info.guild_id)]
    #[diagnostic(help("Contact @solemnattic for assistance"))]
    FailedUnmuteCall {
        source: songbird::error::JoinError,
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
    },

    #[error(
        "Failed to deafen Ayaya in voice channel {} ({}) in guild {} ({}) with error: {source}",
        voice_channel_info.channel_name,
        voice_channel_info.channel_id,
        guild_info.guild_name,
        guild_info.guild_id
    )]
    #[diagnostic(help("Contact @solemnattic for assistance"))]
    FailedDeafenCall {
        source: songbird::error::JoinError,
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
    },

    #[error(
        "Failed to undeafen Ayaya in voice channel {} ({}) in guild {} ({}) with error: {source}",
        voice_channel_info.channel_name,
        voice_channel_info.channel_id,
        guild_info.guild_name,
        guild_info.guild_id
    )]
    #[diagnostic(help("Contact @solemnattic for assistance"))]
    FailedUndeafenCall {
        source: songbird::error::JoinError,
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
    },

    #[error("Ayaya can't find the bond between her and this guild. Time for a reboot perhaps?")]
    #[diagnostic(help("Contact @solemnattic for assistance."))]
    CallDoesNotExist,

    #[error("An error occured with youtube-dl while processing query \"{args}\" : {source}")]
    #[diagnostic(help("Ayaya thinks youtube is being stupid tonight. Just try again"))]
    YoutubeDlError {
        source: youtube_dl::Error,
        args: String,
    },

    #[error("Empty playlist returned from \"{args}\", Ayaya has nothing to play")]
    #[diagnostic(help("Ayaya thinks youtube is being stupid tonight. Just try again."))]
    YoutubeDlEmptyPlaylist { args: String },

    #[error("Ayaya is unable to find the track metadata for uuid: {track_uuid}")]
    #[diagnostic(help(
        "Ayaya forgot something important, contact her owner @solemnattic to find out what."
    ))]
    TrackMetadataNotFound { track_uuid: uuid::Uuid },

    #[error("Failed to retrieve metadata for the track: {0}")]
    #[diagnostic(help(
        "Ayaya thinks YouTube is being stupid tonight. Tell @solemnattic to check what's up"
    ))]
    TrackMetadataRetrieveFailed(#[from] songbird::input::AudioStreamError),

    #[error("Ayaya is unable to find the track state for uuid: {track_uuid}")]
    #[diagnostic(help(
        "Ayaya forgot something important, contact her owner @solemnattic to help her."
    ))]
    TrackStateNotFound {
        source: songbird::error::ControlError,
        track_uuid: uuid::Uuid,
    },

    #[error(
        "Failed to skip the track with uuid {track_uuid} in voice channel {} ({}) in guild {} ({}): {source}",
        voice_channel_info.channel_name,
        voice_channel_info.channel_id,
        guild_info.guild_name,
        guild_info.guild_id
    )]
    #[diagnostic(help("Contact @solemnattic for assistance"))]
    FailedTrackSkip {
        source: songbird::error::ControlError,
        track_uuid: uuid::Uuid,
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
    },

    #[error(
        "Failed to pause the track with uuid {track_uuid} in voice channel {} ({}) in guild {} ({}): {source}",
        voice_channel_info.channel_name,
        voice_channel_info.channel_id,
        guild_info.guild_name,
        guild_info.guild_id
    )]
    #[diagnostic(help("Contact @solemnattic for assistance"))]
    FailedTrackPause {
        source: songbird::error::ControlError,
        track_uuid: uuid::Uuid,
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
    },

    #[error(
        "Failed to resume the track with uuid {track_uuid} in voice channel {} ({}) in guild {} ({}): {source}",
        voice_channel_info.channel_name,
        voice_channel_info.channel_id,
        guild_info.guild_name,
        guild_info.guild_id
    )]
    #[diagnostic(help("Contact @solemnattic for assistance"))]
    FailedTrackResume {
        source: songbird::error::ControlError,
        track_uuid: uuid::Uuid,
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
    },

    #[error(
        "Failed to seek the track with uuid {track_uuid} in voice channel {} ({}) in guild {} ({}) to position {position}: {source}",
        voice_channel_info.channel_name,
        voice_channel_info.channel_id,
        guild_info.guild_name,
        guild_info.guild_id
    )]
    #[diagnostic(help("Contact @solemnattic for assistance"))]
    FailedTrackSeek {
        source: songbird::error::ControlError,
        track_uuid: uuid::Uuid,
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
        position: u64,
    },

    #[error(
        "Error accessing the index {index} in the queue for voice channel {} ({}) in guild {} ({})",
        voice_channel_info.channel_name,
        voice_channel_info.channel_id,
        guild_info.guild_name,
        guild_info.guild_id
    )]
    #[diagnostic(help(
        "Ayaya tried to access the non existent, so pick something that exists..."
    ))]
    QueueOutOfBounds {
        index: usize,
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
    },

    #[error(
        "Ayaya tried to yeet whatever she's playing in the voice channel {} ({}) guild {} ({}).",
        voice_channel_info.channel_name,
        voice_channel_info.channel_id,
        guild_info.guild_name,
        guild_info.guild_id
    )]
    #[diagnostic(help("Sorry, Ayaya can't delete what she is playing. Maybe pick another?"))]
    QueueDeleteNowPlaying {
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
    },

    #[error("Ayaya has no audio tracks to seek in channel {} ({}) in guild {} ({})", voice_channel_info.channel_name, voice_channel_info.channel_id, guild_info.guild_name, guild_info.guild_id)]
    #[diagnostic(help("How about playing a song and then telling Ayaya to seek forward?"))]
    NoTrackToSeek {
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
    },

    #[error("Ayaya has been waiting too long for an input...")]
    #[diagnostic(help("Ayaya waited too long for you... *baka*."))]
    SearchTimeout,
    #[error("Somehow Ayaya was given an empty source....")]
    #[diagnostic(help("Try adding the music again"))]
    EmptySource,
    #[error("Ayaya is unable to loop the current track {} times in voice channel {} ({}) in guild {} ({})", count.unwrap_or(0), voice_channel_info.channel_name, voice_channel_info.channel_id, guild_info.guild_name,guild_info.guild_id)]
    #[diagnostic(help("Error: {} ; contect the owners", source))]
    FailedTrackLoop {
        source: songbird::error::ControlError,
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
        count: Option<usize>,
    },
}

impl ErrorName for MusicCommandError {
    fn name(&self) -> String {
        let name = match self {
            MusicCommandError::BotVoiceNotJoined { .. } => "bot_voice_not_joined",
            MusicCommandError::UserVoiceNotJoined { .. } => "user_voice_not_joined",
            MusicCommandError::FailedJoinCall { .. } => "failed_join_call",
            MusicCommandError::FailedLeaveCall { .. } => "failed_leave_call",
            MusicCommandError::FailedUnmuteCall { .. } => "failed_unmute_call",
            MusicCommandError::FailedDeafenCall { .. } => "failed_deafen_call",
            MusicCommandError::FailedUndeafenCall { .. } => "failed_undeafen_call",
            MusicCommandError::CallDoesNotExist => "call_does_not_exist",
            MusicCommandError::YoutubeDlError { .. } => "youtube_dl_error",
            MusicCommandError::YoutubeDlEmptyPlaylist { .. } => "youtube_dl_empty_playlist",
            MusicCommandError::TrackMetadataNotFound { .. } => "track_metadata_not_found",
            MusicCommandError::TrackMetadataRetrieveFailed(_) => "track_metadata_retrieve_failed",
            MusicCommandError::TrackStateNotFound { .. } => "track_state_not_found",
            MusicCommandError::FailedTrackSkip { .. } => "failed_track_skip",
            MusicCommandError::FailedTrackPause { .. } => "failed_track_pause",
            MusicCommandError::FailedTrackResume { .. } => "failed_track_resume",
            MusicCommandError::FailedTrackSeek { .. } => "failed_track_seek",
            MusicCommandError::QueueOutOfBounds { .. } => "queue_out_of_bounds",
            MusicCommandError::QueueDeleteNowPlaying { .. } => "queue_delete_now_playing",
            MusicCommandError::NoTrackToSeek { .. } => "no_track_to_seek",
            MusicCommandError::SearchTimeout => "search_timeout",
            MusicCommandError::EmptySource => "empty_source",
            MusicCommandError::FailedTrackLoop { .. } => "failed_track_loop",
        };
        format!("music::{name}")
    }
}
