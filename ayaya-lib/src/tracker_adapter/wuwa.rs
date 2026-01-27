use reqwest::Client;
use serde::Serialize;
use crate::{
    tracker::{CardPoolType, DeserializeWrapper, ParsedWuwaPull, error::TrackerError},
    tracker_adapter::{GameAdapter, GameId, PullRecord},
};

const WUWA_REQ_URL: &str = "https://gmserver-api.aki-game2.net/gacha/record/query";

#[derive(Debug, Clone)]
pub struct WuwaAdapter;

#[derive(Debug, Clone)]
pub struct WuwaSession {
    pub server_id: String,
    pub record_id: String,
    pub player_id: u64,
}

impl GameAdapter for WuwaAdapter {
    type Session = WuwaSession;
    type Pull = ParsedWuwaPull;
    type PoolId = CardPoolType;
    type Error = TrackerError;

    fn game_id(&self) -> GameId {
        GameId::WutheringWaves
    }

    fn display_name(&self) -> &'static str {
        "Wuthering Waves"
    }

    fn parse_link(&self, link: &str) -> Result<Self::Session, Self::Error> {
        let url = url::Url::parse(link).map_err(|_| TrackerError::InvalidUrl)?;
        let fragment = url.fragment().ok_or(TrackerError::InvalidUrl)?;
        let params = fragment.strip_prefix("/record?").unwrap_or("");

        let mut server_id = None;
        let mut record_id = None;
        let mut player_id = None;

        for part in params.split('&') {
            let (key, value): (&str, &str) = match part.split_once('=') {
                Some(pair) => pair,
                None => continue,
            };

            match key {
                "svr_id" => server_id = Some(value.to_string()),
                "record_id" => record_id = Some(value.to_string()),
                "player_id" => {
                    let parsed = value
                        .parse::<u64>()
                        .map_err(|source| TrackerError::WuwaPlayerIdInvalid { source })?;
                    player_id = Some(parsed);
                }
                _ => {}
            }
        }

        match (server_id, record_id, player_id) {
            (Some(server_id), Some(record_id), Some(player_id)) => Ok(WuwaSession {
                server_id,
                record_id,
                player_id,
            }),
            _ => Err(TrackerError::InvalidUrl),
        }
    }

    async fn fetch_pulls(
        &self,
        session: &Self::Session,
        client: &Client,
    ) -> Result<Vec<Self::Pull>, Self::Error> {
        let requests = build_requests(session);
        let mut pulls = Vec::new();

        for req in requests {
            let json = req.as_json()?;
            let res = client
                .post(WUWA_REQ_URL)
                .header("Content-Type", "application/json")
                .body(json)
                .send()
                .await
                .map_err(|source| TrackerError::WuwaRequestFailed { source })?;

            let status = res.status();
            let body = res
                .text()
                .await
                .map_err(|source| TrackerError::WuwaResponseRead { source })?;
            let wrapper = serde_json::from_str::<DeserializeWrapper>(&body).map_err(|source| {
                tracing::warn!(
                    "Wuwa response decode failed: status={}, body_snippet={}",
                    status,
                    truncate_body(&body, 500)
                );
                TrackerError::WuwaResponseDecode { source }
            })?;
            pulls.extend(wrapper.data);
        }

        Ok(pulls)
    }

    fn pool_id(&self, pull: &Self::Pull) -> Self::PoolId {
        pull.card_pool_type
    }

    fn normalize_pull(&self, pull: Self::Pull, user_game_id: &str) -> PullRecord {
        PullRecord {
            game_id: self.game_id(),
            user_game_id: user_game_id.to_string(),
            pool_id: pull.card_pool_type.to_string(),
            resource_id: Some(pull.resource_id as i64),
            resource_name: pull.name,
            resource_type: pull.resource_type.to_string(),
            quality: pull.quality_level as i32,
            count: pull.count as i32,
            time: pull.time.assume_offset(time::UtcOffset::UTC),
        }
    }
}

fn build_requests(session: &WuwaSession) -> Vec<WuwaRequest> {
    [1u8, 2, 3, 4]
        .iter()
        .map(|e| WuwaRequest {
            player_id: session.player_id,
            card_pool_type: *e,
            language_code: "en".to_string(),
            server_id: session.server_id.clone(),
            record_id: session.record_id.clone(),
        })
        .collect()
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
    fn as_json(&self) -> Result<String, TrackerError> {
        serde_json::to_string(self).map_err(|source| TrackerError::WuwaRequestEncode { source })
    }
}

fn truncate_body(body: &str, max: usize) -> &str {
    if body.len() <= max {
        body
    } else {
        &body[..max]
    }
}

impl std::fmt::Display for CardPoolType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            CardPoolType::EventCharacterConvene => "Resonators Accurate Modulation",
            CardPoolType::EventWeaponConvene => "Resonators Accurate Modulation - 2",
            CardPoolType::StandardCharacterConvene => "Weapons Accurate Modulation",
            CardPoolType::StandardWeaponConvene => "Full-Range Modualtion",
        };
        write!(f, "{label}")
    }
}
