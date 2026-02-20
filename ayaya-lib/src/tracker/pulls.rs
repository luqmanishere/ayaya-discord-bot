use std::collections::HashMap;

use ::serenity::small_fixed_array::FixedString;
use ayaya_tracker::gacha_tracker::{
    AdapterKind, AkEndPullDto, CardPoolType, GameAdapter, TrackerError, adapter_for,
    akend::AkEndGachaPool, apply_import_boundary,
};
use poise::serenity_prelude as serenity;
use snafu::{ResultExt, whatever};

use crate::{
    Context,
    error::{BotError, DataManagerSnafu, GeneralSerenitySnafu, TrackerSnafu},
    tracker::{SupportedGames, parse_akend_ts},
};

pub async fn pull_import_modal(
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
            if list.is_empty() {
                pre_interaction
                    .create_response(ctx.http(), serenity::CreateInteractionResponse::Acknowledge)
                    .await
                    .context(GeneralSerenitySnafu)?;
                let message = format!(
                    "No {} accounts found. Please register an account via the tracker menu.",
                    game
                );
                pre_interaction
                    .create_followup(
                        ctx.http(),
                        serenity::CreateInteractionResponseFollowup::new()
                            .ephemeral(true)
                            .content(message),
                    )
                    .await
                    .context(GeneralSerenitySnafu)?;
                return Ok(());
            };
            list.iter()
                .map(|e| {
                    serenity::CreateSelectMenuOption::new(
                        format!("{}: {}", e.ak_end_user_id, e.user_desc),
                        e.ak_end_user_id.to_string(),
                    )
                })
                .collect::<Vec<_>>()
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
                {
                    match label_component {
                        serenity::LabelComponent::InputText(input_text) => {
                            let comp_custom_id = input_text.custom_id.as_str();
                            if comp_custom_id == link_custom_id {
                                link = input_text.value.clone().unwrap_or_default().into_string();
                            } else {
                                tracing::warn!("Unknown interaction custom_id: {comp_custom_id}");
                            }
                        }
                        serenity::LabelComponent::SelectMenu(select_menu) => {
                            let comp_custom_id = select_menu.custom_id.as_str();
                            if comp_custom_id == user_account_select_component_custom_id {
                                match select_menu.values[0].clone().parse::<i64>() {
                                    Ok(val) => {
                                        tracing::info!(
                                            "AkEnd import: entered account UID: {}",
                                            val
                                        );
                                        game_account_id = val;
                                    }
                                    Err(_) => {
                                        // TODO: better error handling here
                                        modal_interaction.create_followup(ctx.http(), serenity::CreateInteractionResponseFollowup::new().content("Invalid Account UID. Ensure numbers only and no spaces ")).await.context(GeneralSerenitySnafu)?;
                                        whatever!(
                                            "Unable to create account: invalid account UID provided"
                                        );
                                    }
                                }
                            }
                        }
                        _ => {}
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
                tracing::info!(
                    "AkEnd import start: user_id={}, game_account_id={}, link_len={}",
                    user.id.get(),
                    game_account_id,
                    link.len()
                );
                let session = adapter.parse_link(&link).context(TrackerSnafu)?;

                let pulls_manager = ctx.data().data_manager.akend_tracker();

                tracing::info!("AkEnd import: fetching akend users for discord user");
                let user_game_accounts = pulls_manager
                    .get_akend_users_by_user_id(user.id.get())
                    .await
                    .context(DataManagerSnafu)?;

                if let Some(model) = user_game_accounts
                    .iter()
                    .find(|e| e.ak_end_user_id == game_account_id)
                {
                    tracing::info!(
                        "AkEnd import: matched account {} for user {}",
                        model.ak_end_user_id,
                        user.id.get()
                    );
                    tracing::info!("AkEnd import: fetching pulls from adapter");
                    let pulls = adapter
                        .fetch_pulls(&session, &ctx.data().http)
                        .await
                        .context(TrackerSnafu)?;

                    let pulls_len = pulls.len();
                    tracing::info!("length: {pulls_len}");

                    if pulls.is_empty() {
                        modal_interaction
                            .create_followup(
                                ctx.http(),
                                serenity::CreateInteractionResponseFollowup::new()
                                    .ephemeral(true)
                                    .content("No new records found."),
                            )
                            .await
                            .context(GeneralSerenitySnafu)?;
                        return Ok(());
                    }

                    // Start boundary processing
                    let mut pulls_by_pool: HashMap<String, Vec<_>> = HashMap::new();
                    for pull in pulls {
                        let pool_id = adapter.pool_id(&pull).to_string();
                        pulls_by_pool.entry(pool_id).or_default().push(pull);
                    }

                    let mut new_pulls = Vec::new();
                    let mut updated_boundaries = Vec::new();

                    for (pool_id, mut pool_pulls) in pulls_by_pool {
                        tracing::info!("AkEnd import: pool {pool_id} size={}", pool_pulls.len());
                        pool_pulls.sort_by_key(|p| parse_akend_ts(p.gacha_ts()));

                        tracing::info!("AkEnd import: fetching import boundary for pool {pool_id}");
                        let boundary = pulls_manager
                            .get_akend_import_state(model.ak_end_user_id, &pool_id)
                            .await
                            .context(DataManagerSnafu)?;

                        let filtered = apply_import_boundary(&pool_pulls, boundary, |p| {
                            parse_akend_ts(p.gacha_ts())
                        });

                        new_pulls.extend(filtered.new_items.into_iter().cloned());

                        if let Some(next_boundary) = filtered.next_boundary {
                            updated_boundaries.push((pool_id, next_boundary));
                        }
                    }

                    if new_pulls.is_empty() {
                        modal_interaction
                            .create_followup(
                                ctx.http(),
                                serenity::CreateInteractionResponseFollowup::new()
                                    .ephemeral(true)
                                    .content("No new records found."),
                            )
                            .await
                            .context(GeneralSerenitySnafu)?;

                        return Ok(());
                    }
                    // end boundary processing

                    let records: Vec<AkEndPullDto> = new_pulls
                        .into_iter()
                        .map(|pull| {
                            adapter.normalize_pull(pull, model.ak_end_user_id.to_string().as_str())
                        })
                        .collect();

                    tracing::info!(
                        "AkEnd import: inserting {} records for account {}",
                        records.len(),
                        model.ak_end_user_id
                    );
                    let inserted = pulls_manager
                        .insert_akend_pull_records(user.id.get(), model.ak_end_user_id, records)
                        .await
                        .context(DataManagerSnafu)?;

                    tracing::info!(
                        "AkEnd import: updating {} boundaries",
                        updated_boundaries.len()
                    );
                    for (pool_id, boundary) in updated_boundaries {
                        pulls_manager
                            .upsert_akend_import_state(model.ak_end_user_id, &pool_id, boundary)
                            .await
                            .context(DataManagerSnafu)?;
                    }

                    tracing::info!(
                        "AkEnd import: completed with pulls_len={}, inserted={}",
                        pulls_len,
                        inserted
                    );
                    modal_interaction
                        .create_followup(
                            ctx.http(),
                            serenity::CreateInteractionResponseFollowup::new()
                                .ephemeral(true)
                                .flags(serenity::MessageFlags::IS_COMPONENTS_V2 | serenity::MessageFlags::EPHEMERAL)
                                // TODO: better formatting
                                .components(vec![serenity::CreateComponent::TextDisplay(
                                    serenity::CreateTextDisplay::new(format!("Records parsed: {pulls_len}, {inserted} new records inserted into database"))
                                )])
                        )
                        .await
                        .context(GeneralSerenitySnafu)?;
                } else {
                    tracing::warn!(
                        "AkEnd import: no matching account for user_id={} game_account_id={}",
                        user.id.get(),
                        game_account_id
                    );
                }
            }
        }
    }

    Ok(())
}

