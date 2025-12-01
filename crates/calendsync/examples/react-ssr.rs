//! React SSR Example with Hydration using Deno Core and Axum
//!
//! This example demonstrates:
//! - Server-side rendering React with `deno_core` and `renderToString`
//! - Client-side hydration with `hydrateRoot`
//! - Custom fetch op for making HTTP requests from JavaScript
//! - Axum web server to serve SSR HTML and client bundle
//! - Automatic TypeScript compilation at startup
//!
//! # Running
//! ```bash
//! cargo run --example react-ssr -p calendsync
//! ```
//!
//! Then open http://localhost:3001 in your browser.
//! The counter (initialized from London's temperature) should be interactive.

use std::cell::RefCell;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use axum::http::header;
use axum::response::Html;
use axum::routing::get;
use axum::{Json, Router};
use deno_core::{extension, op2, JsRuntime, RuntimeOptions};
use rand::Rng;
use serde_json::{json, Value};
use tokio::sync::oneshot;

// Cache the bundles in memory after first read
static SERVER_BUNDLE: OnceLock<String> = OnceLock::new();
static CLIENT_BUNDLE: OnceLock<String> = OnceLock::new();

// Custom error type for fetch operations that implements JsErrorClass
#[derive(Debug, thiserror::Error, deno_error::JsError)]
#[class(generic)]
#[error("{0}")]
struct FetchError(String);

impl From<reqwest::Error> for FetchError {
    fn from(err: reqwest::Error) -> Self {
        FetchError(err.to_string())
    }
}

// Thread-local storage for the rendered HTML (used within SSR thread)
thread_local! {
    static RENDERED_HTML: RefCell<Option<String>> = const { RefCell::new(None) };
}

// Custom op to receive HTML from JavaScript
#[op2(fast)]
fn op_set_html(#[string] html: String) {
    RENDERED_HTML.with(|cell| {
        *cell.borrow_mut() = Some(html);
    });
}

// Custom async op to fetch a URL and return the response body as text
#[op2(async)]
#[string]
async fn op_fetch(#[string] url: String) -> Result<String, FetchError> {
    let response = reqwest::get(&url).await?;
    let text = response.text().await?;
    Ok(text)
}

// Define the extension with both sync and async ops
extension!(react_ssr_ext, ops = [op_set_html, op_fetch]);

fn main() {
    // Build TypeScript bundles before starting server
    println!("Preparing React SSR example...");
    if let Err(e) = build_typescript() {
        eprintln!("Failed to build TypeScript: {e}");
        std::process::exit(1);
    }

    // Load bundles into memory
    if let Err(e) = load_bundles() {
        eprintln!("Failed to load bundles: {e}");
        std::process::exit(1);
    }

    // Start the server
    start_server();
}

fn start_server() {
    // Create a multi-threaded runtime for Axum
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        // Create Axum router
        let app = Router::new()
            .route("/", get(ssr_handler))
            .route("/api/weather", get(mock_weather_handler))
            .route("/hello-world-client.js", get(client_bundle_handler));

        let addr = SocketAddr::from(([127, 0, 0, 1], 3001));

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("Failed to bind to port 3001");

        println!("Server running at http://{addr}");
        println!("Open in your browser to see SSR + hydration in action!");

        axum::serve(listener, app).await.unwrap();
    });
}

fn get_hello_world_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/hello-world")
}

fn get_bundle_path(name: &str) -> PathBuf {
    get_hello_world_dir().join("dist").join(name)
}

fn build_typescript() -> Result<(), Box<dyn std::error::Error>> {
    let hello_world_dir = get_hello_world_dir();

    // Check if node_modules exists, if not run bun install
    if !hello_world_dir.join("node_modules").exists() {
        println!("Installing dependencies...");
        let status = Command::new("bun")
            .arg("install")
            .current_dir(&hello_world_dir)
            .status()?;
        if !status.success() {
            return Err("bun install failed".into());
        }
    }

    // Build both server and client bundles
    println!("Building TypeScript bundles...");
    let status = Command::new("bun")
        .args(["run", "build"])
        .current_dir(&hello_world_dir)
        .status()?;
    if !status.success() {
        return Err("bun run build failed".into());
    }

    println!("TypeScript build complete!");
    Ok(())
}

fn load_bundles() -> Result<(), Box<dyn std::error::Error>> {
    let server = std::fs::read_to_string(get_bundle_path("hello-world.js"))?;
    SERVER_BUNDLE.set(server).ok();

    let client = std::fs::read_to_string(get_bundle_path("hello-world-client.js"))?;
    CLIENT_BUNDLE.set(client).ok();

    Ok(())
}

async fn client_bundle_handler() -> impl axum::response::IntoResponse {
    let bundle = CLIENT_BUNDLE.get().map(|s| s.as_str()).unwrap_or("");
    (
        [(header::CONTENT_TYPE, "application/javascript")],
        bundle.to_string(),
    )
}

