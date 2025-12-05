axum is a web application framework that focuses on ergonomics and modularity.

## [§](#high-level-features)High-level features

- Route requests to handlers with a macro-free API.
- Declaratively parse requests using extractors.
- Simple and predictable error handling model.
- Generate responses with minimal boilerplate.
- Take full advantage of the [`tower`](https://crates.io/crates/tower) and [`tower-http`](https://crates.io/crates/tower-http) ecosystem of
  middleware, services, and utilities.

In particular, the last point is what sets `axum` apart from other frameworks.`axum` doesn’t have its own middleware system but instead uses[`tower::Service`](https://docs.rs/tower-service/0.3.3/x86_64-unknown-linux-gnu/tower_service/trait.Service.html). This means `axum` gets timeouts, tracing, compression,
authorization, and more, for free. It also enables you to share middleware with
applications written using [`hyper`](http://crates.io/crates/hyper) or [`tonic`](http://crates.io/crates/tonic).

## [§](#compatibility)Compatibility

axum is designed to work with [tokio](https://docs.rs/tokio/1.47.1/x86_64-unknown-linux-gnu/tokio/index.html) and [hyper](https://docs.rs/hyper/1.7.0/x86_64-unknown-linux-gnu/hyper/index.html). Runtime and
transport layer independence is not a goal, at least for the time being.

## [§](#example)Example

The “Hello, World!” of axum is:

```
use axum::{
    routing::get,
    Router,
};

#[tokio::main]
async fn main() {
    // build our application with a single route
    let app = Router::new().route("/", get(|| async { "Hello, World!" }));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

Note using `#[tokio::main]` requires you enable tokio’s `macros` and `rt-multi-thread` features
or just `full` to enable all features (`cargo add tokio --features macros,rt-multi-thread`).

## [§](#routing)Routing

[`Router`](struct.Router.html) is used to set up which paths go to which services:

```
use axum::{Router, routing::get};

// our router
let app = Router::new()
    .route("/", get(root))
    .route("/foo", get(get_foo).post(post_foo))
    .route("/foo/bar", get(foo_bar));

// which calls one of these handlers
async fn root() {}
async fn get_foo() {}
async fn post_foo() {}
async fn foo_bar() {}
```

See [`Router`](struct.Router.html) for more details on routing.

## [§](#handlers)Handlers

In axum a “handler” is an async function that accepts zero or more[“extractors”](extract/index.html) as arguments and returns something that
can be converted [into a response](response/index.html).

Handlers are where your application logic lives and axum applications are built
by routing between handlers.

See [`handler`](handler/index.html) for more details on handlers.

## [§](#extractors)Extractors

An extractor is a type that implements [`FromRequest`](extract/trait.FromRequest.html) or [`FromRequestParts`](extract/trait.FromRequestParts.html). Extractors are
how you pick apart the incoming request to get the parts your handler needs.

```
use axum::extract::{Path, Query, Json};
use std::collections::HashMap;

// `Path` gives you the path parameters and deserializes them.
async fn path(Path(user_id): Path<u32>) {}

// `Query` gives you the query parameters and deserializes them.
async fn query(Query(params): Query<HashMap<String, String>>) {}

// Buffer the request body and deserialize it as JSON into a
// `serde_json::Value`. `Json` supports any type that implements
// `serde::Deserialize`.
async fn json(Json(payload): Json<serde_json::Value>) {}
```

See [`extract`](extract/index.html) for more details on extractors.

## [§](#responses)Responses

Anything that implements [`IntoResponse`](response/trait.IntoResponse.html) can be returned from handlers.

```
use axum::{
    body::Body,
    routing::get,
    response::Json,
    Router,
};
use serde_json::{Value, json};

// `&'static str` becomes a `200 OK` with `content-type: text/plain; charset=utf-8`
async fn plain_text() -> &'static str {
    "foo"
}

// `Json` gives a content-type of `application/json` and works with any type
// that implements `serde::Serialize`
async fn json() -> Json<Value> {
    Json(json!({ "data": 42 }))
}

let app = Router::new()
    .route("/plain_text", get(plain_text))
    .route("/json", get(json));
```

See [`response`](response/index.html) for more details on building responses.

## [§](#error-handling)Error handling

axum aims to have a simple and predictable error handling model. That means
it is simple to convert errors into responses and you are guaranteed that
all errors are handled.

See [`error_handling`](error_handling/index.html) for more details on axum’s
error handling model and how to handle errors gracefully.

## [§](#middleware)Middleware

There are several different ways to write middleware for axum. See[`middleware`](middleware/index.html) for more details.

## [§](#sharing-state-with-handlers)Sharing state with handlers

It is common to share some state between handlers. For example, a
pool of database connections or clients to other services may need to
be shared.

The four most common ways of doing that are:

- Using the [`State`](extract/struct.State.html) extractor
- Using request extensions
- Using closure captures
- Using task-local variables

### [§](#using-the-state-extractor)Using the [`State`](extract/struct.State.html) extractor

```
use axum::{
    extract::State,
    routing::get,
    Router,
};
use std::sync::Arc;

struct AppState {
    // ...
}

let shared_state = Arc::new(AppState { /* ... */ });

let app = Router::new()
    .route("/", get(handler))
    .with_state(shared_state);

async fn handler(
    State(state): State<Arc<AppState>>,
) {
    // ...
}
```

You should prefer using [`State`](extract/struct.State.html) if possible since it’s more type safe. The downside is that
it’s less dynamic than task-local variables and request extensions.

See [`State`](extract/struct.State.html) for more details about accessing state.

### [§](#using-request-extensions)Using request extensions

Another way to share state with handlers is using [`Extension`](struct.Extension.html) as
layer and extractor:

```
use axum::{
    extract::Extension,
    routing::get,
    Router,
};
use std::sync::Arc;

struct AppState {
    // ...
}

let shared_state = Arc::new(AppState { /* ... */ });

let app = Router::new()
    .route("/", get(handler))
    .layer(Extension(shared_state));

async fn handler(
    Extension(state): Extension<Arc<AppState>>,
) {
    // ...
}
```

The downside to this approach is that you’ll get runtime errors
(specifically a `500 Internal Server Error` response) if you try and extract
an extension that doesn’t exist, perhaps because you forgot to add the
middleware or because you’re extracting the wrong type.

### [§](#using-closure-captures)Using closure captures

State can also be passed directly to handlers using closure captures:

```rust
use axum::{
    Json,
    extract::{Extension, Path},
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use serde::Deserialize;

struct AppState {
    // ...
}

let shared_state = Arc::new(AppState { /* ... */ });

let app = Router::new()
    .route(
        "/users",
        post({
            let shared_state = Arc::clone(&shared_state);
            move |body| create_user(body, shared_state)
        }),
    )
    .route(
        "/users/{id}",
        get({
            let shared_state = Arc::clone(&shared_state);
            move |path| get_user(path, shared_state)
        }),
    );

async fn get_user(Path(user_id): Path<String>, state: Arc<AppState>) {
    // ...
}

async fn create_user(Json(payload): Json<CreateUserPayload>, state: Arc<AppState>) {
    // ...
}

#[derive(Deserialize)]
struct CreateUserPayload {
    // ...
}
```

The downside to this approach is that it’s the most verbose approach.

### [§](#using-task-local-variables)Using task-local variables

This also allows to share state with `IntoResponse` implementations:

```
use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use tokio::task_local;

#[derive(Clone)]
struct CurrentUser {
    name: String,
}
task_local! {
    pub static USER: CurrentUser;
}

async fn auth(req: Request, next: Next) -> Result<Response, StatusCode> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if let Some(current_user) = authorize_current_user(auth_header).await {
        // State is setup here in the middleware
        Ok(USER.scope(current_user, next.run(req)).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}
async fn authorize_current_user(auth_token: &str) -> Option<CurrentUser> {
    Some(CurrentUser {
        name: auth_token.to_string(),
    })
}

struct UserResponse;

impl IntoResponse for UserResponse {
    fn into_response(self) -> Response {
        // State is accessed here in the IntoResponse implementation
        let current_user = USER.with(|u| u.clone());
        (StatusCode::OK, current_user.name).into_response()
    }
}

async fn handler() -> UserResponse {
    UserResponse
}

let app: Router = Router::new()
    .route("/", get(handler))
    .route_layer(middleware::from_fn(auth));
```

The main downside to this approach is that it only works when the async executor being used
has the concept of task-local variables. The example above uses[tokio’s `task_local` macro](https://docs.rs/tokio/1/tokio/macro.task_local.html).
smol does not yet offer equivalent functionality at the time of writing (see[this GitHub issue](https://github.com/smol-rs/async-executor/issues/139)).

## [§](#building-integrations-for-axum)Building integrations for axum

Libraries authors that want to provide [`FromRequest`](extract/trait.FromRequest.html), [`FromRequestParts`](extract/trait.FromRequestParts.html), or[`IntoResponse`](response/trait.IntoResponse.html) implementations should depend on the [`axum-core`](http://crates.io/crates/axum-core) crate, instead of `axum` if
possible. [`axum-core`](http://crates.io/crates/axum-core) contains core types and traits and is less likely to receive breaking
changes.

## [§](#required-dependencies)Required dependencies

To use axum there are a few dependencies you have to pull in as well:

```
[dependencies]
axum = "<latest-version>"
tokio = { version = "<latest-version>", features = ["full"] }
tower = "<latest-version>"
```

The `"full"` feature for tokio isn’t necessary but it’s the easiest way to get started.

Tower isn’t strictly necessary either but helpful for testing. See the
testing example in the repo to learn more about testing axum apps.

## [§](#examples)Examples

The axum repo contains [a number of examples](https://github.com/tokio-rs/axum/tree/main/examples) that show how to put all the
pieces together.

## [§](#feature-flags)Feature flags

axum uses a set of [feature flags](https://doc.rust-lang.org/cargo/reference/features.html#the-features-section) to reduce the amount of compiled and
optional dependencies.

The following optional features are available:

| Name           | Description                                                                                                          | Default? |
| -------------- | -------------------------------------------------------------------------------------------------------------------- | -------- |
| `http1`        | Enables hyper’s `http1` feature                                                                                      | ✔       |
| `http2`        | Enables hyper’s `http2` feature                                                                                      |          |
| `json`         | Enables the [`Json`](struct.Json.html) type and some similar convenience functionality                               | ✔       |
| `macros`       | Enables optional utility macros                                                                                      |          |
| `matched-path` | Enables capturing of every request’s router path and the [`MatchedPath`](extract/struct.MatchedPath.html) extractor  | ✔       |
| `multipart`    | Enables parsing `multipart/form-data` requests with [`Multipart`](extract/struct.Multipart.html)                     |          |
| `original-uri` | Enables capturing of every request’s original URI and the [`OriginalUri`](extract/struct.OriginalUri.html) extractor | ✔       |
| `tokio`        | Enables `tokio` as a dependency and `axum::serve`, `SSE` and `extract::connect_info` types.                          | ✔       |
| `tower-log`    | Enables `tower`’s `log` feature                                                                                      | ✔       |
| `tracing`      | Log rejections from built-in extractors                                                                              | ✔       |
| `ws`           | Enables WebSockets support via [`extract::ws`](extract/ws/index.html)                                                |          |
| `form`         | Enables the `Form` extractor                                                                                         | ✔       |
| `query`        | Enables the `Query` extractor                                                                                        | ✔       |

## Anyhow error response

```rust
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};

#[tokio::main]
async fn main() {
    let app = app();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn handler() -> Result<(), AppError> {
    try_thing()?;
    Ok(())
}

fn try_thing() -> Result<(), anyhow::Error> {
    anyhow::bail!("it failed!")
}

// Make our own error that wraps `anyhow::Error`.
struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

fn app() -> Router {
    Router::new().route("/", get(handler))
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request, http::StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_main_page() {
        let response = app()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let body = response.into_body();
        let bytes = body.collect().await.unwrap().to_bytes();
        let html = String::from_utf8(bytes.to_vec()).unwrap();

        assert_eq!(html, "Something went wrong: it failed!");
    }
}
```

## Auto-Reload

```rust
use axum::{response::Html, routing::get, Router};
use listenfd::ListenFd;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    // build our application with a route
    let app = Router::new().route("/", get(handler));

    let mut listenfd = ListenFd::from_env();
    let listener = match listenfd.take_tcp_listener(0).unwrap() {
        // if we are given a tcp listener on listen fd 0, we use that one
        Some(listener) => {
            listener.set_nonblocking(true).unwrap();
            TcpListener::from_std(listener).unwrap()
        }
        // otherwise fall back to local listening
        None => TcpListener::bind("127.0.0.1:3000").await.unwrap(),
    };

    // run it
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}
```

## CORS

```rust
use axum::{
    http::{HeaderValue, Method},
    response::{Html, IntoResponse},
    routing::get,
    Json, Router,
};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() {
    let frontend = async {
        let app = Router::new().route("/", get(html));
        serve(app, 3000).await;
    };

    let backend = async {
        let app = Router::new().route("/json", get(json)).layer(
            // see https://docs.rs/tower-http/latest/tower_http/cors/index.html
            // for more details
            //
            // pay attention that for some request types like posting content-type: application/json
            // it is required to add ".allow_headers([http::header::CONTENT_TYPE])"
            // or see this issue https://github.com/tokio-rs/axum/issues/849
            CorsLayer::new()
                .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
                .allow_methods([Method::GET]),
        );
        serve(app, 4000).await;
    };

    tokio::join!(frontend, backend);
}

async fn serve(app: Router, port: u16) {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn html() -> impl IntoResponse {
    Html(
        r#"
        <script>
            fetch('http://localhost:4000/json')
              .then(response => response.json())
              .then(data => console.log(data));
        </script>
        "#,
    )
}

async fn json() -> impl IntoResponse {
    Json(vec!["one", "two", "three"])
}
```

## Dependency Injection

```rust
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let user_repo = InMemoryUserRepo::default();

    // We generally have two ways to inject dependencies:
    //
    // 1. Using trait objects (`dyn SomeTrait`)
    //
    // Using trait objects is recommended unless you really need generics.

    let using = Router::new()
        .route("/users/{id}", get(get_user))
        .route("/users", post(create_user))
        .with_state(AppStateDyn {
            user_repo: Arc::new(user_repo.clone()),
        });

    let app = Router::new()
        .nest("/dyn", using);

    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

#[derive(Clone)]
struct AppStateDyn {
    user_repo: Arc<dyn UserRepo>,
}

#[derive(Debug, Serialize, Clone)]
struct User {
    id: Uuid,
    name: String,
}

#[derive(Deserialize)]
struct UserParams {
    name: String,
}

async fn create_user(
    State(state): State<AppStateDyn>,
    Json(params): Json<UserParams>,
) -> Json<User> {
    let user = User {
        id: Uuid::new_v4(),
        name: params.name,
    };

    state.user_repo.save_user(&user);

    Json(user)
}

async fn get_user(
    State(state): State<AppStateDyn>,
    Path(id): Path<Uuid>,
) -> Result<Json<User>, StatusCode> {
    match state.user_repo.get_user(id) {
        Some(user) => Ok(Json(user)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

trait UserRepo: Send + Sync {
    fn get_user(&self, id: Uuid) -> Option<User>;

    fn save_user(&self, user: &User);
}

#[derive(Debug, Clone, Default)]
struct InMemoryUserRepo {
    map: Arc<Mutex<HashMap<Uuid, User>>>,
}

impl UserRepo for InMemoryUserRepo {
    fn get_user(&self, id: Uuid) -> Option<User> {
        self.map.lock().unwrap().get(&id).cloned()
    }

    fn save_user(&self, user: &User) {
        self.map.lock().unwrap().insert(user.id, user.clone());
    }
}
```

## Error Handling

````rust
//! Example showing how to convert errors into responses.
//!
//! For successful requests the log output will be
//!
//! ```ignore
//! DEBUG request{method=POST uri=/users matched_path="/users"}: tower_http::trace::on_request: started processing request
//! DEBUG request{method=POST uri=/users matched_path="/users"}: tower_http::trace::on_response: finished processing request latency=0 ms status=200
//! ```
//!
//! For failed requests the log output will be
//!
//! ```ignore
//! DEBUG request{method=POST uri=/users matched_path="/users"}: tower_http::trace::on_request: started processing request
//! ERROR request{method=POST uri=/users matched_path="/users"}: example_error_handling: error from time_library err=failed to get time
//! DEBUG request{method=POST uri=/users matched_path="/users"}: tower_http::trace::on_response: finished processing request latency=0 ms status=500
//! ```

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use axum::{
    extract::{rejection::JsonRejection, FromRequest, MatchedPath, Request, State},
    http::StatusCode,
    middleware::{from_fn, Next},
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use time_library::Timestamp;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = AppState::default();

    let app = Router::new()
        // A dummy route that accepts some JSON but sometimes fails
        .route("/users", post(users_create))
        .layer(
            TraceLayer::new_for_http()
                // Create our own span for the request and include the matched path. The matched
                // path is useful for figuring out which handler the request was routed to.
                .make_span_with(|req: &Request| {
                    let method = req.method();
                    let uri = req.uri();

                    // axum automatically adds this extension.
                    let matched_path = req
                        .extensions()
                        .get::<MatchedPath>()
                        .map(|matched_path| matched_path.as_str());

                    tracing::debug_span!("request", %method, %uri, matched_path)
                })
                // By default `TraceLayer` will log 5xx responses but we're doing our specific
                // logging of errors so disable that
                .on_failure(()),
        )
        .layer(from_fn(log_app_errors))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

#[derive(Default, Clone)]
struct AppState {
    next_id: Arc<AtomicU64>,
    users: Arc<Mutex<HashMap<u64, User>>>,
}

#[derive(Deserialize)]
struct UserParams {
    name: String,
}

#[derive(Serialize, Clone)]
struct User {
    id: u64,
    name: String,
    created_at: Timestamp,
}

async fn users_create(
    State(state): State<AppState>,
    // Make sure to use our own JSON extractor so we get input errors formatted in a way that
    // matches our application
    AppJson(params): AppJson<UserParams>,
) -> Result<AppJson<User>, AppError> {
    let id = state.next_id.fetch_add(1, Ordering::SeqCst);

    // We have implemented `From<time_library::Error> for AppError` which allows us to use `?` to
    // automatically convert the error
    let created_at = Timestamp::now()?;

    let user = User {
        id,
        name: params.name,
        created_at,
    };

    state.users.lock().unwrap().insert(id, user.clone());

    Ok(AppJson(user))
}

// Create our own JSON extractor by wrapping `axum::Json`. This makes it easy to override the
// rejection and provide our own which formats errors to match our application.
//
// `axum::Json` responds with plain text if the input is invalid.
#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(AppError))]
struct AppJson<T>(T);

impl<T> IntoResponse for AppJson<T>
where
    axum::Json<T>: IntoResponse,
{
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}

// The kinds of errors we can hit in our application.
#[derive(Debug)]
enum AppError {
    // The request body contained invalid JSON
    JsonRejection(JsonRejection),
    // Some error from a third party library we're using
    TimeError(time_library::Error),
}

// Tell axum how `AppError` should be converted into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // How we want errors responses to be serialized
        #[derive(Serialize)]
        struct ErrorResponse {
            message: String,
        }

        let (status, message, err) = match &self {
            AppError::JsonRejection(rejection) => {
                // This error is caused by bad user input so don't log it
                (rejection.status(), rejection.body_text(), None)
            }
            AppError::TimeError(_err) => {
                // While we could simply log the error here we would introduce
                // a side-effect to our conversion, instead add the AppError to
                // the Response as an Extension
                // Don't expose any details about the error to the client
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Something went wrong".to_owned(),
                    Some(self),
                )
            }
        };

        let mut response = (status, AppJson(ErrorResponse { message })).into_response();
        if let Some(err) = err {
            // Insert our error into the response, our logging middleware will use this.
            // By wrapping the error in an Arc we can use it as an Extension regardless of any inner types not deriving Clone.
            response.extensions_mut().insert(Arc::new(err));
        }
        response
    }
}

impl From<JsonRejection> for AppError {
    fn from(rejection: JsonRejection) -> Self {
        Self::JsonRejection(rejection)
    }
}

impl From<time_library::Error> for AppError {
    fn from(error: time_library::Error) -> Self {
        Self::TimeError(error)
    }
}

// Our middleware is responsible for logging error details internally
async fn log_app_errors(request: Request, next: Next) -> Response {
    let response = next.run(request).await;
    // If the response contains an AppError Extension, log it.
    if let Some(err) = response.extensions().get::<Arc<AppError>>() {
        tracing::error!(?err, "an unexpected error occurred inside a handler");
    }
    response
}

// Imagine this is some third party library that we're using. It sometimes returns errors which we
// want to log.
mod time_library {
    use std::sync::atomic::{AtomicU64, Ordering};

    use serde::Serialize;

    #[derive(Serialize, Clone)]
    pub struct Timestamp(u64);

    impl Timestamp {
        pub fn now() -> Result<Self, Error> {
            static COUNTER: AtomicU64 = AtomicU64::new(0);

            // Fail on every third call just to simulate errors
            if COUNTER.fetch_add(1, Ordering::SeqCst).is_multiple_of(3) {
                Err(Error::FailedToGetTime)
            } else {
                Ok(Self(1337))
            }
        }
    }

    #[derive(Debug, Clone)]
    pub enum Error {
        FailedToGetTime,
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "failed to get time")
        }
    }
}
````

## Form

```rust
use axum::{extract::Form, response::Html, routing::get, Router};
use serde::Deserialize;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug", env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // build our application with some routes
    let app = app();

    // run it
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

fn app() -> Router {
    Router::new().route("/", get(show_form).post(accept_form))
}

async fn show_form() -> Html<&'static str> {
    Html(
        r#"
        <!doctype html>
        <html>
            <head></head>
            <body>
                <form action="/" method="post">
                    <label for="name">
                        Enter your name:
                        <input type="text" name="name">
                    </label>

                    <label>
                        Enter your email:
                        <input type="text" name="email">
                    </label>

                    <input type="submit" value="Subscribe!">
                </form>
            </body>
        </html>
        "#,
    )
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct Input {
    name: String,
    email: String,
}

async fn accept_form(Form(input): Form<Input>) -> Html<String> {
    dbg!(&input);
    Html(format!(
        "email='{}'\nname='{}'\n",
        &input.email, &input.name
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt; // for `call`, `oneshot`, and `ready` // for `collect`

    #[tokio::test]
    async fn test_get() {
        let app = app();

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body = std::str::from_utf8(&body).unwrap();

        assert!(body.contains(r#"<input type="submit" value="Subscribe!">"#));
    }

    #[tokio::test]
    async fn test_post() {
        let app = app();

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/")
                    .header(
                        http::header::CONTENT_TYPE,
                        mime::APPLICATION_WWW_FORM_URLENCODED.as_ref(),
                    )
                    .body(Body::from("name=foo&email=bar@axum"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body = std::str::from_utf8(&body).unwrap();

        assert_eq!(body, "email='bar@axum'\nname='foo'\n");
    }
}
```

## Graceful shutdown

```rust
use std::time::Duration;

use axum::{routing::get, Router};
use tokio::net::TcpListener;
use tokio::signal;
use tokio::time::sleep;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Enable tracing.
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!(
                    "{}=debug,tower_http=debug,axum=trace",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer().without_time())
        .init();

    // Create a regular axum app.
    let app = Router::new()
        .route("/slow", get(|| sleep(Duration::from_secs(5))))
        .route("/forever", get(std::future::pending::<()>))
        .layer((
            TraceLayer::new_for_http(),
            // Graceful shutdown will wait for outstanding requests to complete. Add a timeout so
            // requests don't hang forever.
            TimeoutLayer::new(Duration::from_secs(10)),
        ));

    // Create a `TcpListener` using tokio.
    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();

    // Run the server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
```
