//! Module containing all metrics related items
use std::sync::Arc;

use prometheus_client::{
    encoding::{EncodeLabelSet, EncodeLabelValue},
    metrics::{counter::Counter, family::Family, gauge::Gauge, histogram::Histogram, info::Info},
    registry::Registry,
};
use strum::Display;
use tokio::sync::Mutex;

/// This struct is safe to clone, as it uses [`Arc`] underneath
#[derive(Clone, Debug)]
pub struct Metrics {
    /// Command call counter
    command_calls: Arc<Mutex<Family<CommandLabel, Counter>>>,
    /// Data access counter
    data_calls: Arc<Mutex<Family<DataAccessLabel, Counter>>>,
    /// Gauge tracking cache items
    cache_items: Arc<Mutex<Family<CacheLabel, Gauge>>>,
    /// Error counter
    errors: Arc<Mutex<Family<ErrorLabel, Counter>>>,
    // Data access latency
    data_access_latency: Arc<Mutex<Family<DataAccessLabel, Histogram>>>,
}

impl Metrics {
    /// Create a new instance of [`Metrics`]
    pub fn new() -> Self {
        Self {
            command_calls: Default::default(),
            data_calls: Default::default(),
            cache_items: Default::default(),
            errors: Default::default(),
            data_access_latency: Arc::new(Mutex::new(Family::new_with_constructor(|| {
                let data_time_buckets = [
                    // Default values from go client(https://github.com/prometheus/client_golang/blob/5d584e2717ef525673736d72cd1d12e304f243d7/prometheus/histogram.go#L68)
                    0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
                ];
                Histogram::new(data_time_buckets.iter().cloned())
            }))),
        }
    }

    /// Register the metrics tracked into the provided registry
    pub async fn register_metrics(&self, registry: Arc<Mutex<Registry>>) {
        let mut registry = registry.lock().await;

        // Static Infos
        let metric_version = Info::new([("metrics_version", "1.0.0")]);
        registry.register("versioning", "Metrics version", metric_version);
        let build_git_info = build_git_info();
        registry.register("build_git", "Build information", build_git_info);
        let build_crate_info = build_crate_info();
        registry.register("build_crate", "Build crate information", build_crate_info);
        let build_rustc_info = build_rustc_info();
        registry.register("build_rustc", "Build rustc information", build_rustc_info);

        let command_calls_metric = self.command_calls.lock().await;
        registry.register(
            "command_calls",
            "Count of command calls",
            command_calls_metric.clone(),
        );

        let data_calls_metrics = self.data_calls.lock().await;
        registry.register(
            "data_accesses",
            "Count of data read and write",
            data_calls_metrics.clone(),
        );

        let cache_metrics = self.cache_items.lock().await;
        registry.register("cache", "Amount of items in cache", cache_metrics.clone());

        let error_metrics = self.errors.lock().await;
        registry.register("errors", "Count of errors", error_metrics.clone());

        let data_time_metrics = self.data_access_latency.lock().await;
        registry.register(
            "data_access_latency",
            "Histogram of time consumed getting data",
            data_time_metrics.clone(),
        );
    }

    /// Increase the counter for a command
    pub async fn increase_command_call_counter(&self, command_name: String) {
        let metric = self.command_calls.lock().await;
        metric.get_or_create(&CommandLabel { command_name }).inc();
    }

    /// Increase the counter for data access
    pub async fn data_access(
        &self,
        data_operation_name: impl ToString,
        data_operation_type: DataOperationType,
    ) {
        let data_operation_name = data_operation_name.to_string();
        let metric = self.data_calls.lock().await;
        metric
            .get_or_create(&DataAccessLabel {
                data_operation_name,
                data_operation_type,
            })
            .inc();
    }

    /// Update the current number of items in the specified cache
    pub async fn cache_len(&self, cache_name: impl ToString, len: usize) {
        let cache_name = cache_name.to_string();
        let metric = self.cache_items.lock().await;
        metric
            .get_or_create(&CacheLabel { cache_name })
            .set(len as i64);
    }