async fn ssr_handler() -> Html<String> {
    // Spawn SSR in a dedicated thread with its own single-threaded runtime
    // because deno_core's JsRuntime is not Send
    let (tx, rx) = oneshot::channel();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let _ = tx.send(rt.block_on(run_react_ssr()));
    });

    match rx.await {
        Ok(Ok(html)) => Html(html),
        Ok(Err(e)) => Html(format!("<h1>Error: {e}</h1>")),
        Err(e) => Html(format!("<h1>Channel Error: {e}</h1>")),
    }
}
async fn mock_weather_handler() -> Json<Value> {
    let temp_c: u32 = rand::rng().random_range(0..100);
    // Fixed mock data in wttr.in format - no external dependencies needed
    Json(json!({
        "current_condition": [{
            "temp_C": format!("{temp_c}"),
            "temp_F": "59",
            "humidity": "65",
            "weatherDesc": [{"value": "Partly cloudy"}],
            "windspeedKmph": "12",
            "winddir16Point": "NW"
        }],
        "nearest_area": [{
            "areaName": [{"value": "London"}],
            "country": [{"value": "United Kingdom"}]
        }]
    }))
}

/// Web API polyfills required for React to run in deno_core.
/// These provide console, performance, MessageChannel, TextEncoder/Decoder, and fetch.
const WEB_API_POLYFILLS: &str = r#"
// SSR Configuration - injected by Rust
globalThis.__SSR_CONFIG__ = {
    weatherApiUrl: "http://localhost:3001/api/weather"
};

// Console polyfill - forward JS logs to Rust stdout
globalThis.console = {
    log: (...args) => Deno.core.print('[JS] ' + args.map(a => typeof a === 'object' ? JSON.stringify(a) : String(a)).join(' ') + '\n', false),
    error: (...args) => Deno.core.print('[JS ERROR] ' + args.map(a => typeof a === 'object' ? JSON.stringify(a) : String(a)).join(' ') + '\n', true),
    warn: (...args) => Deno.core.print('[JS WARN] ' + args.map(a => typeof a === 'object' ? JSON.stringify(a) : String(a)).join(' ') + '\n', false),
    info: (...args) => Deno.core.print('[JS] ' + args.map(a => typeof a === 'object' ? JSON.stringify(a) : String(a)).join(' ') + '\n', false),
    debug: () => {},
};

// Performance polyfill for timing
const performanceStart = Date.now();
globalThis.performance = {
    now: () => Date.now() - performanceStart,
};

// MessageChannel polyfill - React uses this for scheduling
class MessageChannelPolyfill {
    constructor() {
        this.port1 = {
            postMessage: () => {
                if (this.port2.onmessage) {
                    queueMicrotask(this.port2.onmessage);
                }
            },
            onmessage: null,
        };
        this.port2 = {
            postMessage: () => {
                if (this.port1.onmessage) {
                    queueMicrotask(this.port1.onmessage);
                }
            },
            onmessage: null,
        };
    }
}
globalThis.MessageChannel = MessageChannelPolyfill;

// TextEncoder/TextDecoder polyfills - used by React for string encoding
class TextEncoderPolyfill {
    encode(str) {
        const utf8 = unescape(encodeURIComponent(str));
        const result = new Uint8Array(utf8.length);
        for (let i = 0; i < utf8.length; i++) {
            result[i] = utf8.charCodeAt(i);
        }
        return result;
    }
    encodeInto(str, dest) {
        const encoded = this.encode(str);
        const len = Math.min(encoded.length, dest.length);
        dest.set(encoded.subarray(0, len));
        return { read: str.length, written: len };
    }
}
globalThis.TextEncoder = TextEncoderPolyfill;

class TextDecoderPolyfill {
    constructor(label = 'utf-8') {
        this.encoding = label.toLowerCase();
    }
    decode(input) {
        if (!input) return '';
        const bytes = input instanceof Uint8Array ? input : new Uint8Array(input);
        let result = '';
        for (let i = 0; i < bytes.length; i++) {
            result += String.fromCharCode(bytes[i]);
        }
        return decodeURIComponent(escape(result));
    }
}
globalThis.TextDecoder = TextDecoderPolyfill;

// Fetch polyfill using our custom op
globalThis.fetch = async function(url, options = {}) {
    const urlStr = typeof url === 'string' ? url : url.toString();
    const body = await Deno.core.ops.op_fetch(urlStr);
    return {
        ok: true,
        status: 200,
        statusText: 'OK',
        text: async () => body,
        json: async () => JSON.parse(body),
        headers: new Map(),
    };
};
"#;

async fn run_react_ssr() -> anyhow::Result<String> {
    let js_code = SERVER_BUNDLE
        .get()
        .ok_or_else(|| anyhow::anyhow!("Server bundle not loaded"))?
        .as_str();

    // Create runtime with our custom extension
    let mut runtime = JsRuntime::new(RuntimeOptions {
        extensions: vec![react_ssr_ext::init()],
        ..Default::default()
    });

    // Inject polyfills, then execute React bundle
    runtime.execute_script("<polyfills>", WEB_API_POLYFILLS)?;
    runtime.execute_script("<react-ssr>", js_code)?;

    // Run event loop for async operations (fetch)
    runtime.run_event_loop(Default::default()).await?;

    // Retrieve the rendered HTML
    RENDERED_HTML
        .with(|cell| cell.borrow_mut().take())
        .ok_or_else(|| anyhow::anyhow!("No HTML was rendered"))
}
