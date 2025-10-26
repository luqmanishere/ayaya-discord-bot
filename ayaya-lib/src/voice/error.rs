use poise::serenity_prelude::{self as serenity};
use snafu::Snafu;

use crate::{
    error::{ErrorName, UserFriendlyError},
    utils::{ChannelInfo, GuildInfo},
    voice::commands::soundboard::error::SoundboardError,
};

#[derive(Debug, Snafu)]
pub enum MusicCommandError {
    #[snafu(display("Ayaya has not joined any voice channels in the guild {} ({})", guild_info.guild_id, guild_info.guild_name))]
    BotVoiceNotJoined { guild_info: GuildInfo },

    #[snafu(display("Ayaya can't find user {} ({}) in any voice channel in the guild {} ({})", user.name, user.id, guild_info.guild_name, guild_info.guild_id))]
    UserVoiceNotJoined {
        user: serenity::User,
        guild_info: GuildInfo,
    },

    #[snafu(display(
        "Failed to join voice channel {} ({}) in guild {} ({}) with error {source}",
        voice_channel_info.channel_name,
        voice_channel_info.channel_id,
        guild_info.guild_name,
        guild_info.guild_id
    ))]
    FailedJoinCall {
        source: songbird::error::JoinError,
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
    },

    #[snafu(display(
        "Failed to leave voice channel {} ({}) in guild {} ({}) with error: {source}",
        voice_channel_info.channel_name,
        voice_channel_info.channel_id,
        guild_info.guild_name,
        guild_info.guild_id
    ))]
    FailedLeaveCall {
        source: songbird::error::JoinError,
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
    },

    #[snafu(display("Failed to unmute voice channel {} ({}) in guild {} ({}) due to {source}", voice_channel_info.channel_name, voice_channel_info.channel_id, guild_info.guild_name, guild_info.guild_id))]
    FailedUnmuteCall {
        source: songbird::error::JoinError,
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
    },

    #[snafu(display(
        "Failed to deafen Ayaya in voice channel {} ({}) in guild {} ({}) with error: {source}",
        voice_channel_info.channel_name,
        voice_channel_info.channel_id,
        guild_info.guild_name,
        guild_info.guild_id
    ))]
    FailedDeafenCall {
        source: songbird::error::JoinError,
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
    },

    #[snafu(display(
        "Failed to undeafen Ayaya in voice channel {} ({}) in guild {} ({}) with error: {source}",
        voice_channel_info.channel_name,
        voice_channel_info.channel_id,
        guild_info.guild_name,
        guild_info.guild_id
    ))]
    FailedUndeafenCall {
        source: songbird::error::JoinError,
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
    },

    #[snafu(display(
        "Ayaya can't find the bond between her and this guild. Time for a reboot perhaps?"
    ))]
    CallDoesNotExist,

    #[snafu(display(
        "An error occured with youtube-dl while processing query \"{args}\" : {source}"
    ))]
    YoutubeDlError {
        source: youtube_dl::Error,
        args: String,
    },

    #[snafu(display("Empty playlist returned from \"{args}\", Ayaya has nothing to play"))]
    YoutubeDlEmptyPlaylist { args: String },

    #[snafu(display("Ayaya is unable to find the track metadata for uuid: {track_uuid}"))]
    TrackMetadataNotFound { track_uuid: uuid::Uuid },

    #[snafu(display("Failed to retrieve metadata for the track: {source}"))]
    TrackMetadataRetrieveFailed {
        source: songbird::input::AudioStreamError,
    },

    #[snafu(display("Ayaya is unable to find the track state for uuid: {track_uuid}"))]
    TrackStateNotFound {
        source: songbird::error::ControlError,
        track_uuid: uuid::Uuid,
    },

    #[snafu(display(
        "Failed to skip the track with uuid {track_uuid} in voice channel {} ({}) in guild {} ({}): {source}",
        voice_channel_info.channel_name,
        voice_channel_info.channel_id,
        guild_info.guild_name,
        guild_info.guild_id
    ))]
    FailedTrackSkip {
        source: songbird::error::ControlError,
        track_uuid: uuid::Uuid,
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
    },

    #[snafu(display(
        "Failed to pause the track with uuid {track_uuid} in voice channel {} ({}) in guild {} ({}): {source}",
        voice_channel_info.channel_name,
        voice_channel_info.channel_id,
        guild_info.guild_name,
        guild_info.guild_id
    ))]
    FailedTrackPause {
        source: songbird::error::ControlError,
        track_uuid: uuid::Uuid,
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
    },

    #[snafu(display(
        "Failed to resume the track with uuid {track_uuid} in voice channel {} ({}) in guild {} ({}): {source}",
        voice_channel_info.channel_name,
        voice_channel_info.channel_id,
        guild_info.guild_name,
        guild_info.guild_id
    ))]
    FailedTrackResume {
        source: songbird::error::ControlError,
        track_uuid: uuid::Uuid,
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
    },

    #[snafu(display(
        "Failed to seek the track with uuid {track_uuid} in voice channel {} ({}) in guild {} ({}) to position {position}: {source}",
        voice_channel_info.channel_name,
        voice_channel_info.channel_id,
        guild_info.guild_name,
        guild_info.guild_id
    ))]
    FailedTrackSeek {
        source: songbird::error::ControlError,
        track_uuid: uuid::Uuid,
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
        position: u64,
    },

    #[snafu(display(
        "Backwards seeking is not supported, requested position {requested_position}, current position {current_position}"
    ))]
    NoSeekBackwards {
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
        requested_position: u64,
        current_position: u64,
    },

    #[snafu(display(
        "Out of bounds, requested position {requested_position}, max_position {max_position}"
    ))]
    SeekOutOfBounds {
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
        requested_position: u64,
        max_position: u64,
    },

    #[snafu(display("This track has no reported duration, hence seeking is unsafe."))]
    NoDurationNoSeek {
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
        track_uuid: uuid::Uuid,
    },

    #[snafu(display(
        "Error accessing the index {index} in the queue for voice channel {} ({}) in guild {} ({})",
        voice_channel_info.channel_name,
        voice_channel_info.channel_id,
        guild_info.guild_name,
        guild_info.guild_id
    ))]
    QueueOutOfBounds {
        index: usize,
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
    },

    #[snafu(display(
        "Ayaya tried to yeet whatever she's playing in the voice channel {} ({}) guild {} ({}).",
        voice_channel_info.channel_name,
        voice_channel_info.channel_id,
        guild_info.guild_name,
        guild_info.guild_id
    ))]
    QueueDeleteNowPlaying {
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
    },

    #[snafu(display("Ayaya has no audio tracks to seek in channel {} ({}) in guild {} ({})", voice_channel_info.channel_name, voice_channel_info.channel_id, guild_info.guild_name, guild_info.guild_id))]
    NoTrackToSeek {
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
    },

    #[snafu(display("Ayaya has been waiting too long for an input..."))]
    SearchTimeout,

    #[snafu(display("Somehow Ayaya was given an empty source...."))]
    EmptySource,

    #[snafu(display("Ayaya is unable to loop the current track {} times in voice channel {} ({}) in guild {} ({})", count.unwrap_or(0), voice_channel_info.channel_name, voice_channel_info.channel_id, guild_info.guild_name,guild_info.guild_id))]
    FailedTrackLoop {
        source: songbird::error::ControlError,
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
        count: Option<usize>,
    },

    #[snafu(display("Cannot mutate position 1 in the queue."))]
    QueueMoveNoPos1 {
        guild_info: GuildInfo,
        voice_channel_info: ChannelInfo,
    },

    #[snafu(transparent)]
    // #[diagnostic(transparent)]
    SoundboardError { source: SoundboardError },

    #[snafu(display("Failed to add queue event: {error}"))]
    FailedAddEvent {
        error: songbird::error::ControlError,
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
            MusicCommandError::TrackMetadataRetrieveFailed { .. } => {
                "track_metadata_retrieve_failed"
            }
            MusicCommandError::TrackStateNotFound { .. } => "track_state_not_found",
            MusicCommandError::FailedTrackSkip { .. } => "failed_track_skip",
            MusicCommandError::FailedTrackPause { .. } => "failed_track_pause",
            MusicCommandError::FailedTrackResume { .. } => "failed_track_resume",
            MusicCommandError::FailedTrackSeek { .. } => "failed_track_seek",
            MusicCommandError::NoSeekBackwards { .. } => "no_backwards_seek",
            MusicCommandError::SeekOutOfBounds { .. } => "seek_out_of_bounds",
            MusicCommandError::NoDurationNoSeek { .. } => "no_duration_no_seek",
            MusicCommandError::QueueOutOfBounds { .. } => "queue_out_of_bounds",
            MusicCommandError::QueueDeleteNowPlaying { .. } => "queue_delete_now_playing",
            MusicCommandError::NoTrackToSeek { .. } => "no_track_to_seek",
            MusicCommandError::SearchTimeout => "search_timeout",
            MusicCommandError::EmptySource => "empty_source",
            MusicCommandError::FailedTrackLoop { .. } => "failed_track_loop",
            MusicCommandError::QueueMoveNoPos1 { .. } => "queue_move_no_pos1",
            MusicCommandError::SoundboardError { source } => &ErrorName::name(source),
            MusicCommandError::FailedAddEvent { .. } => "failed_add_event",
        };
        format!("music::{name}")
    }
}

