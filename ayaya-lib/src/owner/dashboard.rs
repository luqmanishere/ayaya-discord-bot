//! Dashboard management commands for owner and allowlisted users

use poise::serenity_prelude as serenity;
use poise::serenity_prelude::Mentionable;
use snafu::ResultExt;

use crate::{
    CommandResult, Context,
    error::{BotError, DataManagerSnafu, GeneralSerenitySnafu},
};

/// Check if user is bot owner or in dashboard allowlist
async fn check_dashboard_access(ctx: Context<'_>) -> Result<bool, BotError> {
    if ctx.framework().options().owners.contains(&ctx.author().id) {
        return Ok(true);
    }

    let user_id = ctx.author().id.get() as i64;
    ctx.data()
        .data_manager
        .is_allowlisted(user_id)
        .await
        .context(DataManagerSnafu)
}

/// Dashboard management commands
#[poise::command(
    slash_command,
    subcommands(
        "add_user",
        "remove_user",
        "list_users",
        "list_all_tokens",
        "revoke_token",
        "create_token",
        "list_tokens",
        "revoke_my_token"
    ),
    category = "Dashboard"
)]
pub async fn dashboard(_ctx: Context<'_>) -> CommandResult {
    Ok(())
}

/// Add a user to the dashboard allowlist (Owner only)
#[poise::command(slash_command, owners_only, ephemeral)]
pub async fn add_user(
    ctx: Context<'_>,
    #[description = "User to add to allowlist"] user: serenity::User,
    #[description = "Notes about this user"] notes: Option<String>,
) -> CommandResult {
    ctx.defer_ephemeral().await.context(GeneralSerenitySnafu)?;

    let user_id = user.id.get() as i64;
    let added_by = ctx.author().id.get() as i64;

    match ctx
        .data()
        .data_manager
        .add_to_allowlist(user_id, added_by, notes.clone())
        .await
    {
        Ok(true) => {
            let msg = if let Some(n) = notes {
                format!(
                    "Added {} to dashboard allowlist.\nNotes: {}",
                    user.mention(),
                    n
                )
            } else {
                format!("Added {} to dashboard allowlist.", user.mention())
            };
            ctx.reply(msg).await.context(GeneralSerenitySnafu)?;
        }
        Ok(false) => {
            ctx.reply("Failed to add user (may already be in allowlist)")
                .await
                .context(GeneralSerenitySnafu)?;
        }
        Err(e) => {
            ctx.reply(format!("Error: {}", e))
                .await
                .context(GeneralSerenitySnafu)?;
        }
    }

    Ok(())
}

/// Remove a user from the dashboard allowlist (Owner only)
#[poise::command(slash_command, owners_only, ephemeral)]
pub async fn remove_user(
    ctx: Context<'_>,
    #[description = "User to remove from allowlist"] user: serenity::User,
) -> CommandResult {
    ctx.defer_ephemeral().await.context(GeneralSerenitySnafu)?;

    let user_id = user.id.get() as i64;

    match ctx.data().data_manager.remove_from_allowlist(user_id).await {
        Ok(true) => {
            ctx.reply(format!(
                "Removed {} from dashboard allowlist.\nAll their tokens have been revoked.",
                user.mention()
            ))
            .await
            .context(GeneralSerenitySnafu)?;
        }
        Ok(false) => {
            ctx.reply("User was not in the allowlist")
                .await
                .context(GeneralSerenitySnafu)?;
        }
        Err(e) => {
            ctx.reply(format!("Error: {}", e))
                .await
                .context(GeneralSerenitySnafu)?;
        }
    }

    Ok(())
}

