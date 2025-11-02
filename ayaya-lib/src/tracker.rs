use poise::serenity_prelude as serenity;
use serde::{Deserialize, Deserializer, Serialize};
use snafu::{OptionExt, ResultExt};
use strum::VariantNames;

use crate::{
    CommandResult, Context,
    error::{
        BotError, DataManagerSnafu, GeneralSerenitySnafu, JsonSnafu, ReqwestSnafu, TrackerSnafu,
        UrlParseSnafu,
    },
    tracker::error::{InvalidUrlSnafu, TrackerError, WuwaPlayerIdInvalidSnafu},
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
    const WUWA_REQ_URL: &str = "https://gmserver-api.aki-game2.net/gacha/record/query";

    match game {
        SupportedGames::WutheringWaves => {
            if let Some(link) = link {
                let url = url::Url::parse(&link).context(UrlParseSnafu)?;
                let mut server_id: &str = Default::default();
                let mut record_id: &str = Default::default();
                let mut player_id: u64 = 0;
                let not_url = url
                    .fragment()
                    .context(InvalidUrlSnafu)
                    .context(TrackerSnafu)?;

                // parse the remaining info
                // why is this code so messy
                let new = not_url.strip_prefix("/record?").unwrap_or_default();
                for c in new.split("&") {
                    let (key, value) = c.split_once("=").expect("able to split");

                    match key {
                        "svr_id" => {
                            server_id = value;
                        }
                        "record_id" => {
                            record_id = value;
                        }
                        "player_id" => {
                            player_id = value
                                .parse::<u64>()
                                .context(WuwaPlayerIdInvalidSnafu)
                                .context(TrackerSnafu)?;
                        }
                        _ => {}
                    }
                }
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

                let requests = WuwaRequestBuilder::new()
                    .player_id(player_id)
                    .record_id(record_id)
                    .server_id(server_id)
                    .build()
                    .context(TrackerSnafu)?;

                let client = reqwest::Client::new();
                let mut pulls: Vec<ParsedWuwaPull> = vec![];
                for req in requests {
                    let json = req.as_json()?;
                    let res = client
                        .post(WUWA_REQ_URL)
                        .header("Content-Type", "application/json")
                        .body(json)
                        .send()
                        .await
                        .context(ReqwestSnafu)?;

                    let wrapper = res
                        .json::<DeserializeWrapper>()
                        .await
                        .expect("deserialized properly");
                    pulls.extend(wrapper.data);
                }

                let pulls_len = pulls.len();
                tracing::info!("length: {pulls_len}");

                if pulls_len > 0 {
                    let inserted = pulls_manager
                        .insert_wuwa_pulls(player_id, pulls)
                        .await
                        .context(DataManagerSnafu)?;
                    ctx.reply(format!(
                        "Records parsed: {}, {} new records inserted into database",
                        pulls_len, inserted,
                    ))
                    .await
                    .context(GeneralSerenitySnafu)?;
                } else {
                    ctx.reply("No records found.")
                        .await
                        .context(GeneralSerenitySnafu)?;
                }
            }
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

#[derive(Default)]
struct WuwaRequestBuilder<'a> {
    server_id: Option<&'a str>,
    record_id: Option<&'a str>,
    player_id: Option<u64>,
}

impl<'a> WuwaRequestBuilder<'a> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn server_id(mut self, server_id: &'a str) -> Self {
        self.server_id = Some(server_id);
        self
    }

    pub fn record_id(mut self, record_id: &'a str) -> Self {
        self.record_id = Some(record_id);
        self
    }

    pub fn player_id(mut self, player_id: u64) -> Self {
        self.player_id = Some(player_id);
        self
    }

    pub fn build(self) -> Result<Vec<WuwaRequest>, TrackerError> {
        if let Some(player_id) = self.player_id
            && let Some(server_id) = self.server_id
            && let Some(record_id) = self.record_id
        {
            let it = [1, 2, 3, 4]
                .iter()
                .map(|e| WuwaRequest {
                    player_id,
                    card_pool_type: *e,
                    language_code: "en".to_string(),
                    server_id: server_id.to_string(),
                    record_id: record_id.to_string(),
                })
                .collect();
            Ok(it)
        } else {
            Err(TrackerError::WuwaRequestIncomplete)
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WuwaRequest {
    player_id: u64,
    card_pool_type: u8,
    server_id: String,
    record_id: String,
    language_code: String,
}

impl WuwaRequest {
    pub fn as_json(&self) -> Result<String, BotError> {
        serde_json::to_string(self).context(JsonSnafu)
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
}

#[derive(Debug, Deserialize, Copy, Clone)]
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
    }

    impl ErrorName for TrackerError {
        fn name(&self) -> String {
            let str = match self {
                TrackerError::WuwaRequestIncomplete => "wuwa_request_incomplete",
                TrackerError::UserGameIdMismatch => "user_game_id_mismatch",
                TrackerError::WuwaPlayerIdInvalid { .. } => "wuwa_player_id_invalid",
                TrackerError::InvalidUrl => "invalid_url",
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
