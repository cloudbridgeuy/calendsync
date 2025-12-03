//! SSR worker thread management.
//!
//! Each worker runs in a dedicated thread with its own Tokio runtime
//! because `deno_core::JsRuntime` is not `Send`.

use std::sync::Arc;

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
pub struct SsrWorker {
    request_tx: mpsc::Sender<RenderRequest>,
}

impl SsrWorker {
    /// Spawn a new worker thread.
    ///
    /// This is an I/O operation that spawns an OS thread.
    pub fn spawn(bundle_code: Arc<String>, config: Arc<SsrPoolConfig>) -> Self {
        let (request_tx, mut request_rx) = mpsc::channel::<RenderRequest>(config.max_pending);

        std::thread::spawn(move || {
            // Create a single-threaded Tokio runtime for this worker
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create Tokio runtime for SSR worker");

            rt.block_on(async move {
                tracing::debug!("SSR worker started");

                while let Some(req) = request_rx.recv().await {
                    let result =
                        runtime::render(&bundle_code, &req.config_json, &config.node_env).await;

                    // Send result back, ignoring if receiver dropped
                    let _ = req.response_tx.send(result);
                }

                tracing::debug!("SSR worker shutting down");
            });
        });

        Self { request_tx }
    }

    /// Check if the worker has capacity for more requests.
    pub fn has_capacity(&self) -> bool {
        self.request_tx.capacity() > 0
    }

    /// Get a clone of the sender for sending requests.
    pub fn sender(&self) -> mpsc::Sender<RenderRequest> {
        self.request_tx.clone()
    }
}
