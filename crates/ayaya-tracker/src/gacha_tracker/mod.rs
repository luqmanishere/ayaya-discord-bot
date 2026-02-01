use reqwest::Client;
use time::OffsetDateTime;

pub use self::error::TrackerError;
pub use self::types::{CardPoolType, DeserializeWrapper, ParsedWuwaPull, ResourceType};
use self::wuwa::WuwaAdapter;
pub use ayaya_core::tracker::{ImportBoundary, PullRecord};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameId {
    WutheringWaves,
}

#[derive(Debug)]
pub struct BoundaryResult<'a, T> {
    pub new_items: Vec<&'a T>,
    pub next_boundary: Option<ImportBoundary>,
}

#[allow(async_fn_in_trait)]
pub trait GameAdapter {
    type Session: Send + Sync;
    type Pull: Send + Sync;
    type PoolId: Send + Sync + Clone + Eq + std::hash::Hash + std::fmt::Display;
    type Error;

    fn game_id(&self) -> GameId;
    fn display_name(&self) -> &'static str;
    fn parse_link(&self, link: &str) -> Result<Self::Session, Self::Error>;
    async fn fetch_pulls(
        &self,
        session: &Self::Session,
        client: &Client,
    ) -> Result<Vec<Self::Pull>, Self::Error>;
    fn pool_id(&self, pull: &Self::Pull) -> Self::PoolId;
    fn normalize_pull(&self, pull: Self::Pull, user_game_id: &str) -> PullRecord;
}

/// Apply a boundary to an already-sorted pull list (descending by time).
pub fn apply_import_boundary<'a, T>(
    pulls: &'a [T],
    boundary: Option<ImportBoundary>,
    time_of: impl Fn(&T) -> OffsetDateTime,
) -> BoundaryResult<'a, T> {
    let mut new_items = Vec::new();

    if pulls.is_empty() {
        return BoundaryResult {
            new_items,
            next_boundary: boundary,
        };
    }

    let mut skipped_at_boundary = 0usize;

    for pull in pulls {
        let time = time_of(pull);
        match boundary {
            Some(b) if time > b.time => new_items.push(pull),
            Some(b) if time == b.time => {
                if skipped_at_boundary < b.count_at_time {
                    skipped_at_boundary += 1;
                } else {
                    new_items.push(pull);
                }
            }
            Some(b) if time < b.time => break,
            None => new_items.push(pull),
            _ => {}
        }
    }

    let newest_time = time_of(&pulls[0]);
    let newest_count = pulls
        .iter()
        .take_while(|pull| time_of(pull) == newest_time)
        .count();
    let next_boundary = match boundary {
        Some(b) if b.time == newest_time => Some(ImportBoundary {
            time: newest_time,
            count_at_time: b.count_at_time.max(newest_count),
        }),
        _ => Some(ImportBoundary {
            time: newest_time,
            count_at_time: newest_count,
        }),
    };

    BoundaryResult {
        new_items,
        next_boundary,
    }
}

pub mod error;
pub mod types;
pub mod wuwa;

#[derive(Debug, Clone)]
pub enum AdapterKind {
    Wuwa(WuwaAdapter),
}

pub fn adapter_for(game_id: GameId) -> AdapterKind {
    match game_id {
        GameId::WutheringWaves => AdapterKind::Wuwa(WuwaAdapter),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[derive(Debug)]
    struct Pull {
        time: OffsetDateTime,
        id: u32,
    }

    #[test]
    fn boundary_skips_existing_at_same_time() {
        let pulls = vec![
            Pull {
                time: datetime!(2025-09-01 12:00 UTC),
                id: 1,
            },
            Pull {
                time: datetime!(2025-09-01 12:00 UTC),
                id: 2,
            },
            Pull {
                time: datetime!(2025-09-01 11:59 UTC),
                id: 3,
            },
        ];
        let boundary = ImportBoundary {
            time: datetime!(2025-09-01 12:00 UTC),
            count_at_time: 1,
        };
        let result = apply_import_boundary(&pulls, Some(boundary), |p| p.time);
        assert_eq!(result.new_items.len(), 1);
        assert_eq!(result.new_items[0].id, 2);
        assert_eq!(
            result.next_boundary,
            Some(ImportBoundary {
                time: datetime!(2025-09-01 12:00 UTC),
                count_at_time: 2,
            })
        );
    }
}
