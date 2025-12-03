//! JsRuntime execution for React SSR.
//!
//! This module contains the impure side-effect code that executes
//! JavaScript using deno_core's JsRuntime.

use std::cell::RefCell;

use calendsync_ssr_core::generate_polyfills;
use deno_core::{extension, op2, JsRuntime, RuntimeOptions};

use crate::error::{Result, SsrError};

thread_local! {
    /// Thread-local storage for rendered HTML output.
    /// This is used because deno_core ops can't easily return complex values.
    static RENDERED_HTML: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Custom op to receive rendered HTML from JavaScript.
#[op2(fast)]
fn op_set_html(#[string] html: String) {
    RENDERED_HTML.with(|cell| {
        *cell.borrow_mut() = Some(html);
    });
}

extension!(ssr_ext, ops = [op_set_html]);

/// Execute React SSR and return rendered HTML.
///
/// **MUST be called from a dedicated thread** - `JsRuntime` is not `Send`.
/// Uses pure `generate_polyfills` from core crate.
pub async fn render(bundle_code: &str, config_json: &str, node_env: &str) -> Result<String> {
    // Pure function call from core - generates polyfills string
    let polyfills = generate_polyfills(config_json, node_env).map_err(SsrError::Core)?;

    // Impure: Create and execute JsRuntime
    let mut runtime = JsRuntime::new(RuntimeOptions {
        extensions: vec![ssr_ext::init()],
        ..Default::default()
    });

    // Execute polyfills
    runtime
        .execute_script("<polyfills>", polyfills)
        .map_err(|e| SsrError::JsExecution(e.to_string()))?;

    // Execute React SSR bundle
    runtime
        .execute_script("<react-ssr>", bundle_code.to_string())
        .map_err(|e| SsrError::JsExecution(e.to_string()))?;

    // Run event loop to completion (handles async React rendering)
    runtime
        .run_event_loop(Default::default())
        .await
        .map_err(|e| SsrError::JsExecution(e.to_string()))?;

    // Extract rendered HTML from thread-local storage
    RENDERED_HTML
        .with(|cell| cell.borrow_mut().take())
        .ok_or(SsrError::NoHtmlRendered)
}

/// Clear thread-local HTML storage.
/// Useful for testing or resetting worker state.
#[allow(dead_code)]
pub fn clear_rendered_html() {
    RENDERED_HTML.with(|cell| {
        *cell.borrow_mut() = None;
    });
}
