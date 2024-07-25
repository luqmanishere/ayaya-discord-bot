use miette::Diagnostic;
use poise::serenity_prelude as serenity;
use thiserror::Error;

// TODO: include names of guilds and channels
#[derive(Error, Diagnostic, Debug)]
pub enum MusicCommandError {
    #[error("Ayaya has not joined any voice channels in the guild {voice_guild_id}.")]
    #[diagnostic(help("Try joining Ayaya to a voice channel with the join command."))]
    BotVoiceNotJoined { voice_guild_id: serenity::GuildId },
    #[error(
        "Ayaya can't find user {} ({}) in any voice channel in the guild {voice_guild_name} ({voice_guild_id})", user.name, user.id
    )]
    #[diagnostic(help("Try joining a voice channel before running the command."))]
    UserVoiceNotJoined {
        user: serenity::User,
        voice_guild_id: serenity::GuildId,
        voice_guild_name: String,
    },
    #[error(
        "Failed to join voice channel {voice_guild_name} ({voice_channel_id}) in guild {voice_guild_name} ({voice_guild_id}) due to {source}"
    )]
    #[diagnostic(help("Contact @solemnattic for assistance"))]
    FailedJoinCall {
        source: songbird::error::JoinError,
        voice_guild_id: serenity::GuildId,
        voice_guild_name: String,
        voice_channel_id: serenity::ChannelId,
        voice_channel_name: String,
    },
    #[error(
        "Failed to unmute voice channel {voice_channel_id} in guild {voice_guild_id} due to {source}"
    )]
    #[diagnostic(help("Contact @solemnattic for assistance"))]
    FailedUnmuteCall {
        source: songbird::error::JoinError,
        voice_guild_id: serenity::GuildId,
        voice_channel_id: serenity::ChannelId,
    },
    #[error(
        "Failed to deafen Ayaya in voice channel {voice_channel_id} in guild {voice_guild_id} due to {source}"
    )]
    #[diagnostic(help("Contact @solemnattic for assistance"))]
    FailedDeafenCall {
        source: songbird::error::JoinError,
        voice_guild_id: serenity::GuildId,
        voice_channel_id: serenity::ChannelId,
    },
    #[error(
        "Failed to undeafen Ayaya in voice channel {voice_channel_id} in guild {voice_guild_id} due to {source}"
    )]
    #[diagnostic(help("Contact @solemnattic for assistance"))]
    FailedUndeafenCall {
        source: songbird::error::JoinError,
        voice_guild_id: serenity::GuildId,
        voice_channel_id: serenity::ChannelId,
    },
    #[error("Ayaya can't find the bond between her and this guild ")]
    CallDoesNotExist,
    #[error("An error occured with youtube-dl: {0}")]
    #[diagnostic(help("Ayaya thinks youtube is being stupid tonight."))]
    YoutubeDlError(#[from] youtube_dl::Error),
    #[error("Empty playlist returned, Ayaya has nothing to play")]
    #[diagnostic(help("Ayaya thinks youtube is being stupid tonight."))]
    YoutubeDlEmptyPlaylist,
    #[error("Ayaya is unable to find the track metadata for uuid: {track_uuid}")]
    #[diagnostic(help(
        "Ayaya forgot something important, contact her owner @solemnattic to find out what."
    ))]
    TrackMetadataNotFound { track_uuid: uuid::Uuid },
    #[error("Failed to retrieve metadata for the track: {0}")]
    #[diagnostic(help("Ayaya thinks youtube is being stupid tonight."))]
    TrackMetadataRetrieveFailed(#[from] songbird::input::AudioStreamError),
    #[error("Ayaya is unable to find the track state for uuid: {track_uuid}")]
    #[diagnostic(help(
        "Ayaya forgot something important, contact her owner @solemnattic to find out what."
    ))]
    TrackStateNotFound {
        source: songbird::error::ControlError,
        track_uuid: uuid::Uuid,
    },
    #[error("Failed to skip the track with uuid {track_uuid} in guild {voice_guild_id}: {source}")]
    #[diagnostic(help("Contact @solemnattic for assistance"))]
    FailedTrackSkip {
        source: songbird::error::ControlError,
        track_uuid: uuid::Uuid,
        voice_guild_id: serenity::GuildId,
    },
    #[error(
        "Failed to pause the track with uuid {track_uuid} in guild {voice_guild_id}: {source}"
    )]
    #[diagnostic(help("Contact @solemnattic for assistance"))]
    FailedTrackPause {
        source: songbird::error::ControlError,
        track_uuid: uuid::Uuid,
        voice_guild_id: serenity::GuildId,
    },
    #[error(
        "Failed to resume the track with uuid {track_uuid} in guild {voice_guild_id}: {source}"
    )]
    #[diagnostic(help("Contact @solemnattic for assistance"))]
    FailedTrackResume {
        source: songbird::error::ControlError,
        track_uuid: uuid::Uuid,
        voice_guild_id: serenity::GuildId,
    },
    #[error(
        "Failed to seek the track with uuid {track_uuid} in guild {voice_guild_id} to position {position}: {source}"
    )]
    #[diagnostic(help("Contact @solemnattic for assistance"))]
    FailedTrackSeek {
        source: songbird::error::ControlError,
        track_uuid: uuid::Uuid,
        voice_guild_id: serenity::GuildId,
        position: u64,
    },
    #[error("Error accessing the index {index} in the queue for guild {voice_guild_id}")]
    #[diagnostic(help("Ayaya tried to access the non existent."))]
    QueueOutOfBounds {
        index: usize,
        voice_guild_id: serenity::GuildId,
    },
    #[error("Ayaya has been waiting too long for an input...")]
    #[diagnostic(help("Ayaya waited too long for you... *baka*."))]
    SearchTimeout,
}
