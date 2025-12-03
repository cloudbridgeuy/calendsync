# React SSR Example with deno_core

This document provides context for the React SSR example in `crates/calendsync/examples/`.

## Running

```bash
cargo run --example react-ssr -p calendsync
# Opens at http://localhost:3001
```

## Key Concepts

### 1. deno_core JsRuntime

Executes JavaScript/TypeScript in Rust. The `JsRuntime` is not `Send`, so it requires a dedicated thread with its own tokio runtime:

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

### 2. Custom Ops

Rust functions callable from JavaScript, defined with the `#[op2]` macro:

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

Register ops via an extension:

```rust
extension!(react_ssr_ext, ops = [op_set_html, op_fetch]);
```

### 3. Web API Polyfills

React requires these Web APIs that don't exist in deno_core's minimal environment:

| API | Purpose |
|-----|---------|
| `console` | Forward JS logs to Rust stdout |
| `performance.now()` | React timing internals |
| `MessageChannel` | React scheduler |
| `TextEncoder/TextDecoder` | String encoding |
| `fetch` | HTTP requests via custom op |

Example polyfill structure:

```javascript
globalThis.console = {
    log: (...args) => Deno.core.print('[JS] ' + args.join(' ') + '\n', false),
    error: (...args) => Deno.core.print('[JS ERROR] ' + args.join(' ') + '\n', true),
};

globalThis.fetch = async function(url) {
    const body = await Deno.core.ops.op_fetch(url.toString());
    return {
        ok: true,
        status: 200,
        text: async () => body,
        json: async () => JSON.parse(body),
    };
};
```

### 4. Hydration

Server embeds `__INITIAL_STATE__` in HTML, client reads it to avoid re-fetching and ensure DOM matches:

```tsx
// Server (App.tsx)
<script dangerouslySetInnerHTML={{
    __html: `window.__INITIAL_STATE__ = ${JSON.stringify({ weather, greeting })}`
}} />

// Client (client.tsx)
const { weather, greeting } = window.__INITIAL_STATE__
hydrateRoot(document, <App weather={weather} greeting={greeting} />)
```

### 5. Avoiding Hydration Mismatches

**Critical**: Any value that differs between server and client causes hydration errors:
- Random values (`Math.random()`, `faker.word.adjective()`)
- Current time (`Date.now()`)
- Browser-specific APIs (`window`, `localStorage`)

**Solution**: Generate these values once during SSR and pass through `__INITIAL_STATE__`:

```typescript
// index.tsx (SSR entry point)
const greeting = faker.word.adjective()  // Generate once
const html = renderToString(<App greeting={greeting} />)
```

## File Structure

```
examples/
├── react-ssr.rs           # Rust: Axum server + deno_core SSR
├── README.md              # Full documentation
└── hello-world/           # TypeScript React app
    ├── src/
    │   ├── index.tsx      # Server entry (renderToString)
    │   ├── client.tsx     # Client entry (hydrateRoot)
    │   ├── App.tsx        # Root component
    │   └── components/    # React components
    │       ├── Greeting.tsx
    │       ├── Counter.tsx
    │       └── Weather.tsx
    ├── package.json       # Dependencies (react, faker)
    └── tsconfig.json
```

## Dependencies

Example uses dev dependencies in `calendsync/Cargo.toml`:

| Crate | Purpose |
|-------|---------|
| `deno_core` | JavaScript runtime |
| `deno_error` | Error types for ops |
| `reqwest` | HTTP client for fetch op |
| `rand` | Random number generation |

## Extending

1. **New ops**: Define with `#[op2]` macro, add to extension's `ops` array
2. **New components**: Add to `src/components/`, import in `App.tsx`
3. **New npm packages**: Add to `package.json`, import in TypeScript
4. **New routes**: Add handlers in `start_server()`, register with `Router::new()`

## See Also

- Full documentation: `crates/calendsync/examples/react-ssr/README.md`
- Rust source: `crates/calendsync/examples/react-ssr.rs`
