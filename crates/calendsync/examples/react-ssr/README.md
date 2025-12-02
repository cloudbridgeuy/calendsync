# React SSR Example with deno_core

This example demonstrates server-side rendering (SSR) of a React application using `deno_core` as the JavaScript runtime, with full client-side hydration for interactivity.

## What This Example Demonstrates

- **React SSR with deno_core**: Running `renderToString` inside a Rust application
- **Custom ops**: Bidirectional communication between Rust and JavaScript
- **Async data fetching**: Making HTTP requests from JavaScript during SSR
- **Client-side hydration**: Seamless transition from static HTML to interactive React
- **External npm packages**: Using libraries like `@faker-js/faker` in both SSR and client contexts

## Quick Start

```bash
cargo run --example react-ssr -p calendsync
```

Then open http://localhost:3001 in your browser.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Rust (Axum)                             │
│  ┌───────────────┐    ┌───────────────┐    ┌───────────────┐    │
│  │ GET /         │    │ GET /api/     │    │ GET /hello-   │    │
│  │ (SSR Handler) │    │ weather       │    │ world-client  │    │
│  └───────┬───────┘    └───────────────┘    │ .js           │    │
│          │                                 └───────────────┘    │
│          ▼                                                      │
│  ┌───────────────────────────────────────┐                      │
│  │         Dedicated SSR Thread          │                      │
│  │  ┌─────────────────────────────────┐  │                      │
│  │  │    deno_core JsRuntime          │  │                      │
│  │  │  ┌───────────────────────────┐  │  │                      │
│  │  │  │  Web API Polyfills        │  │  │                      │
│  │  │  │  (console, fetch, etc.)   │  │  │                      │
│  │  │  └───────────────────────────┘  │  │                      │
│  │  │  ┌───────────────────────────┐  │  │                      │
│  │  │  │  React + App Bundle       │  │  │                      │
│  │  │  │  (hello-world.js)         │  │  │                      │
│  │  │  └───────────────────────────┘  │  │                      │
│  │  └─────────────────────────────────┘  │                      │
│  └───────────────────────────────────────┘                      │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼ HTML Response
┌─────────────────────────────────────────────────────────────────┐
│                         Browser                                 │
│  ┌───────────────────────────────────────┐                      │
│  │  Server-rendered HTML                 │                      │
│  │  + Embedded __INITIAL_STATE__         │                      │
│  │  + <script src="/hello-world-client.js">                     │
│  └───────────────────────────────────────┘                      │
│                              │                                  │
│                              ▼ hydrateRoot()                    │
│  ┌───────────────────────────────────────┐                      │
│  │  Interactive React Application        │                      │
│  └───────────────────────────────────────┘                      │
└─────────────────────────────────────────────────────────────────┘
```

## Key Concepts

### Custom Ops

`deno_core` allows defining custom "ops" - functions callable from JavaScript that execute Rust code.

```rust
// Sync op: receives HTML from JavaScript
#[op2(fast)]
fn op_set_html(#[string] html: String) {
    RENDERED_HTML.with(|cell| {
        *cell.borrow_mut() = Some(html);
    });
}

