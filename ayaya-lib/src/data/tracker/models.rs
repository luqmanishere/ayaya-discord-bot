use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct GameConfig {
    pub game: GameInfo,
    pub version: VersionInfo,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GameInfo {
    pub name: String,
    pub short_name: String,
    pub color: u32,
    pub icon_url: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct VersionInfo {
    current: String,
    load_versions: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ServerInfo {
    name: String,
    daily_reset: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct GameEvent {
    pub id: String,
    pub name: String,
    pub description: String,
    // pub r#type: EventType,
    pub trackable: bool,
    pub daily_limit: bool,
    pub enabled: Option<bool>,

    // Time limited
    pub start: Option<String>,
    pub end: Option<String>,

    // Recurring
    pub day_of_week: Option<String>,
    pub day_of_month: Option<String>,
    pub time: Option<String>,

    pub rotation_group: Option<String>,
    pub servers: Vec<String>,
}
