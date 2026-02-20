use std::str::FromStr;

use crate::{
    Context,
    error::{BotError, GeneralSerenitySnafu},
    tracker::SupportedGames,
};
use ::serenity::all::CollectComponentInteractions;
use poise::serenity_prelude as serenity;
use snafu::{ResultExt, whatever};
use strum::VariantNames;

pub async fn game_selector_ui(
    ctx: Context<'_>,
    pre_interaction: Option<&serenity::ComponentInteraction>,
    edit: bool,
) -> Result<(SupportedGames, serenity::ComponentInteraction), BotError> {
    let user_id = ctx.author().id.get();
    let select_id = format!("{}_game_select", user_id);
    let options = SupportedGames::VARIANTS
        .iter()
        .map(|e| serenity::CreateSelectMenuOption::new(e.to_string(), e.to_string()))
        .collect::<Vec<_>>();
    let game_select = serenity::CreateSelectMenu::new(
        select_id,
        serenity::CreateSelectMenuKind::String {
            options: options.into(),
        },
    );
    let action_row = serenity::CreateActionRow::SelectMenu(game_select);
    let text_display = serenity::CreateTextDisplay::new("Select a game to view pull stats:");

    let top_container = serenity::CreateContainer::new(vec![
        serenity::CreateContainerComponent::TextDisplay(text_display),
        serenity::CreateContainerComponent::ActionRow(action_row),
    ])
    .accent_color(serenity::Color::MEIBE_PINK);

    let components = vec![serenity::CreateComponent::Container(top_container)];

    let message_id = if let Some(interaction) = pre_interaction {
        if edit {
            interaction
                .edit_response(
                    ctx.http(),
                    serenity::EditInteractionResponse::new()
                        .flags(serenity::MessageFlags::IS_COMPONENTS_V2)
                        .components(components),
                )
                .await
                .context(GeneralSerenitySnafu)?
                .id
        } else {
            interaction
                .create_followup(
                    ctx.http(),
                    serenity::CreateInteractionResponseFollowup::new()
                        .flags(serenity::MessageFlags::IS_COMPONENTS_V2)
                        .ephemeral(true)
                        .components(components),
                )
                .await
                .context(GeneralSerenitySnafu)?
                .id
        }
    } else {
        let reply = ctx
            .reply_builder(poise::CreateReply::default())
            .flags(serenity::MessageFlags::IS_COMPONENTS_V2)
            .ephemeral(true)
            .components(components);
        let h = ctx.send(reply).await.context(GeneralSerenitySnafu)?;
        h.message().await.context(GeneralSerenitySnafu)?.id
    };

    let interaction = match message_id
        .collect_component_interactions(ctx.serenity_context())
        .timeout(std::time::Duration::from_mins(3))
        .await
    {
        Some(i) => i,
        None => {
            ctx.reply("Timed out waiting for game selection. You need to select a game")
                .await
                .context(GeneralSerenitySnafu)?;
            whatever!("User is required to select atleast one game")
        }
    };

    let game = match interaction.data.kind {
        serenity::ComponentInteractionDataKind::StringSelect { ref values } => {
            let selected_game = &values[0];
            match SupportedGames::from_str(selected_game) {
                Ok(g) => g,
                Err(_) => {
                    ctx.reply("Invalid game selected.")
                        .await
                        .context(GeneralSerenitySnafu)?;
                    whatever!("User is required to select atleast one game")
                }
            }
        }
        _ => {
            ctx.reply("Invalid interaction data.")
                .await
                .context(GeneralSerenitySnafu)?;
            whatever!("User is required to select atleast one game")
        }
    };

    Ok((game, interaction))
}
