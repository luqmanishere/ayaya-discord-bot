use serde::Deserialize;
use strum::IntoEnumIterator;
use url::Url;

use crate::gacha_tracker::{AkEndPullDto, GameAdapter, TrackerError};

#[derive(Debug, Clone)]
pub struct AkEndAdapter;

#[derive(Debug, Clone)]
pub struct AkEndSession {
    u8_token: String,
    server_id: AkEndServer,
}

#[derive(Debug, Clone)]
#[repr(u64)]
pub enum AkEndServer {
    Asia = 2,
}

impl TryFrom<u8> for AkEndServer {
    type Error = TrackerError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            2 => Ok(Self::Asia),
            // TODO: proper error type
            _ => Err(TrackerError::InvalidUrl),
        }
    }
}

#[derive(Default)]
pub struct AkEndSessionBuilder {
    u8_token: Option<String>,
    server_id: Option<AkEndServer>,
}

impl AkEndSessionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_u8_token(&mut self, u8_token: String) -> &mut Self {
        self.u8_token = Some(u8_token);
        self
    }

    pub fn set_server_id(&mut self, server_id: AkEndServer) -> &mut Self {
        self.server_id = Some(server_id);
        self
    }

    pub fn build(self) -> AkEndSession {
        // TODO: return err for validation
        AkEndSession {
            u8_token: self.u8_token.unwrap(),
            server_id: self.server_id.unwrap(),
        }
    }
}

impl GameAdapter for AkEndAdapter {
    type Session = AkEndSession;

    type Pull = ParsedAkEndPull;

    type PoolId = AkEndGachaPool;

    type Dto = AkEndPullDto;

    type Error = TrackerError;

    fn game_id(&self) -> super::GameId {
        super::GameId::ArknightsEndfield
    }

