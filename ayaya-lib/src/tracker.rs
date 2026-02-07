use ::serenity::{
    all::CollectComponentInteractions, futures::StreamExt, small_fixed_array::FixedString,
};
use poise::serenity_prelude as serenity;
use snafu::{ResultExt, whatever};
use std::{collections::HashMap, str::FromStr};
use strum::VariantNames;

use crate::{
    CommandResult, Context,
    error::{BotError, DataManagerSnafu, GeneralSerenitySnafu, TrackerSnafu},
};

use ayaya_tracker::gacha_tracker::{
    AdapterKind, CardPoolType, GameAdapter, GameId, TrackerError, adapter_for,
    apply_import_boundary,
};

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
enum SupportedGames {
    #[strum(to_string = "Wuthering Waves")]
    WutheringWaves,
    #[strum(to_string = "Arknights Endfield")]
    AkEnd,
}

impl SupportedGames {
    fn game_id(self) -> GameId {
        match self {
            SupportedGames::WutheringWaves => GameId::WutheringWaves,
            SupportedGames::AkEnd => GameId::ArknightsEndfield,
        }
    }
}

async fn game_selector_ui(
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

async fn main_account_management_ui(
    ctx: Context<'_>,
    pre_interaction: Option<&serenity::ComponentInteraction>,
    edit: bool,
    game: SupportedGames,
) -> Result<(), BotError> {
    let user = ctx.author();
    let data = ctx.data().data_manager.clone();
    let title = format!(
        "# Manage {}'s {} Accounts",
        user.display_name(),
        game.to_string()
    );
    let subtitle = format!("## Current linked accounts:");
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
    } else {
    }

    Ok(())
}

