//! Pure polyfill generation for React 19 SSR.
//!
//! This module contains pure functions that generate JavaScript polyfills
//! required for server-side rendering React 19 with deno_core.

use crate::error::{Result, SsrCoreError};

/// Generate Web API polyfills required for React 19 prerender.
///
/// This is a pure function - transforms input into output string.
/// Uses safe JSON injection via double-encoding to prevent injection attacks.
pub fn generate_polyfills(config_json: &str, node_env: &str) -> Result<String> {
    // Double-encode: JSON string containing JSON to prevent injection
    let config_json_escaped = serde_json::to_string(config_json)
        .map_err(|e| SsrCoreError::Serialization(e.to_string()))?;

    // Escape node_env for safe string interpolation
    let node_env_escaped = node_env.replace('\\', "\\\\").replace('\'', "\\'");

    let console_polyfill = CONSOLE_POLYFILL;
    let performance_polyfill = PERFORMANCE_POLYFILL;
    let message_channel_polyfill = MESSAGE_CHANNEL_POLYFILL;
    let text_encoder_polyfill = TEXT_ENCODER_POLYFILL;
    let stream_polyfills = STREAM_POLYFILLS;

    Ok(format!(
        r#"
// SSR Configuration - safely injected by Rust
globalThis.__SSR_CONFIG__ = JSON.parse({config_json_escaped});

// Process polyfill (Node.js compatibility)
globalThis.process = {{
    env: {{ NODE_ENV: '{node_env_escaped}' }},
    nextTick: (fn) => queueMicrotask(fn),
}};

{console_polyfill}
{performance_polyfill}
{message_channel_polyfill}
{text_encoder_polyfill}
{stream_polyfills}
"#
    ))
}

const CONSOLE_POLYFILL: &str = r#"
// Console polyfill - forward JS logs to Rust stdout
globalThis.console = {
    log: (...args) => Deno.core.print('[JS] ' + args.map(a => typeof a === 'object' ? JSON.stringify(a) : String(a)).join(' ') + '\n', false),
    error: (...args) => Deno.core.print('[JS ERROR] ' + args.map(a => typeof a === 'object' ? JSON.stringify(a) : String(a)).join(' ') + '\n', true),
    warn: (...args) => Deno.core.print('[JS WARN] ' + args.map(a => typeof a === 'object' ? JSON.stringify(a) : String(a)).join(' ') + '\n', false),
    info: (...args) => Deno.core.print('[JS] ' + args.map(a => typeof a === 'object' ? JSON.stringify(a) : String(a)).join(' ') + '\n', false),
    debug: () => {},
};
"#;

const PERFORMANCE_POLYFILL: &str = r#"
// Performance polyfill for timing
const performanceStart = Date.now();
globalThis.performance = { now: () => Date.now() - performanceStart };
"#;

const MESSAGE_CHANNEL_POLYFILL: &str = r#"
// MessageChannel polyfill - React uses this for scheduling
class MessageChannelPolyfill {
    constructor() {
        this.port1 = {
            postMessage: () => { if (this.port2.onmessage) queueMicrotask(this.port2.onmessage); },
            onmessage: null,
        };
        this.port2 = {
            postMessage: () => { if (this.port1.onmessage) queueMicrotask(this.port1.onmessage); },
            onmessage: null,
        };
    }
}
globalThis.MessageChannel = MessageChannelPolyfill;
"#;

