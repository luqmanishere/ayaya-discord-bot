use ::serenity::{
    all::CollectComponentInteractions, futures::StreamExt, small_fixed_array::FixedString,
};
use poise::serenity_prelude as serenity;
use snafu::{ResultExt, whatever};

use crate::{
    Context,
    error::{BotError, DataManagerSnafu, GeneralSerenitySnafu},
    tracker::SupportedGames,
};

pub async fn main_account_management_ui(
    ctx: Context<'_>,
    pre_interaction: Option<&serenity::ComponentInteraction>,
    edit: bool,
    game: SupportedGames,
) -> Result<(), BotError> {
    let user = ctx.author();
    let data = ctx.data().data_manager.clone();
    let title = format!("# Manage {}'s {} Accounts", user.display_name(), game);
    let subtitle = "## Current linked accounts:";
    let accounts = match game {
        SupportedGames::WutheringWaves => {
            let wuwa_tracker = data.wuwa_tracker();
            let linked_accounts = wuwa_tracker
                .get_wuwa_user_from_user_id(user.id.get())
                .await
                .context(DataManagerSnafu)?;
            if linked_accounts.is_empty() {
                "No linked accounts found.".to_string()
            } else {
                let mut account_list = String::new();
                for account in linked_accounts {
                    account_list.push_str(&format!("- Player ID: {}\n", account.wuwa_user_id));
                }
                account_list
            }
        }
        SupportedGames::AkEnd => {
            let akend_tracker = data.akend_tracker();
            let linked_accounts = akend_tracker
                .get_akend_users_by_user_id(user.id.get())
                .await
                .context(DataManagerSnafu)?;
            if linked_accounts.is_empty() {
                "No linked accounts found.".to_string()
            } else {
                let mut account_list = String::new();
                for account in linked_accounts {
                    account_list.push_str(&format!(
                        "- Akend User ID: {}, Description: {}\n",
                        account.ak_end_user_id, account.user_desc
                    ));
                }
                account_list
            }
        }
    };
    let content = format!("{}\n{}\n{}", title, subtitle, accounts);
    let main_text_display =
        serenity::CreateContainerComponent::TextDisplay(serenity::CreateTextDisplay::new(content));

    let main_container = serenity::CreateComponent::Container(
        serenity::CreateContainer::new(vec![main_text_display])
            .accent_color(serenity::Colour::BLUE),
    );

    let add_account_button = serenity::CreateButton::new("add_account").label("Add Account");
    let _modify_account_button =
        serenity::CreateButton::new("modify_account").label("Modify Account");
    let _delete_account_button =
        serenity::CreateButton::new("delete_account").label("Delete Account");
    let main_action_row = serenity::CreateComponent::ActionRow(serenity::CreateActionRow::Buttons(
        vec![
            add_account_button,
            // modify_account_button,
            // delete_account_button,
        ]
        .into(),
    ));

    let message_id = if let Some(interaction) = pre_interaction {
        if edit {
            interaction
                .edit_response(
                    ctx.http(),
                    serenity::EditInteractionResponse::new()
                        .flags(serenity::MessageFlags::IS_COMPONENTS_V2)
                        .components(vec![main_container, main_action_row]),
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
                        .components(vec![main_container, main_action_row]),
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
            .components(vec![main_container, main_action_row]);
        ctx.send(reply)
            .await
            .context(GeneralSerenitySnafu)?
            .message()
            .await
            .context(GeneralSerenitySnafu)?
            .id
    };

    if let Some(interaction) = message_id
        .collect_component_interactions(ctx.serenity_context())
        .timeout(std::time::Duration::from_mins(3))
        .await
    {
        // the functions here should handle their own responses
        match interaction.data.custom_id.as_str() {
            "add_account" => {
                add_account_modal_ui(ctx, &interaction, game).await?;
            }
            "modify_account" => {
                todo!()
            }
            "delete_account" => {
                todo!()
            }
            _ => {
                whatever!("Invalid action")
            }
        }
    }

    Ok(())
}

pub async fn add_account_modal_ui(
    ctx: Context<'_>,
    pre_interaction: &serenity::ComponentInteraction,
    game: SupportedGames,
) -> Result<(), BotError> {
    let user = ctx.author();
    let data = ctx.data().data_manager.clone();

    let title = format!("Add {} Account", game);

    let account_id_custom_id = format!("{}_account_id", user.id);
    let account_id_component =
        serenity::CreateModalComponent::Label(serenity::CreateLabel::input_text(
            "Account ID",
            serenity::CreateInputText::new(serenity::InputTextStyle::Short, &account_id_custom_id)
                .style(serenity::InputTextStyle::Short)
                .required(true)
                .placeholder("Enter your account ID here"),
        ));

    let account_desc_custom_id = format!("{}_account_desc", user.id);
    let account_desc_component =
        serenity::CreateModalComponent::Label(serenity::CreateLabel::input_text(
            "Account Description",
            serenity::CreateInputText::new(
                serenity::InputTextStyle::Short,
                &account_desc_custom_id,
            )
            .style(serenity::InputTextStyle::Short)
            .required(true)
            .placeholder("Enter a description for this account"),
        ));

    let modal_custom_id = format!("{}_add_account_modal_ui", user.id);
    let modal = serenity::CreateModal::new(&modal_custom_id, title)
        .components(vec![account_id_component, account_desc_component]);

    // respond to the interaction with a modal
    pre_interaction
        .create_response(
            ctx.http(),
            serenity::CreateInteractionResponse::Modal(modal),
        )
        .await
        .context(GeneralSerenitySnafu)?;

    let mut collector = serenity::ModalInteractionCollector::new(ctx.serenity_context())
        .custom_ids(vec![FixedString::from_str_trunc(&modal_custom_id)])
        .timeout(std::time::Duration::from_mins(3))
        .stream();

    let mut account_id = 0_i64;
    let mut account_desc = String::new();

    if let Some(modal_interaction) = collector.next().await {
        tracing::info!("{:?}", &modal_interaction);
        modal_interaction
            .create_response(ctx.http(), serenity::CreateInteractionResponse::Acknowledge)
            .await
            .context(GeneralSerenitySnafu)?;

        let custom_id = modal_interaction.data.custom_id.as_str();

        if custom_id == modal_custom_id {
            for component in &modal_interaction.data.components {
                if let serenity::ModalComponent::Label(serenity::Label {
                    component: label_component,
                    ..
                }) = component
                    && let serenity::LabelComponent::InputText(input_text) = label_component
                {
                    let custom_id = input_text.custom_id.as_str();
                    if custom_id == account_id_custom_id {
                        match input_text.value.clone().unwrap_or_default().parse::<i64>() {
                            Ok(val) => account_id = val,
                            Err(_) => {
                                let component = serenity::CreateContainerComponent::TextDisplay(
                                    serenity::CreateTextDisplay::new(
                                        "Invalid Account UID. Ensure numbers only and no spaces ",
                                    ),
                                );
                                let component = serenity::CreateComponent::Container(
                                    serenity::CreateContainer::new(vec![component])
                                        .accent_color(serenity::Colour::RED),
                                );
                                pre_interaction
                                    .create_followup(
                                        ctx.http(),
                                        serenity::CreateInteractionResponseFollowup::new()
                                            .components(vec![component]),
                                    )
                                    .await
                                    .context(GeneralSerenitySnafu)?;
                                whatever!("Unable to create account: invalid account UID provided");
                            }
                        }
                    } else if custom_id == account_desc_custom_id {
                        account_desc = input_text.value.clone().unwrap_or_default().into_string();
                    } else {
                        tracing::warn!("Unknown interaction custom_id: {custom_id}");
                    }
                }
            }
        }
        modal_interaction
    } else {
        whatever!("Interaction not found")
    };

    data.akend_tracker()
        .insert_akend_user(user.id.get(), account_id, &account_desc)
        .await
        .context(DataManagerSnafu)?;

    let text_display_comp =
        serenity::CreateComponent::TextDisplay(serenity::CreateTextDisplay::new(format!(
            "Bound new account UID {account_id} to {} with description {account_desc}",
            user.display_name()
        )));
    pre_interaction
        .create_followup(
            ctx.http(),
            serenity::CreateInteractionResponseFollowup::new()
                .flags(serenity::MessageFlags::EPHEMERAL | serenity::MessageFlags::IS_COMPONENTS_V2)
                .components(vec![text_display_comp]),
        )
        .await
        .context(GeneralSerenitySnafu)?;

    tracing::info!("Finished collecting modal interactions");

    Ok(())
}
