use async_trait::async_trait;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum DataOperationType {
    Read,
    Write,
}

impl std::fmt::Display for DataOperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataOperationType::Read => write!(f, "read"),
            DataOperationType::Write => write!(f, "write"),
        }
    }
}

#[async_trait]
pub trait MetricsSink: Send + Sync + 'static {
    async fn data_access(&self, name: &str, op: DataOperationType);
    async fn data_time(&self, name: &str, op: DataOperationType, time: f64);
    async fn cache_len(&self, name: &str, len: usize);
}

#[derive(Debug, Default, Clone)]
pub struct NoopMetrics;

#[async_trait]
impl MetricsSink for NoopMetrics {
    async fn data_access(&self, _name: &str, _op: DataOperationType) {}

    async fn data_time(&self, _name: &str, _op: DataOperationType, _time: f64) {}

    async fn cache_len(&self, _name: &str, _len: usize) {}
}
