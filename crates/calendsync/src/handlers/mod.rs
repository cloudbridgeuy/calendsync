pub mod calendar_react;
#[cfg(debug_assertions)]
pub mod dev;
pub mod entries;
pub mod error;
pub mod events;
pub mod health;
pub mod static_files;

pub use error::AppError;