impl UserFriendlyError for MusicCommandError {
    fn help_text(&self) -> &str {
        const DEFAULT: &str = "Contact @solemnattic for assistance";
        match self {
            Self::BotVoiceNotJoined { .. } => {
                "Try joining Ayaya to a voice channel with the join command."
            }
            Self::UserVoiceNotJoined { .. } => {
                "Try joining a voice channel before running the command."
            }
            Self::YoutubeDlError { .. } | Self::YoutubeDlEmptyPlaylist { .. } => {
                "Ayaya thinks youtube is being stupid tonight. Just try again"
            }
            Self::TrackMetadataNotFound { .. } | Self::TrackStateNotFound { .. } => {
                "Ayaya forgot something important, contact her owner @solemnattic to find out what."
            }
            Self::NoSeekBackwards { .. } => {
                "Just don't seek backwards? Look at the autocomplete lol."
            }
            Self::SeekOutOfBounds { .. } => {
                "You can seek up to -5 seconds before end. Refer to the autocomplete for max."
            }
            Self::NoDurationNoSeek { .. } => "Go complain to @solemnattic",
            Self::QueueOutOfBounds { .. } => {
                "Ayaya tried to access the non existent, so pick something that exists..."
            }
            Self::QueueDeleteNowPlaying { .. } => {
                "Sorry, Ayaya can't delete what she is playing. Maybe pick another?"
            }
            Self::NoTrackToSeek { .. } => {
                "How about playing a song and then telling Ayaya to seek forward?"
            }
            Self::SearchTimeout => "Ayaya waited too long for you... *baka*.",
            Self::EmptySource => "Try adding the music again",
            Self::FailedTrackLoop { .. } => "Failed to loop the track",
            Self::QueueMoveNoPos1 { .. } => {
                "To move to the next song position, use position 2. Or leave the target empty."
            }
            Self::SoundboardError { source } => source.help_text(),
            _ => DEFAULT,
        }
    }

    fn category(&self) -> crate::error::ErrorCategory {
        match self {
            MusicCommandError::BotVoiceNotJoined { .. } => crate::error::ErrorCategory::UserMistake,
            MusicCommandError::UserVoiceNotJoined { .. } => {
                crate::error::ErrorCategory::UserMistake
            }
            MusicCommandError::QueueMoveNoPos1 { .. } => crate::error::ErrorCategory::UserMistake,
            _ => crate::error::ErrorCategory::BotIssue,
        }
    }
}
