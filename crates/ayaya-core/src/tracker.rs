use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportBoundary {
    pub time: OffsetDateTime,
    pub count_at_time: usize,
}

pub mod wuwa {
    use time::OffsetDateTime;

    #[derive(Debug, Clone)]
    pub struct WuwaPullDto {
        pub pool_id: String,
        pub resource_id: i64,
        pub resource_name: String,
        pub resource_type: String,
        pub quality: i32,
        pub count: i32,
        pub time: OffsetDateTime,
    }
}

pub mod akend {
    use time::OffsetDateTime;

    #[derive(Debug, Clone)]
    pub enum AkEndPullDto {
        Character(AkEndCharPullDto),
        Weapon(AkEndWeapPullDto),
    }

    #[derive(Debug, Clone)]
    pub struct AkEndCharPullDto {
        pub user_game_id: String,
        pub pool_type: String,
        pub pool_id: String,
        pub pool_name: String,
        pub char_id: String,
        pub char_name: String,
        pub rarity: i32,
        pub is_free: bool,
        pub is_new: bool,
        pub time: OffsetDateTime,
        pub seq_id: String,
    }

    #[derive(Debug, Clone)]
    pub struct AkEndWeapPullDto {
        pub user_game_id: String,
        pub pool_type: String,
        pub pool_id: String,
        pub pool_name: String,
        pub weapon_id: String,
        pub weapon_name: String,
        pub weapon_type: String,
        pub rarity: i32,
        pub is_new: bool,
        pub time: OffsetDateTime,
        pub seq_id: String,
    }
}
