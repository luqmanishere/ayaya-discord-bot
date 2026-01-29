use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportBoundary {
    pub time: OffsetDateTime,
    pub count_at_time: usize,
}

#[derive(Debug, Clone)]
pub struct PullRecord {
    pub pool_id: String,
    pub resource_id: Option<i64>,
    pub resource_name: String,
    pub resource_type: String,
    pub quality: i32,
    pub count: i32,
    pub time: OffsetDateTime,
}
