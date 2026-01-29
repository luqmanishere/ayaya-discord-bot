use serde::{Deserialize, Deserializer};

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

    let value: &str = Deserialize::deserialize(deserializer)?;
    let parsed = time::PrimitiveDateTime::parse(value, FORMAT).expect("format proper");
    Ok(parsed)
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub enum ResourceType {
    Weapon,
    Resonator,
    Item,
}

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            ResourceType::Weapon => "Weapon",
            ResourceType::Resonator => "Resonator",
            ResourceType::Item => "Item",
        };
        write!(f, "{label}")
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_type_1() {
        let model_path = format!(
            "{}/../../dev/wuwa_model_type_1.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let DeserializeWrapper { data: pulls } =
            serde_json::from_str(&std::fs::read_to_string(model_path).unwrap()).unwrap();

        assert!(pulls.iter().find(|e| e.name == "Carlotta").is_some());
    }

    #[test]
    fn test_deserialize_type_2() {
        let model_path = format!(
            "{}/../../dev/wuwa_model_type_2.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let DeserializeWrapper { data: pulls } =
            serde_json::from_str(&std::fs::read_to_string(model_path).unwrap()).unwrap();

        assert!(pulls.iter().find(|e| e.name == "The Last Dance").is_some());
    }

    #[test]
    fn test_deserialize_type_3() {
        let model_path = format!(
            "{}/../../dev/wuwa_model_type_3.json",
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
            "{}/../../dev/wuwa_model_type_4.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let DeserializeWrapper { data: pulls } =
            serde_json::from_str(&std::fs::read_to_string(model_path).unwrap()).unwrap();

        assert!(pulls.iter().find(|e| e.name == "Cosmic Ripples").is_some());
    }
}
