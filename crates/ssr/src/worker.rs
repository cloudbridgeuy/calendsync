//! SSR worker thread management.
//!
//! Each worker runs in a dedicated thread with its own Tokio runtime
//! because `deno_core::JsRuntime` is not `Send`.

use std::sync::Arc;
use std::thread::JoinHandle;

use calendsync_ssr_core::SsrPoolConfig;
use tokio::sync::{mpsc, oneshot};

use crate::{error::SsrError, runtime};

/// Request sent to a worker for rendering.
pub struct RenderRequest {
    /// Serialized SSR config JSON.
    pub config_json: String,
    /// Channel to send the result back.
    pub response_tx: oneshot::Sender<Result<String, SsrError>>,
}

/// A dedicated SSR worker thread.
///
/// Each worker runs in its own OS thread with a single-threaded Tokio runtime
/// to handle the deno_core JsRuntime which is not Send.
///
/// # Shutdown Behavior
///
/// When the worker is dropped, it sends a shutdown signal and attempts a
/// non-blocking join with a timeout. This ensures graceful shutdown while
/// not blocking the async runtime.
pub struct SsrWorker {
    request_tx: mpsc::Sender<RenderRequest>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    handle: Option<JoinHandle<()>>,
}

impl SsrWorker {
    /// Spawn a new worker thread.
    ///
    /// This is an I/O operation that spawns an OS thread.
    pub fn spawn(bundle_code: Arc<String>, config: Arc<SsrPoolConfig>) -> Self {
        let (request_tx, mut request_rx) = mpsc::channel::<RenderRequest>(config.max_pending);
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel::<()>();

        let handle = std::thread::spawn(move || {
            // Create a single-threaded Tokio runtime for this worker
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime for SSR worker");

            rt.block_on(async move {
                tracing::debug!("SSR worker started");

                loop {
                    tokio::select! {
                        // Prioritize shutdown signal
                        biased;

                        _ = &mut shutdown_rx => {
                            tracing::debug!("SSR worker received shutdown signal");
                            break;
                        }

                        req = request_rx.recv() => {
                            match req {
                                Some(req) => {
                                    let result =
                                        runtime::render(&bundle_code, &req.config_json, &config.node_env).await;

                                    // Send result back, ignoring if receiver dropped
                                    let _ = req.response_tx.send(result);
                                }
                                None => {
                                    // Channel closed, exit
                                    tracing::debug!("SSR worker channel closed");
                                    break;
                                }
                            }
                        }
                    }
                }

                tracing::debug!("SSR worker shutting down");
            });
        });

        Self {
            request_tx,
            shutdown_tx: Some(shutdown_tx),
            handle: Some(handle),
        }
    }

    /// Check if the worker has capacity for more requests.
    pub fn has_capacity(&self) -> bool {
        self.request_tx.capacity() > 0
    }

    /// Get a clone of the sender for sending requests.
    pub fn sender(&self) -> mpsc::Sender<RenderRequest> {
        self.request_tx.clone()
    }

    /// Check if the worker thread has finished.
    #[cfg(test)]
    pub fn is_finished(&self) -> bool {
        self.handle.as_ref().is_none_or(|h| h.is_finished())
    }
}

impl Drop for SsrWorker {
    fn drop(&mut self) {
        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }

        // Non-blocking join with timeout
        // Spawn a thread to avoid blocking async runtime
        if let Some(handle) = self.handle.take() {
            std::thread::spawn(move || {
                // Give worker 100ms to finish gracefully
                let start = std::time::Instant::now();
                let timeout = std::time::Duration::from_millis(100);

                while !handle.is_finished() && start.elapsed() < timeout {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }

                // Join if finished, otherwise let OS clean up
                if handle.is_finished() {
                    let _ = handle.join();
                } else {
                    tracing::warn!("SSR worker did not terminate within 100ms, abandoning");
                }
            });
        }
    }
}
