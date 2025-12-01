//! Static file serving handler for JavaScript/CSS assets.

use axum::{
    body::Body,
    extract::Path,
    http::{header, Response, StatusCode},
    response::IntoResponse,
};
use std::fs;

/// Serve static files from the dist directory.
pub async fn serve_static(Path(filename): Path<String>) -> impl IntoResponse {
    // Build path to the dist directory (symlinked from frontend)
    let dist_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/dist");
    let file_path = format!("{dist_dir}/{filename}");

    // Read the file
    match fs::read(&file_path) {
        Ok(contents) => {
            // Determine content type based on extension
            let content_type = if filename.ends_with(".js") {
                "application/javascript; charset=utf-8"
            } else if filename.ends_with(".css") {
                "text/css; charset=utf-8"
            } else if filename.ends_with(".map") {
                "application/json"
            } else {
                "application/octet-stream"
            };

            // Set cache headers for hashed files (immutable)
            let is_hashed = filename.contains('-') && !filename.ends_with(".map");
            let cache_control = if is_hashed {
                "public, max-age=31536000, immutable"
            } else {
                "public, max-age=3600"
            };

            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, content_type)
                .header(header::CACHE_CONTROL, cache_control)
                .body(Body::from(contents))
                .unwrap()
        }
        Err(_) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not found"))
            .unwrap(),
    }
}