/// This function will show the UI non-ephemeral
pub async fn pulls_data_ui(
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
            let akend_tracker = ctx.data().data_manager.akend_tracker();

            let akend_accounts = akend_tracker
                .get_akend_users_by_user_id(user_id.get())
                .await
                .context(DataManagerSnafu)?;

            if akend_accounts.is_empty() {
                let text_comp =
                    serenity::CreateComponent::TextDisplay(serenity::CreateTextDisplay::new(
                        "No Arknights Endfield account found for you.",
                    ));
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
                for account in akend_accounts {
                    let pulls = akend_tracker
                        .get_all_char_pulls_from_akend_id(account.ak_end_user_id)
                        .await
                        .context(DataManagerSnafu)?;

                    // TODO: summary data by pool type: total pull count, current count to pity, count to soft pity 25
                    // TODO: history of pulled 6 stars,
                    // TODO: summoned character count

                    let six_stars = pulls.iter().filter(|e| e.rarity == 6).collect::<Vec<_>>();
                    let limited_chars = six_stars
                        .iter()
                        .filter(|e| e.pool_type == AkEndGachaPool::Special.get_api_name())
                        .count();

                    let msg = format!(
                        "Account ID {}: Limited 6 star characters obtained: {limited_chars}",
                        account.ak_end_user_id
                    );
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
    }
    Ok(())
}