    fn display_name(&self) -> &'static str {
        "Arknights Endfield"
    }

    /// What an Endfield url looks like (as of 1.0):
    /// Base: https://ef-webview.gryphline.com/page/gacha_char
    ///
    /// Params:
    /// 1. pool_id: special_1_0_1 - Likely denoting the current banner
    /// 2. u8_token: A very long per account token
    /// 3. platform: Windows - The platform the url was generated on
    /// 4. channel: 6 - TODO: idk
    /// 5. subChannel: 6 - TODO: idk
    /// 6. lang=en-us
    /// 7. server: 2 - 2 is Asia
    fn parse_link(&self, link: &str) -> Result<Self::Session, Self::Error> {
        let url = Url::parse(link).map_err(|_| TrackerError::InvalidUrl)?;
        let query_pairs = url.query_pairs();

        let mut session_builder = AkEndSessionBuilder::new();
        for (key, value) in query_pairs {
            match key {
                std::borrow::Cow::Borrowed("server_id") => {
                    session_builder.set_server_id(value.parse::<u8>().unwrap().try_into().unwrap());
                }
                std::borrow::Cow::Borrowed("u8_token") | std::borrow::Cow::Borrowed("token") => {
                    session_builder.set_u8_token(value.to_string());
                }
                _ => {}
            }
        }
        let session = session_builder.build();
        Ok(session)
    }

    #[tracing::instrument(skip(client, session))]
    async fn fetch_pulls(
        &self,
        session: &Self::Session,
        client: &reqwest::Client,
    ) -> Result<Vec<Self::Pull>, Self::Error> {
        // absolutely bonkers logic incoming

        let mut pulls = vec![];

        for pool_type in AkEndGachaPool::iter() {
            let first_req_url = build_url(session, pool_type, None);
            let first_req = client
                .get(first_req_url)
                .send()
                .await
                .map_err(|e| TrackerError::WuwaRequestFailed { source: e })?;
            tracing::debug!("First request status: {}", first_req.status());
            let json: AkEndResponseDes = first_req.json().await.unwrap();
            let mut has_more = if json.data.has_more {
                Some(json.data.list.last().unwrap().seq_id.clone())
            } else {
                None
            };

            while let Some(ref seq_id) = has_more {
                let req_url = build_url(session, pool_type, Some(seq_id.clone()));
                let req = client
                    .get(req_url)
                    .send()
                    .await
                    .map_err(|e| TrackerError::WuwaRequestFailed { source: e })?;
                let json: AkEndResponseDes = req.json().await.unwrap();
                if json.data.has_more {
                    has_more = Some(json.data.list.last().unwrap().seq_id.clone());
                } else {
                    has_more = None;
                }

                let data = json.data.list.into_iter().map(|mut e| {
                    e.pool_type = Some(pool_type);
                    e
                });

                pulls.extend(data);
            }
        }

        Ok(pulls)
    }

    fn pool_id(&self, pull: &Self::Pull) -> Self::PoolId {
        pull.pool_type.unwrap().clone()
    }

    fn normalize_pull(&self, pull: Self::Pull, user_game_id: &str) -> Self::Dto {
        let ts = parse_akend_ts(&pull.gacha_ts);
        Self::Dto {
            user_game_id: user_game_id.to_string(),
            pool_type: pull.pool_type.unwrap().get_api_name(),
            pool_id: pull.pool_id,
            pool_name: pull.pool_name,
            char_id: pull.char_id,
            char_name: pull.char_name,
            rarity: pull.rarity,
            is_free: pull.is_free,
            is_new: pull.is_new,
            time: ts,
            seq_id: pull.seq_id,
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
#[expect(dead_code)]
pub struct AkEndResponseDes {
    code: Option<u64>,
    data: DataDes,
    msg: String,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct DataDes {
    list: Vec<ParsedAkEndPull>,
    has_more: bool,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ParsedAkEndPull {
    #[serde(skip)]
    pub pool_type: Option<AkEndGachaPool>,
    pub pool_id: String,
    pub pool_name: String,
    pub char_id: String,
    pub char_name: String,
    pub rarity: i32,
    pub is_free: bool,
    pub is_new: bool,
    pub gacha_ts: String,
    pub seq_id: String,
}

#[derive(Deserialize, Debug, Copy, Clone, Hash, Eq, PartialEq, strum::EnumIter)]
pub enum AkEndGachaPool {
    #[serde(alias = "E_CharacterGachaPoolType_Special")]
    Special,
    #[serde(alias = "E_CharacterGachaPoolType_Standard")]
    Standard,
    #[serde(alias = "E_CharacterGachaPoolType_Beginner")]
    Beginner,
}

impl AkEndGachaPool {
    pub fn get_api_name(&self) -> String {
        match self {
            AkEndGachaPool::Special => "E_CharacterGachaPoolType_Special",
            AkEndGachaPool::Standard => "E_CharacterGachaPoolType_Standard",
            AkEndGachaPool::Beginner => "E_CharacterGachaPoolType_Beginner",
        }
        .to_string()
    }
}

impl std::fmt::Display for AkEndGachaPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Special => "Special",
            Self::Standard => "Standard",
            Self::Beginner => "Beginner",
        };
        write!(f, "{label}")
    }
}

fn build_url(session: &AkEndSession, pool_type: AkEndGachaPool, seq_id: Option<String>) -> String {
    static BASE_URL: &str = "https://ef-webview.gryphline.com/api/record/char";

    let mut url = Url::parse(BASE_URL).expect("valid AkEnd base url");
    {
        let mut pairs = url.query_pairs_mut();
        pairs.append_pair("lang", "en-us");
        pairs.append_pair("pool_type", &pool_type.get_api_name());
        pairs.append_pair("server_id", &(session.server_id.clone() as u64).to_string());
        pairs.append_pair("token", &session.u8_token);
        if let Some(seq_id) = seq_id {
            pairs.append_pair("seq_id", &seq_id);
        }
    }
    url.to_string()
}

fn parse_akend_ts(raw: &str) -> time::OffsetDateTime {
    let value = raw.parse::<i128>().expect("unix timestamp");
    if value > 1_000_000_000_000 {
        let nanos = value
            .checked_mul(1_000_000)
            .expect("unix timestamp nanos");
        time::OffsetDateTime::from_unix_timestamp_nanos(nanos)
            .expect("proper unix timestamp")
    } else {
        time::OffsetDateTime::from_unix_timestamp(value as i64).expect("proper unix timestamp")
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sample_response() {
        let model_path = format!("{}/../../models/akend_1_0.json", env!("CARGO_MANIFEST_DIR"));
        let body = std::fs::read_to_string(model_path).expect("sample response read");
        let response: AkEndResponseDes =
            serde_json::from_str(&body).expect("sample response parse");

        assert_eq!(response.code, Some(0));
        assert!(!response.data.list.is_empty());

        let first = &response.data.list[0];
        assert!(!first.pool_id.is_empty());
        assert!(!first.char_id.is_empty());
        assert!(!first.char_name.is_empty());
        assert!(!first.gacha_ts.is_empty());
        assert!(!first.seq_id.is_empty());
        assert!(
            response
                .data
                .list
                .iter()
                .find(|e| e.char_name == "Laevatain")
                .is_some()
        );
    }

    #[tokio::test]
    #[ignore = "requires live AkEnd HTTP endpoint"]
    async fn akend_fetch_pulls_live() {
        let _ = dotenvy::dotenv();
        let adapter = AkEndAdapter;
        let token = std::env::var("AKE_TOKEN").expect("AKE_TOKEN not set");
        let mut session = AkEndSessionBuilder::new();
        session.set_u8_token(token).set_server_id(AkEndServer::Asia);
        let session = session.build();

        let client = reqwest::Client::new();
        let pulls = adapter
            .fetch_pulls(&session, &client)
            .await
            .expect("fetch pulls");

        let user_game_id = "12345";

        let pulls = pulls
            .iter()
            .map(|e| adapter.normalize_pull(e.clone(), user_game_id))
            .collect::<Vec<_>>();

        dbg!(&pulls);
        dbg!(pulls.len());
        assert!(!pulls.is_empty());
    }
}
