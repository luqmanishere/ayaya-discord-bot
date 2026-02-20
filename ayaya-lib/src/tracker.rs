use ::serenity::all::CollectComponentInteractions;
use poise::serenity_prelude as serenity;
use snafu::ResultExt;

use crate::{
    CommandResult, Context,
    error::GeneralSerenitySnafu,
    tracker::{
        accounts::main_account_management_ui,
        game_selector::game_selector_ui,
        pulls::{pull_import_modal, pulls_data_ui},
    },
};

use ayaya_tracker::gacha_tracker::GameId;

pub mod accounts;
pub mod game_selector;
pub mod pulls;

// TODO: split this file up

#[poise::command(slash_command, aliases("t"))]
pub async fn tracker(ctx: Context<'_>) -> CommandResult {
    ctx.defer_ephemeral().await.context(GeneralSerenitySnafu)?;
    let user = ctx.author();

    let manage_accounts_button_id = format!("{}_manage_accounts_button", user.id);
    let pulls_button_id = format!("{}_pulls_button", user.id);
    let import_pulls_button_id = format!("{}_import_pulls_button", user.id);
    let stats_button_id = format!("{}_stats_button", user.id);

    let buttons = vec![
        serenity::CreateButton::new("manage_accounts")
            .custom_id(&manage_accounts_button_id)
            .label("Manage Accounts")
            .style(serenity::ButtonStyle::Primary),
        serenity::CreateButton::new("pulls")
            .custom_id(&pulls_button_id)
            .label("Pull Data")
            .style(serenity::ButtonStyle::Primary),
        serenity::CreateButton::new("import_pulls")
            .custom_id(&import_pulls_button_id)
            .label("Import Pulls")
            .style(serenity::ButtonStyle::Success),
        serenity::CreateButton::new("stats")
            .custom_id(&stats_button_id)
            .label("Global Stats")
            .style(serenity::ButtonStyle::Secondary),
    ];

    let first_action_row = serenity::CreateActionRow::Buttons(buttons.into());
    let first_display = serenity::CreateTextDisplay::new("Gacha Tracker Menu:");
    let first_container = serenity::CreateContainer::new(vec![
        serenity::CreateContainerComponent::TextDisplay(first_display),
        serenity::CreateContainerComponent::ActionRow(first_action_row),
    ])
    .accent_color(serenity::Colour::BLUE);
    let first_components = vec![serenity::CreateComponent::Container(first_container)];
    let message = poise::CreateReply::new()
        .components(first_components)
        .flags(serenity::MessageFlags::EPHEMERAL | serenity::MessageFlags::IS_COMPONENTS_V2);

    let reply1 = ctx.reply_builder(message);
    let reply1 = ctx.send(reply1).await.context(GeneralSerenitySnafu)?;
    let reply1 = reply1.message().await.context(GeneralSerenitySnafu)?;

    // TODO: make it so that this is reusable for some time
    if let Some(interaction) = reply1
        .id
        .collect_component_interactions(ctx.serenity_context())
        .timeout(std::time::Duration::from_mins(3))
        .await
    {
        interaction
            .create_response(ctx.http(), serenity::CreateInteractionResponse::Acknowledge)
            .await
            .context(GeneralSerenitySnafu)?;

        let custom_id = interaction.data.custom_id.as_str();

        // dispatch to func based on type
        if custom_id == manage_accounts_button_id {
            let (game, new_inter) = game_selector_ui(ctx, Some(&interaction), false).await?;
            new_inter
                .defer_ephemeral(ctx.http())
                .await
                .context(GeneralSerenitySnafu)?;
            tracing::debug!("selected_game: {}", game.to_string());
            main_account_management_ui(ctx, Some(&new_inter), true, game).await?;
        } else if custom_id == pulls_button_id {
            let (game, new_inter) = game_selector_ui(ctx, Some(&interaction), false).await?;
            tracing::debug!("selected_game: {}", game.to_string());
            pulls_data_ui(ctx, &new_inter, game).await?;
        } else if custom_id == import_pulls_button_id {
            let (game, new_inter) = game_selector_ui(ctx, Some(&interaction), false).await?;
            tracing::debug!("selected_game: {}", game.to_string());
            pull_import_modal(ctx, &new_inter, game).await?;
        } else if custom_id == stats_button_id {
            // TODO: impl
            interaction
                .create_followup(
                    ctx.http(),
                    serenity::CreateInteractionResponseFollowup::new()
                        .components(vec![serenity::CreateComponent::TextDisplay(
                            serenity::CreateTextDisplay::new("TODO - Global Stats"),
                        )])
                        .ephemeral(false)
                        .flags(serenity::MessageFlags::IS_COMPONENTS_V2),
                )
                .await
                .context(GeneralSerenitySnafu)?;
        } else {
            interaction
                .create_followup(
                    ctx.http(),
                    serenity::CreateInteractionResponseFollowup::new()
                        .components(vec![serenity::CreateComponent::TextDisplay(
                            serenity::CreateTextDisplay::new("How did you get here?"),
                        )])
                        .ephemeral(false)
                        .flags(serenity::MessageFlags::IS_COMPONENTS_V2),
                )
                .await
                .context(GeneralSerenitySnafu)?;
        }
    }

    Ok(())
}

#[derive(
    poise::ChoiceParameter, strum::EnumString, strum::Display, strum::VariantNames, PartialEq, Eq,
)]
pub enum SupportedGames {
    #[strum(to_string = "Wuthering Waves")]
    WutheringWaves,
    #[strum(to_string = "Arknights Endfield")]
    AkEnd,
}

fn parse_akend_ts(raw: &str) -> time::OffsetDateTime {
    let value = raw.parse::<i128>().expect("unix timestamp");
    if value > 1_000_000_000_000 {
        let nanos = value.checked_mul(1_000_000).expect("unix timestamp nanos");
        time::OffsetDateTime::from_unix_timestamp_nanos(nanos).expect("proper unix timestamp")
    } else {
        time::OffsetDateTime::from_unix_timestamp(value as i64).expect("proper unix timestamp")
    }
}

impl SupportedGames {
    fn game_id(self) -> GameId {
        match self {
            SupportedGames::WutheringWaves => GameId::WutheringWaves,
            SupportedGames::AkEnd => GameId::ArknightsEndfield,
        }
    }
}
