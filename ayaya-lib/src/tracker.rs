use ::serenity::all::CollectComponentInteractions;
use poise::serenity_prelude as serenity;
use snafu::ResultExt;
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

#[poise::command(slash_command, subcommands("pulls"), aliases("t"))]
pub async fn tracker(_ctx: Context<'_>) -> Result<(), BotError> {
    Ok(())
}

#[poise::command(slash_command, subcommands("import_pulls", "stats"), aliases("p"))]
pub async fn pulls(_ctx: Context<'_>) -> CommandResult {
    Ok(())
}

/// Import pulls from a game's links
#[poise::command(slash_command, rename = "import", aliases("i"))]
pub async fn import_pulls(
    ctx: Context<'_>,
    // #[autocomplete = "autocomplete_game"]
    #[description = "A supported game"] game: SupportedGames,
    #[description = "Link from the game for parsing"] link: Option<String>,
) -> CommandResult {
    // TODO: other games so we can know whether by game seperation is necessary
    let Some(link) = link else {
        ctx.reply("Please provide the import link from the game.")
            .await
            .context(GeneralSerenitySnafu)?;
        return Ok(());
    };

    ctx.defer().await.context(GeneralSerenitySnafu)?;
    match adapter_for(game.game_id()) {
        AdapterKind::Wuwa(adapter) => {
            let session = adapter.parse_link(&link).context(TrackerSnafu)?;
            let player_id = session.player_id;
            tracing::info!("player id: {player_id}");

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
                pool_pulls
                    .sort_by_key(|p| std::cmp::Reverse(p.time.assume_offset(time::UtcOffset::UTC)));

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
        AdapterKind::AkEnd(akend_adapter) => {
            todo!()
        }
        #[expect(unused)]
        _ => {
            tracing::warn!("Unimplemented!");
        }
    }

    Ok(())
}

/// Shows pull stats for a game
#[poise::command(slash_command, aliases("s"))]
async fn stats(ctx: Context<'_>) -> CommandResult {
    // ctx.defer().await.context(GeneralSerenitySnafu)?;
    let game = {
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

        let reply = ctx
            .reply_builder(poise::CreateReply::default())
            .flags(serenity::MessageFlags::IS_COMPONENTS_V2)
            .ephemeral(true)
            .components(components);
        let h = ctx.send(reply).await.context(GeneralSerenitySnafu)?;

        let interaction = match h
            .message()
            .await
            .context(GeneralSerenitySnafu)?
            .id
            .collect_component_interactions(ctx.serenity_context())
            .timeout(std::time::Duration::from_mins(3))
            .await
        {
            Some(i) => {
                i.create_response(ctx.http(), serenity::CreateInteractionResponse::Acknowledge)
                    .await
                    .context(GeneralSerenitySnafu)?;
                i
            }
            None => {
                ctx.reply("Timed out waiting for game selection. You need to select a game")
                    .await
                    .context(GeneralSerenitySnafu)?;
                return Ok(());
            }
        };

        let game = match interaction.data.kind {
            serenity::ComponentInteractionDataKind::StringSelect { values } => {
                let selected_game = &values[0];
                match SupportedGames::from_str(selected_game) {
                    Ok(g) => g,
                    Err(_) => {
                        ctx.reply("Invalid game selected.")
                            .await
                            .context(GeneralSerenitySnafu)?;
                        return Ok(());
                    }
                }
            }
            _ => {
                ctx.reply("Invalid interaction data.")
                    .await
                    .context(GeneralSerenitySnafu)?;
                return Ok(());
            }
        };

        game
    };

    match game {
        SupportedGames::WutheringWaves => {
            // show current amount of pulls, 5 star pity count, 4 star pity count, and list of 5 star chars
            let user_id = ctx.author().id;
            let wuwa_tracker = ctx.data().data_manager.wuwa_tracker();

            let player_ids = wuwa_tracker
                .get_wuwa_user_from_user_id(user_id.get())
                .await
                .context(DataManagerSnafu)?;

            if player_ids.is_empty() {
                ctx.reply("No Wuwa Account found for you.")
                    .await
                    .context(GeneralSerenitySnafu)?;
            } else {
                // show embed
                for wuwa_player_id in player_ids {
                    let pulls = wuwa_tracker
                        .get_pulls_from_wuwa_id(wuwa_player_id.wuwa_user_id as u64)
                        .await
                        .context(DataManagerSnafu)?;
                    // TODO: alg
                    let five_stars = pulls
                        .iter()
                        .filter(|e| e.quality_level == 5)
                        .collect::<Vec<_>>();
                    let limited_chars = five_stars
                        .iter()
                        .filter(|e| e.pull_type == CardPoolType::EventCharacterConvene as i32)
                        .count();
                    let msg = format!("Limited 5 star characters obtained: {limited_chars}");
                    ctx.reply(msg).await.context(GeneralSerenitySnafu)?;
                }
            }
        }
        SupportedGames::AkEnd => {
            // TODO: implement
        }
    }
    Ok(())
}

#[derive(poise::ChoiceParameter, strum::EnumString, strum::Display, strum::VariantNames)]
enum SupportedGames {
    #[name = "Wuthering Waves"]
    WutheringWaves,
    #[name = "Arknights Endfield"]
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