/// List all users in the dashboard allowlist (Owner only)
#[poise::command(slash_command, owners_only, ephemeral)]
pub async fn list_users(ctx: Context<'_>) -> CommandResult {
    ctx.defer_ephemeral().await.context(GeneralSerenitySnafu)?;

    let allowlist = ctx
        .data()
        .data_manager
        .list_allowlist()
        .await
        .context(DataManagerSnafu)?;

    if allowlist.is_empty() {
        ctx.reply("No users in dashboard allowlist.")
            .await
            .context(GeneralSerenitySnafu)?;
        return Ok(());
    }

    let mut message = String::from("**Dashboard Allowlist:**\n\n");
    for entry in allowlist {
        let user_mention = serenity::UserId::new(entry.user_id as u64).mention();
        let added_by_mention = serenity::UserId::new(entry.added_by as u64).mention();
        message.push_str(&format!(
            "• {} (added by {} on <t:{}:f>)\n",
            user_mention,
            added_by_mention,
            entry.added_at.unix_timestamp()
        ));
        if let Some(notes) = entry.notes {
            message.push_str(&format!("  Notes: {}\n", notes));
        }
    }

    ctx.reply(message).await.context(GeneralSerenitySnafu)?;
    Ok(())
}

/// List all dashboard tokens (Owner only)
#[poise::command(slash_command, owners_only, ephemeral)]
pub async fn list_all_tokens(ctx: Context<'_>) -> CommandResult {
    ctx.defer_ephemeral().await.context(GeneralSerenitySnafu)?;

    let tokens = ctx
        .data()
        .data_manager
        .list_all_tokens()
        .await
        .context(DataManagerSnafu)?;

    if tokens.is_empty() {
        ctx.reply("No dashboard tokens exist.")
            .await
            .context(GeneralSerenitySnafu)?;
        return Ok(());
    }

    let mut message = String::from("**All Dashboard Tokens:**\n\n");
    for token in tokens {
        let user_mention = serenity::UserId::new(token.user_id as u64).mention();
        let status = if token.active {
            "✅ Active"
        } else {
            "❌ Revoked"
        };
        message.push_str(&format!(
            "• {} - `{}` ({})\n  User: {}\n  Created: <t:{}:f>\n",
            token.description,
            &token.token_id.to_string()[..8],
            status,
            user_mention,
            token.created_at.unix_timestamp()
        ));
        if let Some(last_used) = token.last_used_at {
            message.push_str(&format!(
                "  Last used: <t:{}:R>\n",
                last_used.unix_timestamp()
            ));
        }
    }

    ctx.reply(message).await.context(GeneralSerenitySnafu)?;
    Ok(())
}

/// Revoke any dashboard token by ID (Owner only)
#[poise::command(slash_command, owners_only, ephemeral)]
pub async fn revoke_token(
    ctx: Context<'_>,
    #[description = "Token ID (first 8 characters)"] token_id_prefix: String,
) -> CommandResult {
    ctx.defer_ephemeral().await.context(GeneralSerenitySnafu)?;

    let all_tokens = ctx
        .data()
        .data_manager
        .list_all_tokens()
        .await
        .context(DataManagerSnafu)?;

    let matching_token = all_tokens
        .iter()
        .find(|t| t.token_id.to_string().starts_with(&token_id_prefix));

    match matching_token {
        Some(token) => {
            ctx.data()
                .data_manager
                .revoke_token(token.token_id)
                .await
                .context(DataManagerSnafu)?;

            let user_mention = serenity::UserId::new(token.user_id as u64).mention();
            ctx.reply(format!(
                "Revoked token `{}` belonging to {}",
                token.description, user_mention
            ))
            .await
            .context(GeneralSerenitySnafu)?;
        }
        None => {
            ctx.reply("No token found with that ID prefix")
                .await
                .context(GeneralSerenitySnafu)?;
        }
    }

    Ok(())
}

// ==================== Allowlisted User Subcommands ====================

