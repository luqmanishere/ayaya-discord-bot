//! Command reserved for admins or specific users
use poise::serenity_prelude as serenity;

use crate::{
    utils::{autocomplete_command_names, GuildInfo},
    CommandResult, Commands, Context,
};

pub fn admin_commands() -> Commands {
    vec![
        restrict_command_role(),
        restrict_category_role(),
        allow_user_command(),
        list_command_restrictions(),
    ]
}

/// Restrict commands to a certain role.
///
/// Arguments are a command name and and the role.
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    rename = "setrcr",
    category = "Admin Commands"
)]
pub async fn restrict_command_role(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_command_names"] command: String,
    role: serenity::Role,
) -> CommandResult {
    ctx.defer().await?;
    let mut data_manager = ctx.data().data_manager.clone();
    let guild_id = GuildInfo::guild_id_or_0(ctx);

    let model = data_manager
        .permissions_mut()
        .new_command_role_restriction(guild_id, &role, &command);

    match model.await {
        Ok(res) => {
            ctx.reply(format!(
                "Command restriction added for role `{}` & command `{}`.",
                role.name, res.command
            ))
            .await?;
        }
        Err(e) => {
            ctx.reply(format!(
                "Error inserting restriction for role `{}` and command `{}` into database, {}",
                role.name, command, "Maybe it already exists?"
            ))
            .await?;
            return Err(e.into());
        }
    }

    Ok(())
}

/// Restrict command categories to a role
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    rename = "setrcatr",
    category = "Admin Commands"
)]
pub async fn restrict_category_role(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_command_categories"] category: String,
    role: serenity::Role,
) -> CommandResult {
    ctx.defer().await?;
    let mut data_manager = ctx.data().data_manager.clone();
    let guild_id = GuildInfo::guild_id_or_0(ctx);

    let model = data_manager
        .permissions_mut()
        .new_category_role_restriction(guild_id, &role, &category);

    match model.await {
        Ok(res) => {
            ctx.reply(format!(
                "Category restriction added for role `{}` & category `{}`.",
                role.name, res.category
            ))
            .await?;
        }
        Err(e) => {
            // TODO: proper error for this
            ctx.reply(format!(
                "Error inserting restriction for role `{}` and category `{}` into database",
                role.name, category
            ))
            .await?;
            return Err(e.into());
        }
    }

    Ok(())
}

/// Allow a user to use a command regardless of role restrictions
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    rename = "setausercom",
    category = "Admin Commands"
)]
pub async fn allow_user_command(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_command_names"] command: String,
    user: serenity::User,
) -> CommandResult {
    ctx.defer().await?;
    let mut data_manager = ctx.data().data_manager.clone();
    let guild_id = GuildInfo::guild_id_or_0(ctx);

    let model =
        data_manager
            .permissions_mut()
            .new_command_user_allowed(guild_id, user.id.get(), &command);

    match model.await {
        Ok(res) => {
            ctx.reply(format!(
                "User allowance added for user `{}` & command `{}`.",
                user.name, res.command
            ))
            .await?;
        }
        Err(e) => {
            // TODO: proper error for this
            ctx.reply(format!(
                "Error inserting allowance for user `{}` and command `{}` into database",
                user.name, command
            ))
            .await?;
            return Err(e.into());
        }
    }

    Ok(())
}

#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    rename = "lcr",
    category = "Admin Commands"
)]
pub async fn list_command_restrictions(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_command_names"] command: String,
) -> CommandResult {
    ctx.defer().await?;
    let mut data_manager = ctx.data().data_manager.clone();
    let guild_id = GuildInfo::guild_id_or_0(ctx);
    // TODO: properly store command categories
    let command_category =
        if let Some(Some(category)) = ctx.data().command_categories_map.get(&command) {
            category.clone()
        } else {
            "Uncategorized".to_string()
        };

    let allowed_users = data_manager
        .permissions_mut()
        .findall_user_allowed(guild_id, &command)
        .await?;

    let required_roles_command = data_manager
        .permissions_mut()
        .find_command_roles_allowed(guild_id, &command)
        .await?;

    let required_roles_category = data_manager
        .permissions_mut()
        .find_category_roles_allowed(guild_id, &command_category)
        .await?;

    let mut message = serenity::MessageBuilder::default();
    message.push_line(format!("# Restrictions info for command: {}", &command));

    message.push_line("### Allowed Users");
    for (i, model) in allowed_users.iter().enumerate() {
        let user = serenity::UserId::new(model.user_id).to_user(ctx).await?;
        message.push_line(format!("{}. {}", i + 1, user.name));
    }
    message.push_line("### Command Roles");
    let roles = serenity::GuildId::new(guild_id).roles(ctx).await?;
    for (i, model) in required_roles_command.iter().enumerate() {
        let role = if let Some(role) = roles.get(&serenity::RoleId::new(model.role_id)) {
            role.name.clone()
        } else {
            "Unknown Role".to_string()
        };
        message.push_line(format!("{}. {}", i + 1, role));
    }
    message.push_line("### Category Roles");
    for (i, model) in required_roles_category.iter().enumerate() {
        let role = if let Some(role) = roles.get(&serenity::RoleId::new(model.role_id)) {
            role.name.clone()
        } else {
            "Unknown Role".to_string()
        };
        message.push_line(format!("{}. {}", i + 1, role));
    }

    let embed = serenity::CreateEmbed::default().description(message.to_string());
    ctx.send(poise::CreateReply::default().embed(embed).reply(true))
        .await?;

    Ok(())
}

async fn autocomplete_command_categories(ctx: Context<'_>, partial: &'_ str) -> Vec<String> {
    let partial = partial.to_lowercase();
    let command_categories = &ctx.data().command_categories;
    command_categories
        .iter()
        .filter(|s| s.contains(&partial))
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
}