async fn add_account_modal_ui(
    ctx: Context<'_>,
    pre_interaction: &serenity::ComponentInteraction,
    game: SupportedGames,
) -> Result<(), BotError> {
    let user = ctx.author();
    let data = ctx.data().data_manager.clone();

    let title = format!(
        "# Add {} Account for {}",
        game.to_string(),
        user.display_name()
    );

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
                                // TODO: better error handling here
                                pre_interaction.create_followup(ctx.http(), serenity::CreateInteractionResponseFollowup::new().content("Invalid Account UID. Ensure numbers only and no spaces ")).await.context(GeneralSerenitySnafu)?;
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

async fn pull_import_modal(
    ctx: Context<'_>,
    pre_interaction: &serenity::ComponentInteraction,
    game: SupportedGames,
) -> Result<(), BotError> {
    let user = ctx.author();
    let data = ctx.data().data_manager.clone();

    let title = format!("Import pulls for {game}");

    let script = match game {
        SupportedGames::WutheringWaves => {
            r#"iwr -UseBasicParsing -Headers @{"User-Agent"="Mozilla/5.0"} https://raw.githubusercontent.com/wuwatracker/wuwatracker/refs/heads/main/import.ps1 | iex"#
        }
        SupportedGames::AkEnd => {
            "Set-ExecutionPolicy Bypass -Scope Process -Force; [System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor 3072; $scriptUrl='https://raw.githubusercontent.com/holstonline/endfield-gacha-url/refs/heads/main/extract-headhunt-api-url.ps1'; $scriptText=(Invoke-WebRequest -UseBasicParsing -Uri $scriptUrl).Content; Invoke-Expression $scriptText"
        }
    };
    let script_prompt =
        serenity::CreateModalComponent::TextDisplay(serenity::CreateTextDisplay::new(format!(
            "Open Windows Powershell and run this:\n```sh\n{}\n```",
            script
        )));

    let link_custom_id = format!("{}_link", user.id.get());
    let link_input_compnent = serenity::CreateModalComponent::Label(
        serenity::CreateLabel::input_text(
            "Link",
            serenity::CreateInputText::new(serenity::InputTextStyle::Short, &link_custom_id)
                .required(true)
                .placeholder("Paste the extracted game link here."),
        )
        .description("Example: https://ef-webview.gryphline.com/api/record/char..."),
    );

    let user_accounts_choices = match game {
        SupportedGames::WutheringWaves => vec![],
        SupportedGames::AkEnd => {
            let list = data
                .akend_tracker()
                .get_akend_users_by_user_id(user.id.get())
                .await
                .context(DataManagerSnafu)?;
            let list = list
                .iter()
                .map(|e| {
                    serenity::CreateSelectMenuOption::new(
                        format!("{}: {}", e.ak_end_user_id, e.user_desc),
                        e.ak_end_user_id.to_string(),
                    )
                })
                .collect::<Vec<_>>();
            list
        }
    };
    let user_account_select_component_custom_id = format!("{}_user_account", user.id.get());
    let user_account_select_component =
        serenity::CreateModalComponent::Label(serenity::CreateLabel::select_menu(
            "Select Account UID",
            serenity::CreateSelectMenu::new(
                &user_account_select_component_custom_id,
                serenity::CreateSelectMenuKind::String {
                    options: user_accounts_choices.into(),
                },
            )
            .required(true),
        ));

    let mut modal_components = vec![script_prompt, link_input_compnent];
    if game == SupportedGames::AkEnd {
        modal_components.push(user_account_select_component);
    }
    let modal_custom_id = format!("{}_modal", user.id.get());
    let modal = serenity::CreateModal::new(&modal_custom_id, title).components(modal_components);

    pre_interaction
        .create_response(
            ctx.http(),
            serenity::CreateInteractionResponse::Modal(modal),
        )
        .await
        .context(GeneralSerenitySnafu)?;

    if let Some(modal_interaction) =
        serenity::ModalInteractionCollector::new(ctx.serenity_context())
            .custom_ids(vec![FixedString::from_str_trunc(&modal_custom_id)])
            .timeout(std::time::Duration::from_mins(5))
            .await
    {
        let custom_id = modal_interaction.data.custom_id.as_str();
        let mut link = String::new();
        let mut game_account_id = 0_i64;

        if custom_id == modal_custom_id {
            modal_interaction
                .create_response(ctx.http(), serenity::CreateInteractionResponse::Acknowledge)
                .await
                .context(GeneralSerenitySnafu)?;
            for component in &modal_interaction.data.components {
                if let serenity::ModalComponent::Label(serenity::Label {
                    component: label_component,
                    ..
                }) = component
                    && let serenity::LabelComponent::InputText(input_text) = label_component
                {
                    let custom_id = input_text.custom_id.as_str();
                    if custom_id == link_custom_id {
                        link = input_text.value.clone().unwrap_or_default().into_string();
                    } else if custom_id == user_account_select_component_custom_id {
                        match input_text.value.clone().unwrap_or_default().parse::<i64>() {
                            Ok(val) => game_account_id = val,
                            Err(_) => {
                                // TODO: better error handling here
                                pre_interaction.create_followup(ctx.http(), serenity::CreateInteractionResponseFollowup::new().content("Invalid Account UID. Ensure numbers only and no spaces ")).await.context(GeneralSerenitySnafu)?;
                                whatever!("Unable to create account: invalid account UID provided");
                            }
                        }
                    } else {
                        tracing::warn!("Unknown interaction custom_id: {custom_id}");
                    }
                }
            }
        }

        match adapter_for(game.game_id()) {
            AdapterKind::Wuwa(adapter) => {
                let session = adapter.parse_link(&link).context(TrackerSnafu)?;
                let player_id = session.player_id;
                tracing::info!("parsed player id: {player_id}");

                let pulls_manager = ctx.data().data_manager.wuwa_tracker();

                // check for the player id owner
                if let Some(user_id) = pulls_manager
                    .get_user_id_from_wuwa_user(player_id)
                    .await
                    .context(DataManagerSnafu)?
                {
                    if user_id != ctx.author().id.get() {
                        return Err(TrackerError::UserGameIdMismatch).context(TrackerSnafu);
                    }
                } else {
                    // register the player id owner
                    // TODO: interface
                    pulls_manager
                        .insert_wuwa_user(ctx.author().id.get(), player_id)
                        .await
                        .context(DataManagerSnafu)?;
                }

                let pulls = adapter
                    .fetch_pulls(&session, &ctx.data().http)
                    .await
                    .context(TrackerSnafu)?;

                let pulls_len = pulls.len();
                tracing::info!("length: {pulls_len}");

                if pulls_len == 0 {
                    ctx.reply("No records found.")
                        .await
                        .context(GeneralSerenitySnafu)?;
                    return Ok(());
                }

                let mut pulls_by_pool: HashMap<String, Vec<_>> = HashMap::new();
                for pull in pulls {
                    let pool_id = adapter.pool_id(&pull).to_string();
                    pulls_by_pool.entry(pool_id).or_default().push(pull);
                }

                let mut new_pulls = Vec::new();
                let mut updated_boundaries = Vec::new();

                for (pool_id, mut pool_pulls) in pulls_by_pool {
                    pool_pulls.sort_by_key(|p| {
                        std::cmp::Reverse(p.time.assume_offset(time::UtcOffset::UTC))
                    });

                    let boundary = pulls_manager
                        .get_wuwa_import_state(player_id, &pool_id)
                        .await
                        .context(DataManagerSnafu)?;

                    let filtered = apply_import_boundary(&pool_pulls, boundary, |p| {
                        p.time.assume_offset(time::UtcOffset::UTC)
                    });

                    new_pulls.extend(filtered.new_items.into_iter().cloned());

                    if let Some(next_boundary) = filtered.next_boundary {
                        updated_boundaries.push((pool_id, next_boundary));
                    }
                }

                if new_pulls.is_empty() {
                    ctx.reply("No new records found.")
                        .await
                        .context(GeneralSerenitySnafu)?;
                    return Ok(());
                }

                let user_game_id = player_id.to_string();
                let records = new_pulls
                    .into_iter()
                    .map(|pull| adapter.normalize_pull(pull, &user_game_id))
                    .collect();

                let inserted = pulls_manager
                    .insert_wuwa_pull_records(player_id, records)
                    .await
                    .context(DataManagerSnafu)?;

                for (pool_id, boundary) in updated_boundaries {
                    pulls_manager
                        .upsert_wuwa_import_state(player_id, &pool_id, boundary)
                        .await
                        .context(DataManagerSnafu)?;
                }

                ctx.reply(format!(
                    "Records parsed: {}, {} new records inserted into database",
                    pulls_len, inserted,
                ))
                .await
                .context(GeneralSerenitySnafu)?;
            }
            AdapterKind::AkEnd(adapter) => {
                todo!()
            }
        }
    }

    Ok(())
}

/// This function will show the UI non-ephemeral
async fn pulls_data_ui(
    ctx: Context<'_>,
    pre_interaction: &serenity::ComponentInteraction,
    game: SupportedGames,
) -> Result<(), BotError> {
    pre_interaction
        .defer(ctx.http())
        .await
        .context(GeneralSerenitySnafu)?;
    let user_id = ctx.author().id;

    match game {
        SupportedGames::WutheringWaves => {
            // show current amount of pulls, 5 star pity count, 4 star pity count, and list of 5 star chars
            let wuwa_tracker = ctx.data().data_manager.wuwa_tracker();

            let player_ids = wuwa_tracker
                .get_wuwa_user_from_user_id(user_id.get())
                .await
                .context(DataManagerSnafu)?;

            if player_ids.is_empty() {
                let text_comp = serenity::CreateComponent::TextDisplay(
                    serenity::CreateTextDisplay::new("No Wuwa Account found for you."),
                );
                pre_interaction
                    .create_followup(
                        ctx.http(),
                        serenity::CreateInteractionResponseFollowup::new()
                            .ephemeral(false)
                            .flags(serenity::MessageFlags::IS_COMPONENTS_V2)
                            .components(vec![text_comp]),
                    )
                    .await
                    .context(GeneralSerenitySnafu)?;
            } else {
                // show embed
                for wuwa_player_id in player_ids {
                    let pulls = wuwa_tracker
                        .get_pulls_from_wuwa_id(wuwa_player_id.wuwa_user_id as u64)
                        .await
                        .context(DataManagerSnafu)?;
                    // TODO: proper data to be shown alg
                    let five_stars = pulls
                        .iter()
                        .filter(|e| e.quality_level == 5)
                        .collect::<Vec<_>>();
                    let limited_chars = five_stars
                        .iter()
                        .filter(|e| e.pull_type == CardPoolType::EventCharacterConvene as i32)
                        .count();

                    let msg = format!("Limited 5 star characters obtained: {limited_chars}");
                    let text_comp = serenity::CreateComponent::TextDisplay(
                        serenity::CreateTextDisplay::new(msg),
                    );

                    pre_interaction
                        .create_followup(
                            ctx.http(),
                            serenity::CreateInteractionResponseFollowup::new()
                                .ephemeral(false)
                                .flags(serenity::MessageFlags::IS_COMPONENTS_V2)
                                .components(vec![text_comp]),
                        )
                        .await
                        .context(GeneralSerenitySnafu)?;
                }
            }
        }
        SupportedGames::AkEnd => {
            // TODO: implement
        }
    }
    Ok(())
}