/// Create a new dashboard token (Allowlisted users)
#[poise::command(slash_command, ephemeral)]
pub async fn create_token(
    ctx: Context<'_>,
    #[description = "Description for this token (e.g., 'My Laptop')"] description: String,
) -> CommandResult {
    ctx.defer_ephemeral().await.context(GeneralSerenitySnafu)?;

    if !check_dashboard_access(ctx).await? {
        ctx.reply("You don't have access to the dashboard.")
            .await
            .context(GeneralSerenitySnafu)?;
        return Ok(());
    }

    let user_id = ctx.author().id.get() as i64;

    match ctx
        .data()
        .data_manager
        .create_dashboard_token(user_id, description.clone())
        .await
    {
        Ok(token) => {
            ctx.reply(format!(
                "**Dashboard Token Created**\n\n🔑 Token: `{}`\n\n⚠️ **Save this token immediately!** It will not be shown again.\n\nDescription: {}\n\nUse this token in the dashboard's login page.",
                token, description
            ))
            .await
            .context(GeneralSerenitySnafu)?;
        }
        Err(e) => {
            ctx.reply(format!("Error creating token: {}", e))
                .await
                .context(GeneralSerenitySnafu)?;
        }
    }

    Ok(())
}

/// List your dashboard tokens (Allowlisted users)
#[poise::command(slash_command, ephemeral)]
pub async fn list_tokens(ctx: Context<'_>) -> CommandResult {
    ctx.defer_ephemeral().await.context(GeneralSerenitySnafu)?;

    if !check_dashboard_access(ctx).await? {
        ctx.reply("You don't have access to the dashboard.")
            .await
            .context(GeneralSerenitySnafu)?;
        return Ok(());
    }

    let user_id = ctx.author().id.get() as i64;
    let tokens = ctx
        .data()
        .data_manager
        .list_user_tokens(user_id)
        .await
        .context(DataManagerSnafu)?;

    if tokens.is_empty() {
        ctx.reply("You have no dashboard tokens.\nUse `/dashboard create-token` to create one.")
            .await
            .context(GeneralSerenitySnafu)?;
        return Ok(());
    }

    let mut message = String::from("**Your Dashboard Tokens:**\n\n");
    for token in tokens {
        let status = if token.active {
            "✅ Active"
        } else {
            "❌ Revoked"
        };
        message.push_str(&format!(
            "• {} - `{}` ({})\n  Created: <t:{}:f>\n",
            token.description,
            &token.token_id.to_string()[..8],
            status,
            token.created_at.unix_timestamp()
        ));
        if let Some(last_used) = token.last_used_at {
            message.push_str(&format!(
                "  Last used: <t:{}:R>\n",
                last_used.unix_timestamp()
            ));
        }
    }

    ctx.reply(message).await.context(GeneralSerenitySnafu)?;
    Ok(())
}

/// Revoke one of your dashboard tokens (Allowlisted users)
#[poise::command(slash_command, ephemeral)]
pub async fn revoke_my_token(
    ctx: Context<'_>,
    #[description = "Token ID (first 8 characters)"] token_id_prefix: String,
) -> CommandResult {
    ctx.defer_ephemeral().await.context(GeneralSerenitySnafu)?;

    if !check_dashboard_access(ctx).await? {
        ctx.reply("You don't have access to the dashboard.")
            .await
            .context(GeneralSerenitySnafu)?;
        return Ok(());
    }

    let user_id = ctx.author().id.get() as i64;
    let user_tokens = ctx
        .data()
        .data_manager
        .list_user_tokens(user_id)
        .await
        .context(DataManagerSnafu)?;

    let matching_token = user_tokens
        .iter()
        .find(|t| t.token_id.to_string().starts_with(&token_id_prefix));

    match matching_token {
        Some(token) => {
            ctx.data()
                .data_manager
                .revoke_token(token.token_id)
                .await
                .context(DataManagerSnafu)?;

            ctx.reply(format!("Revoked token: `{}`", token.description))
                .await
                .context(GeneralSerenitySnafu)?;
        }
        None => {
            ctx.reply("No token found with that ID prefix, or it doesn't belong to you")
                .await
                .context(GeneralSerenitySnafu)?;
        }
    }

    Ok(())
}
