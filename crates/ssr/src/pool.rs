//! SSR worker pool for concurrent rendering.
//!
//! The pool manages multiple worker threads, distributing render requests
//! using round-robin scheduling with backpressure support.

use std::ffi::OsStr;
use std::path::Path;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use calendsync_ssr_core::{SsrConfig, SsrPoolConfig};
use serde::Serialize;
use tokio::sync::oneshot;

use crate::{
    error::{Result, SsrError},
    worker::{RenderRequest, SsrWorker},
};

/// A pool of SSR workers for concurrent rendering.
pub struct SsrPool {
    workers: Vec<SsrWorker>,
    next_worker: AtomicUsize,
    config: Arc<SsrPoolConfig>,
}

impl SsrPool {
    /// Create a new SSR pool.
    ///
    /// This is an I/O operation that:
    /// - Reads the bundle from disk
    /// - Spawns worker threads
    pub fn new(config: SsrPoolConfig, bundle_path: &Path) -> Result<Self> {
        // Validate bundle path
        let canonical = bundle_path
            .canonicalize()
            .map_err(|e| SsrError::BundleLoad {
                path: bundle_path.display().to_string(),
                reason: e.to_string(),
            })?;

        // Ensure it's a .js file
        if canonical.extension() != Some(OsStr::new("js")) {
            return Err(SsrError::BundleLoad {
                path: bundle_path.display().to_string(),
                reason: "Bundle must be a .js file".to_string(),
            });
        }

        // Read bundle from disk
        let bundle_code =
            std::fs::read_to_string(&canonical).map_err(|e| SsrError::BundleLoad {
                path: canonical.display().to_string(),
                reason: e.to_string(),
            })?;

        let bundle_code = Arc::new(bundle_code);
        let config = Arc::new(config);

        // Spawn worker threads
        let workers: Vec<_> = (0..config.worker_count)
            .map(|_| SsrWorker::spawn(Arc::clone(&bundle_code), Arc::clone(&config)))
            .collect();

        tracing::info!(
            worker_count = workers.len(),
            bundle_path = %canonical.display(),
            "SSR pool initialized"
        );

        Ok(Self {
            workers,
            next_worker: AtomicUsize::new(0),
            config,
        })
    }

    /// Create pool and warm up all workers.
    ///
    /// Sends a minimal render request to each worker to ensure they're ready.
    pub async fn new_with_warmup(config: SsrPoolConfig, bundle_path: &Path) -> Result<Self> {
        let pool = Self::new(config, bundle_path)?;

        tracing::info!("Warming up {} SSR workers...", pool.workers.len());

        let warmup_config = SsrConfig::new(serde_json::json!({
            "initialData": {
                "warmup": true,
                "calendarId": "warmup",
                "highlightedDay": "2024-01-01",
                "days": [],
                "clientBundleUrl": "",
                "controlPlaneUrl": ""
            }
        }))
        .map_err(SsrError::Core)?;

        for i in 0..pool.workers.len() {
            match pool.render(warmup_config.clone()).await {
                Ok(_) => tracing::debug!(worker = i, "Worker warmed up"),
                Err(e) => tracing::warn!(worker = i, error = %e, "Worker warmup failed"),
            }
        }

        tracing::info!("SSR pool warm-up complete");
        Ok(pool)
    }

    /// Render HTML using the SSR pool.
    ///
    /// Uses round-robin scheduling to distribute requests across workers.
    /// Returns `Overloaded` error if no workers have capacity.
    pub async fn render(&self, config: SsrConfig) -> Result<String> {
        // Check capacity before queueing (backpressure)
        let available = self.workers.iter().filter(|w| w.has_capacity()).count();
        if available == 0 {
            return Err(SsrError::Overloaded {
                retry_after_secs: 5,
            });
        }

        // Serialize config (pure operation from core)
        let config_json = config.to_json().map_err(SsrError::Core)?;

        let (response_tx, response_rx) = oneshot::channel();

        // Round-robin worker selection
        let worker_idx = self.next_worker.fetch_add(1, Ordering::Relaxed) % self.workers.len();
        let worker = &self.workers[worker_idx];

        // Send request to worker
        worker
            .sender()
            .send(RenderRequest {
                config_json,
                response_tx,
            })
            .await
            .map_err(|_| SsrError::ChannelClosed)?;

        // Wait for response with timeout
        let timeout = tokio::time::Duration::from_millis(self.config.render_timeout_ms);
        match tokio::time::timeout(timeout, response_rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(SsrError::ChannelClosed),
            Err(_) => Err(SsrError::Timeout(self.config.render_timeout_ms)),
        }
    }

    /// Get pool statistics (passive - no I/O).
    pub fn stats(&self) -> SsrPoolStats {
        SsrPoolStats {
            worker_count: self.workers.len(),
            workers_with_capacity: self.workers.iter().filter(|w| w.has_capacity()).count(),
        }
    }

    /// Active health check - verifies workers can process requests.
    ///
    /// Sends a minimal render request and checks for response within timeout.
    /// Returns `Ok(HealthStatus)` with latency if healthy.
    pub async fn health_check(&self) -> Result<HealthStatus> {
        let start = std::time::Instant::now();

        // Minimal config that renders quickly
        let probe_config = SsrConfig::new(serde_json::json!({
            "initialData": {
                "healthCheck": true,
                "calendarId": "health-probe",
                "highlightedDay": "2024-01-01",
                "days": [],
                "clientBundleUrl": "",
                "controlPlaneUrl": ""
            }
        }))
        .map_err(SsrError::Core)?;

        // Use shorter timeout for health checks (5s)
        let health_timeout = std::time::Duration::from_millis(5000);
        let config_json = probe_config.to_json().map_err(SsrError::Core)?;

        let (response_tx, response_rx) = oneshot::channel();
        let worker_idx = self.next_worker.fetch_add(1, Ordering::Relaxed) % self.workers.len();
        let worker = &self.workers[worker_idx];

        worker
            .sender()
            .send(RenderRequest {
                config_json,
                response_tx,
            })
            .await
            .map_err(|_| SsrError::ChannelClosed)?;

        match tokio::time::timeout(health_timeout, response_rx).await {
            Ok(Ok(Ok(_html))) => {
                let latency = start.elapsed();
                Ok(HealthStatus {
                    healthy: true,
                    latency_ms: latency.as_millis() as u64,
                    worker_idx,
                    stats: self.stats(),
                    error: None,
                })
            }
            Ok(Ok(Err(e))) => Ok(HealthStatus {
                healthy: false,
                latency_ms: start.elapsed().as_millis() as u64,
                worker_idx,
                stats: self.stats(),
                error: Some(e.to_string()),
            }),
            Ok(Err(_)) => Err(SsrError::ChannelClosed),
            Err(_) => Err(SsrError::Timeout(5000)),
        }
    }
}

/// Pool statistics (passive data).
#[derive(Debug, Clone, Serialize)]
pub struct SsrPoolStats {
    pub worker_count: usize,
    pub workers_with_capacity: usize,
}

/// Health check result.
#[derive(Debug, Clone, Serialize)]
pub struct HealthStatus {
    pub healthy: bool,
    pub latency_ms: u64,
    pub worker_idx: usize,
    pub stats: SsrPoolStats,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
