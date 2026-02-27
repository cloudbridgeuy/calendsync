# Fix Dev Annotations Compilation Errors

> **For Claude:** REQUIRED SUB-SKILL: Use executing-plans to implement this plan task-by-task.

**Goal:** Make the codebase compile without errors or warnings across all valid feature combinations, specifically when `dev-annotations` is disabled.

**Architecture:** The `handlers/dev/annotations.rs` and `handlers/dev/sessions.rs` modules depend on `get_store()` which requires the `dev-annotations` feature. Gate those modules and their route registration behind the same feature flag. Fix 3 independent type-inference issues in the handler code.

**Tech Stack:** Rust, axum, cfg feature gates

---

## Task 1: Gate annotation and session handler modules

**File:** `crates/calendsync/src/handlers/dev/mod.rs`

**Why:** `annotations.rs` and `sessions.rs` import `get_store` which is behind `#[cfg(feature = "dev-annotations")]`. Without the same gate on the modules, they fail to compile when the feature is off.

**Change the full file to:**

```rust
//! Development-only handlers for hot-reload and UI annotations.
//!
//! Only available when `DEV_MODE` environment variable is set.

#[cfg(feature = "dev-annotations")]
pub mod annotations;
pub mod hot_reload;
#[cfg(feature = "dev-annotations")]
pub mod sessions;
pub mod types;

/// Extract the dev store from state, returning 500 if not initialized.
///
/// Shared by annotation and session handlers.
#[cfg(feature = "dev-annotations")]
pub(crate) fn get_store(
    state: &crate::state::AppState,
) -> Result<&std::sync::Arc<crate::dev::DevAnnotationStore>, crate::handlers::AppError> {
    state.dev_store.as_ref().ok_or_else(|| {
        crate::handlers::AppError(anyhow::anyhow!("dev annotation store not initialized"))
    })
}
```

**Verify:** `cargo check -p calendsync --no-default-features --features inmemory,memory 2>&1 | grep "error\[E0432\]"` should produce no output (the unresolved import errors are gone).

## Task 2: Gate annotation/session routes in app.rs

**File:** `crates/calendsync/src/app.rs`

**Why:** The route registration at lines 103-141 imports from `annotations` and `sessions` modules. When those modules are gated (Task 1), these imports fail without a matching gate.

**Replace lines 100-145** (the `#[cfg(debug_assertions)]` block) with:

```rust
    // Dev-only routes (when DEV_MODE is set and debug build)
    #[cfg(debug_assertions)]
    if std::env::var("DEV_MODE").is_ok() {
        use crate::handlers::dev::hot_reload::{dev_events_sse, reload_ssr, report_build_error};
        router = router
            // Hot-reload endpoints
            .route("/_dev/reload", post(reload_ssr))
            .route("/_dev/events", get(dev_events_sse))
            .route("/_dev/error", post(report_build_error));

        #[cfg(feature = "dev-annotations")]
        {
            use crate::handlers::dev::{
                annotations::{
                    acknowledge_annotation, add_thread_message, clear_annotations,
                    create_annotation, delete_annotation, dev_annotations_sse,
                    dismiss_annotation, get_annotation, list_annotations,
                    list_pending_annotations, resolve_annotation, watch_annotations,
                },
                sessions::{close_session, get_session, list_sessions},
            };
            router = router
                // Annotation CRUD
                .route(
                    "/_dev/annotations",
                    get(list_annotations)
                        .post(create_annotation)
                        .delete(clear_annotations),
                )
                .route("/_dev/annotations/pending", get(list_pending_annotations))
                .route("/_dev/annotations/watch", get(watch_annotations))
                .route("/_dev/annotations/events", get(dev_annotations_sse))
                .route(
                    "/_dev/annotations/{id}",
                    get(get_annotation).delete(delete_annotation),
                )
                .route(
                    "/_dev/annotations/{id}/acknowledge",
                    patch(acknowledge_annotation),
                )
                .route("/_dev/annotations/{id}/resolve", patch(resolve_annotation))
                .route("/_dev/annotations/{id}/dismiss", patch(dismiss_annotation))
                .route("/_dev/annotations/{id}/thread", post(add_thread_message))
                // Session endpoints
                .route("/_dev/sessions", get(list_sessions))
                .route("/_dev/sessions/{id}", get(get_session))
                .route("/_dev/sessions/{id}/close", patch(close_session));
        }

        tracing::info!(
            "Dev mode enabled: /_dev/reload, /_dev/events, /_dev/error endpoints available"
        );
        #[cfg(feature = "dev-annotations")]
        tracing::info!("Dev annotations enabled: /_dev/annotations, /_dev/sessions endpoints available");
    }
```