const TEXT_ENCODER_POLYFILL: &str = r#"
// TextEncoder/TextDecoder polyfills
class TextEncoderPolyfill {
    encode(str) {
        const utf8 = unescape(encodeURIComponent(str));
        const result = new Uint8Array(utf8.length);
        for (let i = 0; i < utf8.length; i++) result[i] = utf8.charCodeAt(i);
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
    constructor(label = 'utf-8') { this.encoding = label.toLowerCase(); }
    decode(input) {
        if (!input) return '';
        const bytes = input instanceof Uint8Array ? input : new Uint8Array(input);
        let result = '';
        for (let i = 0; i < bytes.length; i++) result += String.fromCharCode(bytes[i]);
        return decodeURIComponent(escape(result));
    }
}
globalThis.TextDecoder = TextDecoderPolyfill;
"#;

const STREAM_POLYFILLS: &str = r#"
// ReadableStream polyfill for React 19 prerender
class ReadableStreamPolyfill {
    constructor(underlyingSource) {
        this._source = underlyingSource;
        this._started = false;
        this._done = false;
        this._chunks = [];
        this._controller = {
            enqueue: (chunk) => { this._chunks.push(chunk); },
            close: () => { this._done = true; },
            error: (e) => { this._error = e; },
            desiredSize: 1,
        };
    }
    getReader() {
        const self = this;
        return {
            async read() {
                if (self._error) throw self._error;
                if (self._chunks.length > 0) return { done: false, value: self._chunks.shift() };
                if (self._done) return { done: true, value: undefined };
                if (!self._started) {
                    self._started = true;
                    if (self._source.start) await self._source.start(self._controller);
                }
                if (self._chunks.length > 0) return { done: false, value: self._chunks.shift() };
                while (!self._done && self._chunks.length === 0) {
                    if (self._source.pull) await self._source.pull(self._controller);
                    else break;
                }
                if (self._error) throw self._error;
                if (self._chunks.length > 0) return { done: false, value: self._chunks.shift() };
                return { done: true, value: undefined };
            },
            releaseLock() {},
        };
    }
}
globalThis.ReadableStream = ReadableStreamPolyfill;

class WritableStreamPolyfill {
    constructor(underlyingSink) { this._sink = underlyingSink; }
    getWriter() {
        const self = this;
        return {
            write(chunk) { if (self._sink?.write) return self._sink.write(chunk); },
            close() { if (self._sink?.close) return self._sink.close(); },
            releaseLock() {},
        };
    }
}
globalThis.WritableStream = WritableStreamPolyfill;

class TransformStreamPolyfill {
    constructor(transformer) {
        this._transformer = transformer;
        this.readable = new ReadableStreamPolyfill({ start: () => {}, pull: () => {} });
        this.writable = new WritableStreamPolyfill({ write: () => {}, close: () => {} });
    }
}
globalThis.TransformStream = TransformStreamPolyfill;
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_polyfills_contains_config() {
        let result = generate_polyfills(r#"{"test": true}"#, "production").unwrap();
        // Config should be double-encoded for safe injection
        assert!(result.contains("JSON.parse("));
        assert!(result.contains("NODE_ENV: 'production'"));
    }

    #[test]
    fn test_generate_polyfills_contains_all_polyfills() {
        let result = generate_polyfills("{}", "development").unwrap();
        assert!(result.contains("globalThis.console"));
        assert!(result.contains("globalThis.performance"));
        assert!(result.contains("globalThis.MessageChannel"));
        assert!(result.contains("globalThis.TextEncoder"));
        assert!(result.contains("globalThis.ReadableStream"));
    }

    #[test]
    fn test_generate_polyfills_escapes_node_env() {
        // Test that special characters in node_env are escaped
        let result = generate_polyfills("{}", "test's \"env\"").unwrap();
        assert!(result.contains("NODE_ENV: 'test\\'s \"env\"'"));
    }

    #[test]
    fn test_generate_polyfills_handles_complex_config() {
        let config = r#"{"nested":{"key":"value"},"array":[1,2,3]}"#;
        let result = generate_polyfills(config, "production").unwrap();
        // Should contain the escaped JSON
        assert!(result.contains("JSON.parse("));
    }

    #[test]
    fn test_polyfills_prevent_js_injection() {
        // Attempt JavaScript injection via config - should be safely contained
        // in JSON.parse() which only parses data, not code
        let malicious = r#"{"x":"'); alert('xss'); ('"}"#;
        let result = generate_polyfills(malicious, "production").unwrap();
        // The config should be inside JSON.parse(), not directly interpolated
        assert!(result.contains("JSON.parse("));
        // The malicious payload should be double-escaped (JSON string containing JSON)
        // so it can't break out of the string context
        assert!(result.contains(r#"\""#)); // Escaped quotes
    }

    #[test]
    fn test_polyfills_node_env_injection() {
        // Attempt injection via node_env
        let result = generate_polyfills("{}", "'; alert('xss'); '").unwrap();
        // Single quotes should be escaped
        assert!(result.contains(r"\'"));
    }
}