    pub async fn error(&self, error_name: impl ToString, error_type: ErrorType) {
        let error_name = error_name.to_string();
        let metric = self.errors.lock().await;
        metric
            .get_or_create(&ErrorLabel {
                error_name,
                error_type,
            })
            .inc();
    }

    pub async fn data_time(
        &self,
        data_operation_name: impl ToString,
        data_operation_type: DataOperationType,
        time: f64,
    ) {
        let data_operation_name = data_operation_name.to_string();
        let metric = self.data_access_latency.lock().await;
        metric
            .get_or_create(&DataAccessLabel {
                data_operation_name,
                data_operation_type,
            })
            .observe(time);
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct CommandLabel {
    command_name: String,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct DataAccessLabel {
    pub data_operation_name: String,
    pub data_operation_type: DataOperationType,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, EncodeLabelValue, Display)]
pub enum DataOperationType {
    Read,
    Write,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct CacheLabel {
    pub cache_name: String,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ErrorLabel {
    pub error_name: String,
    pub error_type: ErrorType,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, EncodeLabelValue)]
pub enum ErrorType {
    Command,
}

/// Build data from git
fn build_git_info<'a>() -> Info<Vec<(&'a str, &'a str)>> {
    let mut infos = vec![];

    if let Some(git_sha) = option_env!("VERGEN_GIT_SHA") {
        infos.push(("git_sha", git_sha));
    }

    if let Some(git_branch) = option_env!("VERGEN_GIT_BRANCH") {
        infos.push(("git_branch", git_branch));
    }

    if let Some(git_commit_timestamp) = option_env!("VERGEN_GIT_COMMIT_TIMESTAMP") {
        infos.push(("git_commit_timestamp", git_commit_timestamp));
    }

    if let Some(git_describe) = option_env!("VERGEN_GIT_DESCRIBE") {
        infos.push(("git_describe", git_describe));
    }

    Info::new(infos)
}

/// Build data from the crate
fn build_crate_info<'a>() -> Info<Vec<(&'a str, &'a str)>> {
    let mut infos = vec![];

    infos.push(("crate_version", env!("CARGO_PKG_VERSION")));

    if let Some(build_date) = option_env!("VERGEN_BUILD_DATE") {
        infos.push(("build_date", build_date));
    }

    if let Some(build_timestamp) = option_env!("VERGEN_BUILD_TIMESTAMP") {
        infos.push(("build_timestamp", build_timestamp));
    }

    Info::new(infos)
}

/// Build data from rustc
fn build_rustc_info<'a>() -> Info<Vec<(&'a str, &'a str)>> {
    let mut infos = vec![];

    if let Some(rustc_channel) = option_env!("VERGEN_RUSTC_CHANNEL") {
        infos.push(("rustc_channel", rustc_channel));
    }

    if let Some(rustc_commit_date) = option_env!("VERGEN_RUSTC_COMMIT_DATE") {
        infos.push(("rustc_commit_date", rustc_commit_date));
    }

    if let Some(rustc_commit_hash) = option_env!("VERGEN_RUSTC_COMMIT_HASH") {
        infos.push(("rustc_commit_hash", rustc_commit_hash));
    }

    if let Some(rustc_host_triple) = option_env!("VERGEN_RUSTC_HOST_TRIPLE") {
        infos.push(("rustc_host_triple", rustc_host_triple));
    }

    if let Some(rustc_llvm_version) = option_env!("VERGEN_RUSTC_LLVM_VERSION") {
        infos.push(("rustc_llvm_version", rustc_llvm_version));
    }

    if let Some(rustc_semver) = option_env!("VERGEN_RUSTC_SEMVER") {
        infos.push(("rustc_semver", rustc_semver));
    }

    Info::new(infos)
}