**Verify:** `cargo check -p calendsync --no-default-features --features inmemory,memory 2>&1 | grep "error"` should produce no errors.

## Task 3: Fix type inference in `get_annotation`

**File:** `crates/calendsync/src/handlers/dev/annotations.rs:198`

**Why:** `Json(ann)` can't infer the type parameter `T` for `Json<T>`.

**Replace:**
```rust
        Some(ann) => Ok(Json(ann).into_response()),
```

**With:**
```rust
        Some(ann) => Ok(Json::<DevAnnotation>(ann).into_response()),
```

**Verify:** `cargo check -p calendsync 2>&1 | grep "E0282"` should produce no output.

## Task 4: Fix never-type fallback warnings

**File:** `crates/calendsync/src/handlers/dev/annotations.rs`

**Why:** Functions returning `Result<impl IntoResponse, AppError>` that use `serde_json::json!()` trigger `dependency_on_unit_never_type_fallback` warnings. These will become hard errors in Rust 2024 edition.

**4a** — `list_pending_annotations` (line 186): Replace:
```rust
    let annotations = store.list_pending().await?;
```
With:
```rust
    let annotations: Vec<DevAnnotation> = store.list_pending().await?;
```

**4b** — `clear_annotations` (line 287): Replace:
```rust
    let count = store.clear_all().await?;
```
With:
```rust
    let count: usize = store.clear_all().await?;
```

**4c** — `watch_annotations` — two bindings. Replace line 337:
```rust
    let pending = store.list_pending().await?;
```
With:
```rust
    let pending: Vec<DevAnnotation> = store.list_pending().await?;
```
And replace line 366:
```rust
    let pending = store.list_pending().await?;
```
With:
```rust
    let pending: Vec<DevAnnotation> = store.list_pending().await?;
```

**File:** `crates/calendsync/src/handlers/dev/sessions.rs`

**4d** — `list_sessions` (line 19): Replace:
```rust
    let sessions = store.list_sessions().await?;
```
With:
```rust
    let sessions: Vec<super::types::DevSession> = store.list_sessions().await?;
```

**4e** — `get_session` (line 33): Replace:
```rust
            let annotations = store.list_by_session(&id).await?;
```
With:
```rust
            let annotations: Vec<super::types::DevAnnotation> = store.list_by_session(&id).await?;
```

**Verify:** `cargo check -p calendsync 2>&1 | grep "dependency_on_unit_never_type_fallback"` should produce no output.

## Task 5: Fix unused `mut` warning in main.rs

**File:** `crates/calendsync/src/main.rs:63`

**Why:** `let mut state` is only mutated inside the `#[cfg(feature = "dev-annotations")]` block (line 73: `state.set_dev_store()`). Without the feature, `mut` is unused.

**Replace line 63:**
```rust
    let mut state = AppState::new(&config).await?;
```

**With:**
```rust
    let state = AppState::new(&config).await?;
```

**Then replace lines 66-78** (the cfg block) with:
```rust
    // Initialize dev annotation store when in dev mode
    #[cfg(feature = "dev-annotations")]
    let state = {
        let mut state = state;
        if std::env::var("DEV_MODE").is_ok() {
            // Ensure the parent directory exists
            if let Some(parent) = std::path::Path::new(&config.dev_annotations_db_path).parent() {
                std::fs::create_dir_all(parent)?;
            }
            let dev_store = dev::DevAnnotationStore::new(&config.dev_annotations_db_path).await?;
            state.set_dev_store(dev_store);
            tracing::info!(
                path = %config.dev_annotations_db_path,
                "Dev annotation store initialized"
            );
        }
        state
    };
```

**Verify:** `cargo check -p calendsync --no-default-features --features inmemory,memory 2>&1 | grep "unused_mut"` should produce no output.

## Task 6: Full verification

Run both feature combinations and the lint suite:

```bash
# Without dev-annotations (the broken path)
cargo check -p calendsync --no-default-features --features inmemory,memory

# With default features (should still work)
cargo check -p calendsync

# Full lint suite
cargo xtask lint
```

**Expected:** All three commands pass with zero errors and zero warnings.
