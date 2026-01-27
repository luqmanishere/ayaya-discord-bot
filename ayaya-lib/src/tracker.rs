use poise::serenity_prelude as serenity;
use serde::{Deserialize, Deserializer};
use std::collections::HashMap;
use snafu::ResultExt;
use strum::VariantNames;

use crate::{
    CommandResult, Context,
    error::{
        BotError, DataManagerSnafu, GeneralSerenitySnafu, TrackerSnafu,
    },
    tracker::error::TrackerError,
    tracker_adapter::{AdapterKind, GameAdapter, GameId, adapter_for, apply_import_boundary},
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
#[poise::command(slash_command, rename = "import", aliases("i"), ephemeral)]
pub async fn import_pulls(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_game"]
    #[description = "A supported game"]
    game: SupportedGames,
    #[description = "Link from the game for parsing"] link: Option<String>,
) -> CommandResult {
    // TODO: other games so we can know whether by game seperation is necessary

    ctx.defer_ephemeral().await.context(GeneralSerenitySnafu)?;
    match adapter_for(game.game_id()) {
        AdapterKind::Wuwa(adapter) => {
            let Some(link) = link else {
                ctx.reply("Please provide the import link from the game.")
                    .await
                    .context(GeneralSerenitySnafu)?;
                return Ok(());
            };

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
    }

    Ok(())
}

/// Shows pull stats for a game
#[poise::command(slash_command, aliases("s"), ephemeral)]
async fn stats(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_game"]
    #[description = "A supported game"]
    game: SupportedGames,
) -> CommandResult {
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
    }
    Ok(())
}

#[derive(strum::EnumString, strum::Display, strum::VariantNames)]
enum SupportedGames {
    WutheringWaves,
}

impl SupportedGames {
    fn game_id(self) -> GameId {
        match self {
            SupportedGames::WutheringWaves => GameId::WutheringWaves,
        }
    }
}

async fn autocomplete_game(
    _ctx: Context<'_>,
    partial: &str,
) -> impl Iterator<Item = serenity::AutocompleteChoice> {
    let partial = partial.to_lowercase();
    SupportedGames::VARIANTS.iter().filter_map(move |e| {
        if e.to_lowercase().contains(&partial) {
            Some(serenity::AutocompleteChoice::new(
                e.to_string(),
                e.to_string(),
            ))
        } else {
            None
        }
    })
}

#[derive(Debug, Deserialize)]
pub struct DeserializeWrapper {
    pub data: Vec<ParsedWuwaPull>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ParsedWuwaPull {
    pub card_pool_type: CardPoolType,
    pub resource_id: u64,
    pub quality_level: u64,
    pub resource_type: ResourceType,
    pub name: String,
    pub count: u64,
    #[serde(deserialize_with = "deserialize_time")]
    pub time: time::PrimitiveDateTime,
}

fn deserialize_time<'de, D>(deserializer: D) -> Result<time::PrimitiveDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    static FORMAT: &[time::format_description::BorrowedFormatItem] = time::macros::format_description!(
        "[year]-[month repr:numerical]-[day] [hour repr:24]:[minute]:[second]"
    );

    let s: &str = Deserialize::deserialize(deserializer)?;
    let ti = time::PrimitiveDateTime::parse(s, FORMAT).expect("format proper");
    Ok(ti)
}

#[derive(Debug, Deserialize, Clone, Copy, strum::Display)]
pub enum ResourceType {
    Weapon,
    Resonator,
    Item,
}

#[derive(Debug, Deserialize, Copy, Clone, Eq, Hash, PartialEq)]
#[expect(clippy::enum_variant_names)]
pub enum CardPoolType {
    #[serde(rename = "Resonators Accurate Modulation")]
    EventCharacterConvene,
    #[serde(rename = "Resonators Accurate Modulation - 2")]
    EventWeaponConvene,
    #[serde(rename = "Weapons Accurate Modulation")]
    StandardCharacterConvene,
    #[serde(rename = "Full-Range Modualtion")]
    StandardWeaponConvene,
}

pub mod error {
    use std::num::ParseIntError;

    use snafu::Snafu;

    use crate::error::ErrorName;

    #[derive(Debug, Snafu)]
    #[snafu(visibility(pub))]
    pub enum TrackerError {
        #[snafu(display("not enough arguments to build"))]
        WuwaRequestIncomplete,
        #[snafu(display("You are not the owner of this player id."))]
        UserGameIdMismatch,
        #[snafu(display("The player id format is invalid"))]
        WuwaPlayerIdInvalid { source: ParseIntError },

        #[snafu(display("The provided url is invalid."))]
        InvalidUrl,
        #[snafu(display("Failed to send request to Wuwa API."))]
        WuwaRequestFailed { source: reqwest::Error },
        #[snafu(display("Failed to read Wuwa API response."))]
        WuwaResponseRead { source: reqwest::Error },
        #[snafu(display("Failed to decode Wuwa API response."))]
        WuwaResponseDecode { source: serde_json::Error },
        #[snafu(display("Failed to encode Wuwa API request."))]
        WuwaRequestEncode { source: serde_json::Error },
    }

    impl ErrorName for TrackerError {
        fn name(&self) -> String {
            let str = match self {
                TrackerError::WuwaRequestIncomplete => "wuwa_request_incomplete",
                TrackerError::UserGameIdMismatch => "user_game_id_mismatch",
                TrackerError::WuwaPlayerIdInvalid { .. } => "wuwa_player_id_invalid",
                TrackerError::InvalidUrl => "invalid_url",
                TrackerError::WuwaRequestFailed { .. } => "wuwa_request_failed",
                TrackerError::WuwaResponseRead { .. } => "wuwa_response_read",
                TrackerError::WuwaResponseDecode { .. } => "wuwa_response_decode",
                TrackerError::WuwaRequestEncode { .. } => "wuwa_request_encode",
            };
            format!("tracker::{str}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_type_1() {
        let model_path = format!(
            "{}/../dev/wuwa_model_type_1.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let DeserializeWrapper { data: pulls } =
            serde_json::from_str(&std::fs::read_to_string(model_path).unwrap()).unwrap();

        assert!(pulls.iter().find(|e| e.name == "Carlotta").is_some());
    }

    #[test]
    fn test_deserialize_type_2() {
        let model_path = format!(
            "{}/../dev/wuwa_model_type_2.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let DeserializeWrapper { data: pulls } =
            serde_json::from_str(&std::fs::read_to_string(model_path).unwrap()).unwrap();

        assert!(pulls.iter().find(|e| e.name == "The Last Dance").is_some());
    }

    #[test]
    fn test_deserialize_type_3() {
        let model_path = format!(
            "{}/../dev/wuwa_model_type_3.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let DeserializeWrapper { data: pulls } =
            serde_json::from_str(&std::fs::read_to_string(model_path).unwrap()).unwrap();

        assert!(
            pulls
                .iter()
                .find(|e| e.name == "Originite: Type IV")
                .is_some()
        );
    }

    #[test]
    fn test_deserialize_type_4() {
        let model_path = format!(
            "{}/../dev/wuwa_model_type_4.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let DeserializeWrapper { data: pulls } =
            serde_json::from_str(&std::fs::read_to_string(model_path).unwrap()).unwrap();

        assert!(pulls.iter().find(|e| e.name == "Cosmic Ripples").is_some());
    }
}
