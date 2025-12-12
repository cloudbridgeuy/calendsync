use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let frontend_dir = Path::new(&manifest_dir);
    let dist_dir = frontend_dir.join("dist");
    let manifest_path = frontend_dir.join("manifest.json");
    let src_dir = frontend_dir.join("src");

    // Rerun if TypeScript/React sources change
    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=package.json");
    println!("cargo:rerun-if-changed=tsconfig.json");

    // Check if source directory exists
    if !src_dir.exists() {
        eprintln!("Warning: No sources found at {src_dir:?}");
        // Create empty manifest
        fs::write(&manifest_path, "{}").expect("Failed to write empty manifest.json");
        return;
    }

    // Clean dist directory
    if dist_dir.exists() {
        fs::remove_dir_all(&dist_dir).expect("Failed to clean dist directory");
    }
    fs::create_dir_all(&dist_dir).expect("Failed to create dist directory");

    // Determine build mode based on profile
    let profile = env::var("PROFILE").unwrap_or_default();
    let build_script = if profile == "release" {
        "build"
    } else {
        "build:dev"
    };

    // Run bun build
    let status = Command::new("bun")
        .args(["run", build_script])
        .current_dir(frontend_dir)
        .status()
        .expect("Failed to run bun build. Is bun installed?");

    if !status.success() {
        panic!("bun build failed");
    }

    // Generate manifest.json by scanning dist directory
    let mut manifest: HashMap<String, Value> = HashMap::new();

    if dist_dir.exists() {
        for entry in fs::read_dir(&dist_dir).expect("Failed to read dist directory") {
            let entry = entry.expect("Failed to read entry");
            let filename = entry.file_name().to_string_lossy().to_string();

            // Handle JS files (exclude source maps)
            if filename.ends_with(".js") && !filename.ends_with(".js.map") {
                // Check if it's a hashed file (contains hash before .js)
                if let Some(dash_pos) = filename.rfind('-') {
                    // Hashed file: name-hash.js -> name.js
                    let original_name = &filename[..dash_pos];
                    manifest.insert(format!("{original_name}.js"), json!(filename));
                } else {
                    // Non-hashed file: name.js -> name.js (e.g., calendar-react-server.js)
                    manifest.insert(filename.clone(), json!(filename));
                }
            }

            // Handle CSS files (now hashed like JS files)
            if filename.ends_with(".css") && !filename.ends_with(".css.map") {
                // Check if it's a hashed file (contains hash before .css)
                if let Some(dash_pos) = filename.rfind('-') {
                    // Hashed file: name-hash.css -> name.css
                    let original_name = &filename[..dash_pos];
                    manifest.insert(format!("{original_name}.css"), json!(filename));
                } else {
                    // Non-hashed file: name.css -> name.css
                    manifest.insert(filename.clone(), json!(filename));
                }
            }
        }
    }

    // Write manifest.json
    let manifest_json =
        serde_json::to_string_pretty(&manifest).expect("Failed to serialize manifest");
    fs::write(&manifest_path, &manifest_json).expect("Failed to write manifest.json");

    println!("cargo:warning=Frontend build complete. Manifest: {manifest:?}");
}
