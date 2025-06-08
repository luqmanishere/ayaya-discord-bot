use poise::serenity_prelude::{self as serenity};
use serenity::{model::channel::Message, Result as SerenityResult};
use tracing::error;

use crate::{voice::error::MusicCommandError, BotError, Context};

/// Checks that a message successfully sent; if not, then logs why to stdout.
pub fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        error!("Error sending message: {:?}", why);
    }
}

pub trait OptionExt<String> {
    fn unwrap_or_unknown(self) -> String;
}

impl OptionExt<String> for Option<String> {
    fn unwrap_or_unknown(self) -> String {
        self.unwrap_or_else(|| "Unknown".to_string())
    }
}

pub fn get_guild_id(ctx: Context<'_>) -> Result<serenity::GuildId, BotError> {
    ctx.guild_id().ok_or(BotError::NoGuildId)
}

pub fn get_guild(ctx: Context<'_>) -> Result<serenity::Guild, BotError> {
    Ok(ctx.guild().ok_or(BotError::NoGuild)?.clone())
}

pub fn get_guild_name(ctx: Context<'_>) -> Result<String, BotError> {
    Ok(get_guild_id(ctx)?
        .name(ctx)
        .unwrap_or("Unknown Guild".to_string()))
}

pub fn songbird_channel_to_serenity_channel(
    songbird_voice_channel_id: songbird::id::ChannelId,
) -> serenity::ChannelId {
    let channel_id: u64 = songbird_voice_channel_id.0.into();
    serenity::ChannelId::from(channel_id)
}

#[derive(Debug, Clone, Default)]
pub struct GuildInfo {
    pub guild_name: String,
    pub guild_id: serenity::GuildId,
}

impl GuildInfo {
    pub fn from_ctx(ctx: Context<'_>) -> Result<Self, BotError> {
        let guild_id = get_guild_id(ctx)?;
        let guild_name = get_guild_name(ctx)?;
        Ok(Self {
            guild_name,
            guild_id,
        })
    }

    pub fn guild_id_or_0(ctx: Context<'_>) -> u64 {
        if let Ok(guild_id) = get_guild_id(ctx) {
            guild_id.get()
        } else {
            0
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChannelInfo {
    pub channel_name: String,
    pub channel_id: serenity::ChannelId,
    pub is_voice: bool,
}

impl ChannelInfo {
    pub async fn from_ctx(ctx: Context<'_>, is_voice: bool) -> Result<Self, BotError> {
        let channel_id = ctx.channel_id();
        let channel_name = channel_id.name(ctx).await?;
        Ok(Self {
            channel_name,
            channel_id,
            is_voice,
        })
    }

    pub async fn from_songbird_current_channel(
        ctx: Context<'_>,
        songbird_voice_channel: Option<songbird::id::ChannelId>,
        guild_info: &GuildInfo,
    ) -> Result<Self, BotError> {
        let channel_id = songbird_channel_to_serenity_channel(songbird_voice_channel.ok_or(
            MusicCommandError::BotVoiceNotJoined {
                guild_info: guild_info.clone(),
            },
        )?);
        let channel_name = channel_id.name(ctx).await?;
        Ok(Self {
            channel_name,
            channel_id,
            is_voice: true,
        })
    }

    pub async fn from_serenity_id(
        ctx: Context<'_>,
        channel_id: serenity::ChannelId,
        is_voice: bool,
    ) -> Result<Self, BotError> {
        let channel_name = channel_id.name(ctx).await?;
        Ok(Self {
            channel_name,
            channel_id,
            is_voice,
        })
    }
}

/// Autocomplete function for command names
pub async fn autocomplete_command_names(ctx: Context<'_>, partial: &str) -> Vec<String> {
    let partial = partial.to_lowercase();
    let command_names = &ctx.data().command_names;
    command_names
        .iter()
        .filter(|s| s.contains(&partial))
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
}

/// Check command to determine if a commmand is allowed for a user.
///
/// This checks for the following:
/// 1. Whether a user is explicitly allowed.
/// 2. Whether the user possesses a role that is allowed for the command.
/// 3. Whether the user possesses a role that is allowed for the command category.
///
/// If the command is restricted, and the user does not meet the above requirement, then the
/// command use is not allowed.
pub async fn check_command_allowed(ctx: Context<'_>) -> Result<bool, BotError> {
    let mut data_manager = ctx.data().data_manager.clone();
    let user_id = ctx.author().id.get();
    let guild_id = GuildInfo::guild_id_or_0(ctx);
    let command = ctx.command().name.clone();
    let command_category = ctx
        .command()
        .category
        .clone()
        .unwrap_or("Unknown".to_string());

    // TODO: cache

    // check first if user is allowed to use the command
    let user_allowed = data_manager
        .permissions_mut()
        .find_user_allowed(guild_id, user_id, &command)
        .await?;
    if let Some(_model) = user_allowed {
        return Ok(true);
    }

    // check for roles. if present, then iter, else check for catgory role
    let command_roles_allowed = data_manager
        .permissions_mut()
        .find_command_roles_allowed(guild_id, &command)
        .await?;
    if !command_roles_allowed.is_empty() {
        for role in command_roles_allowed {
            let role_id = role.role_id;
            if ctx.author().has_role(ctx, guild_id, role_id).await? {
                return Ok(true);
            }
        }
        // TODO: explanation
        ctx.reply(format!(
            "You are not allowed to use the command `{}` due not having the required roles.",
            &command
        ))
        .await?;
        #[expect(clippy::needless_return)]
        return Ok(false);
    } else {
        // check for category roles. if present, iter, else allow
        let category_roles_allowed = data_manager
            .permissions_mut()
            .find_category_roles_allowed(guild_id, &command_category)
            .await?;
        if !category_roles_allowed.is_empty() {
            for role in category_roles_allowed {
                let role_id = role.role_id;
                if ctx.author().has_role(ctx, guild_id, role_id).await? {
                    return Ok(true);
                }
            }
            // TODO: explanation
            ctx.reply(format!(
                "You are not allowed to use the command `{}` due not having the required roles.",
                &command
            ))
            .await?;
            return Ok(false);
        }

        // allow the command if not restricted to a role
        Ok(true)
    }
}
