use crate::metrics::{DataOperationType, Metrics};

pub struct DataTiming {
    start: std::time::Instant,
    name: String,
    operation_type: DataOperationType,
    metric_handler: Option<Metrics>,
}

impl DataTiming {
    pub fn new(
        name: String,
        operation_type: DataOperationType,
        metric_handler: Option<Metrics>,
    ) -> Self {
        let start = std::time::Instant::now();
        Self {
            start,
            name,
            operation_type,
            metric_handler,
        }
    }
}

impl Drop for DataTiming {
    fn drop(&mut self) {
        let elapsed = std::time::Instant::now() - self.start;
        let elapsed = elapsed.as_secs_f64();
        let name = self.name.clone();
        let operation_type = self.operation_type.clone();
        tracing::debug!(
            "timed {} operation type: {} for: {} seconds",
            name,
            operation_type,
            elapsed
        );
        if let Some(metric_handler) = self.metric_handler.take() {
            tokio::spawn(async move {
                metric_handler
                    .data_time(name, operation_type, elapsed)
                    .await;
            });
        }
    }
}
