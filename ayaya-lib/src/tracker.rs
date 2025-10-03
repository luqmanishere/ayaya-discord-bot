use poise::serenity_prelude as serenity;
use serde::{Deserialize, Deserializer, Serialize};
use strum::VariantNames;

use crate::{CommandResult, Context, error::BotError};

#[poise::command(slash_command, subcommands("pulls"), aliases("t"))]
pub async fn tracker(_ctx: Context<'_>) -> Result<(), BotError> {
    Ok(())
}

#[poise::command(slash_command, subcommands("import_pulls"), aliases("p"))]
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

    ctx.defer_ephemeral().await?;
    const WUWA_REQ_URL: &str = "https://gmserver-api.aki-game2.net/gacha/record/query";

    match game {
        SupportedGames::WutheringWaves => {
            if let Some(link) = link {
                let url = url::Url::parse(&link)?;
                let mut server_id: &str = Default::default();
                let mut record_id: &str = Default::default();
                let mut player_id: u64 = 0;
                let not_url = url.fragment().expect("has fragment");

                // parse the remaining info
                // why is this code so messy
                let new = not_url.strip_prefix("/record?").unwrap_or_default();
                for c in new.split("&") {
                    let (key, value) = c.split_once("=").unwrap();

                    match key.as_ref() {
                        "svr_id" => {
                            server_id = value;
                        }
                        "record_id" => {
                            record_id = value;
                        }
                        "player_id" => {
                            player_id = value.parse::<u64>().map_err(BotError::generic)?;
                        }
                        _ => {}
                    }
                }
                tracing::info!("player id: {player_id}");

                let pulls_manager = ctx.data().data_manager.wuwa_tracker();

                // check for the player id owner
                if let Some(user_id) = pulls_manager.get_user_id_from_wuwa_user(player_id).await? {
                    if user_id != ctx.author().id.get() {
                        return Err(BotError::StringError(
                            "You are not the owner of this player id".to_string(),
                        ));
                    }
                } else {
                    // register the player id owner
                    // TODO: interface
                    pulls_manager
                        .insert_wuwa_user(ctx.author().id.get(), player_id)
                        .await?;
                }

                let requests = WuwaRequestBuilder::new()
                    .player_id(player_id)
                    .record_id(record_id)
                    .server_id(server_id)
                    .build()
                    .map_err(BotError::string)?;

                let client = reqwest::Client::new();
                let mut pulls: Vec<ParsedWuwaPull> = vec![];
                for req in requests {
                    let json = req.to_json()?;
                    let res = client
                        .post(WUWA_REQ_URL)
                        .header("Content-Type", "application/json")
                        .body(json)
                        .send()
                        .await
                        .unwrap();

                    let wrapper = res.json::<DeserializeWrapper>().await.unwrap();
                    pulls.extend(wrapper.data);
                }

                let pulls_len = pulls.len();
                tracing::info!("length: {pulls_len}");

                if pulls_len > 0 {
                    let inserted = pulls_manager.insert_wuwa_pulls(player_id, pulls).await?;
                    ctx.reply(format!(
                        "Records parsed: {}, {} new records inserted into database",
                        pulls_len, inserted,
                    ))
                    .await?;
                } else {
                    ctx.reply(format!("No records found.")).await?;
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

    pub fn build(self) -> Result<Vec<WuwaRequest>, String> {
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
            Err("not enough arguments to build".to_string())
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
    pub fn to_json(self) -> Result<String, BotError> {
        Ok(serde_json::to_string(&self)?)
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
    let ti = time::PrimitiveDateTime::parse(s, FORMAT).unwrap();
    Ok(ti)
}

#[derive(Debug, Deserialize, Clone, Copy, strum::Display)]
pub enum ResourceType {
    Weapon,
    Resonator,
}

#[derive(Debug, Deserialize, Copy, Clone)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_type_1() {
        let DeserializeWrapper { data: pulls } = serde_json::from_str(
            &std::fs::read_to_string("../dev/wuwa_model_type_1.json").unwrap(),
        )
        .unwrap();

        assert!(pulls.iter().find(|e| e.name == "Carlotta").is_some());
    }

    #[test]
    fn test_deserialize_type_2() {
        let DeserializeWrapper { data: pulls } = serde_json::from_str(
            &std::fs::read_to_string("../dev/wuwa_model_type_2.json").unwrap(),
        )
        .unwrap();

        assert!(pulls.iter().find(|e| e.name == "The Last Dance").is_some());
    }

    #[test]
    fn test_deserialize_type_3() {
        let DeserializeWrapper { data: pulls } = serde_json::from_str(
            &std::fs::read_to_string("../dev/wuwa_model_type_2.json").unwrap(),
        )
        .unwrap();

        assert!(
            pulls
                .iter()
                .find(|e| e.name == "Originite: Type IV")
                .is_some()
        );
    }

    #[test]
    fn test_deserialize_type_4() {
        let DeserializeWrapper { data: pulls } = serde_json::from_str(
            &std::fs::read_to_string("../dev/wuwa_model_type_4.json").unwrap(),
        )
        .unwrap();

        assert!(pulls.iter().find(|e| e.name == "Cosmic Ripples").is_some());
    }
}
