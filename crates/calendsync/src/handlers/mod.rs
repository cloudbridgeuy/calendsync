pub mod calendar_react;
pub mod calendars;
#[cfg(debug_assertions)]
pub mod dev;
pub mod entries;
pub mod error;
pub mod events;
#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
pub mod flash;
pub mod health;
#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
pub mod login;
#[cfg(any(feature = "auth-sqlite", feature = "auth-redis", feature = "auth-mock"))]
pub mod root;
pub mod static_files;

pub use error::AppError;