// Async op: fetch HTTP requests
#[op2(async)]
#[string]
async fn op_fetch(#[string] url: String) -> Result<String, FetchError> {
    let response = reqwest::get(&url).await?;
    Ok(response.text().await?)
}
```

These ops are registered via an extension:

```rust
extension!(
    react_ssr_ext,
    ops = [op_set_html, op_fetch]
);
```

### Thread Spawning for JsRuntime

`deno_core::JsRuntime` is not `Send`, so it cannot be used directly in `async` Axum handlers. The solution is to spawn a dedicated thread with its own tokio runtime:

```rust
async fn ssr_handler() -> Html<String> {
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
```

### Bundle Loading

TypeScript is compiled at startup using Bun, then loaded into memory:

```rust
static SERVER_BUNDLE: OnceLock<String> = OnceLock::new();
static CLIENT_BUNDLE: OnceLock<String> = OnceLock::new();

fn load_bundles() -> Result<(), Box<dyn std::error::Error>> {
    let server = std::fs::read_to_string(get_bundle_path("hello-world.js"))?;
    SERVER_BUNDLE.set(server).ok();
    // ...
}
```

## Polyfill Requirements

React and react-dom expect certain Web APIs that don't exist in `deno_core's` minimal environment. These must be polyfilled:

### console

React logs warnings and errors. Forward these to Rust's stdout:

```javascript
globalThis.console = {
  log: (...args) => Deno.core.print("[JS] " + args.join(" ") + "\n", false),
  error: (...args) =>
    Deno.core.print("[JS ERROR] " + args.join(" ") + "\n", true),
  // ...
};
```

### performance.now()

Used internally by React for timing:

```javascript
const performanceStart = Date.now();
globalThis.performance = {
  now: () => Date.now() - performanceStart,
};
```

### MessageChannel

React's scheduler uses `MessageChannel` for task scheduling:

```javascript
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
      /* ... */
    };
  }
}
globalThis.MessageChannel = MessageChannelPolyfill;
```

### TextEncoder / TextDecoder

Used by React for string encoding:

```javascript
class TextEncoderPolyfill {
  encode(str) {
    const utf8 = unescape(encodeURIComponent(str));
    const result = new Uint8Array(utf8.length);
    for (let i = 0; i < utf8.length; i++) {
      result[i] = utf8.charCodeAt(i);
    }
    return result;
  }
}
globalThis.TextEncoder = TextEncoderPolyfill;
```

### fetch

Implemented via the custom `op_fetch` op:

```javascript
globalThis.fetch = async function (url) {
  const body = await Deno.core.ops.op_fetch(url.toString());
  return {
    ok: true,
    status: 200,
    text: async () => body,
    json: async () => JSON.parse(body),
  };
};
```

## TypeScript Structure

```
hello-world/
├── src/
│   ├── index.tsx           # Server entry point (SSR)
│   ├── client.tsx          # Client entry point (hydration)
│   ├── App.tsx             # Root component
│   └── components/
│       ├── Greeting.tsx    # Uses @faker-js/faker
│       ├── Counter.tsx     # Stateful component (useState)
│       └── Weather.tsx     # Displays fetched data
├── package.json
└── tsconfig.json
```

### Server Bundle (index.tsx)

Fetches data, renders React to HTML, and sends it to Rust:

```typescript
async function main() {
    const weather = await fetchWeather("London")
    const html = renderToString(<App weather={weather} />)
    Deno.core.ops.op_set_html(html)
}
```

### Client Bundle (client.tsx)

Hydrates the server-rendered HTML:

```typescript
const { weather } = window.__INITIAL_STATE__
hydrateRoot(document, <App weather={weather} />)
```

## Hydration

State is passed from server to client via an embedded script tag:

```tsx
// In App.tsx
<script dangerouslySetInnerHTML={{
    __html: `window.__INITIAL_STATE__ = ${JSON.stringify({ weather, greeting })}`
}} />
<script src="/hello-world-client.js" />
```

The client reads this state and passes it to the same components, ensuring React can "hydrate" (attach event handlers to) the existing DOM without re-rendering.

### Avoiding Hydration Mismatches

**Important**: Any value that differs between server and client will cause a hydration mismatch error. This includes:

- Random values (`Math.random()`, `faker.word.adjective()`)
- Current time (`Date.now()`)
- Browser-specific APIs (`window`, `localStorage`)

The solution is to generate these values once during SSR and pass them through `__INITIAL_STATE__`:

```typescript
// index.tsx (SSR entry point)
const greeting = faker.word.adjective()  // Generate once
const html = renderToString(<App greeting={greeting} />)

// App.tsx - include in __INITIAL_STATE__
window.__INITIAL_STATE__ = ${JSON.stringify({ greeting })}

// client.tsx - read and pass the same value
const { greeting } = window.__INITIAL_STATE__
hydrateRoot(document, <App greeting={greeting} />)
```

## External Dependencies

The example uses `@faker-js/faker` to demonstrate that npm packages work in the deno_core SSR context:

```typescript
// index.tsx (server entry point)
import { faker } from "@faker-js/faker"

const greeting = faker.word.adjective()
const html = renderToString(<App greeting={greeting} />)
```

Note: Since faker generates random values, we only call it during SSR and pass the result through `__INITIAL_STATE__` to avoid hydration mismatches.

## Console Output

When running, you'll see logs from JavaScript forwarded to the terminal:

```
[JS] [SSR] Starting React SSR...
[JS] [SSR] Fetching weather for London...
[JS] [SSR] Weather data fetched successfully
[JS] [Greeting] Generated adjective: flawless
[JS] [SSR] React render complete
```

## Extending This Example

To add new functionality:

1. **New ops**: Define with `#[op2]` macro, add to extension's `ops` array
2. **New components**: Add to `src/components/`, import in `App.tsx`
3. **New npm packages**: Add to `package.json`, import in TypeScript
4. **New routes**: Add handlers in `start_server()`, register with `Router::new()`

## Dependencies

### Rust (Cargo.toml)

- `deno_core` - JavaScript runtime
- `deno_error` - Error types for ops
- `axum` - Web framework
- `reqwest` - HTTP client for `op_fetch`
- `tokio` - Async runtime

### TypeScript (package.json)

- `react` / `react-dom` - React 19
- `@faker-js/faker` - Random data generation
- `typescript` - Type checking
