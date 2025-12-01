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
    let ts_dir = frontend_dir.join("src/ts");

    // Rerun if TypeScript sources change
    println!("cargo:rerun-if-changed=src/ts/");
    println!("cargo:rerun-if-changed=package.json");
    println!("cargo:rerun-if-changed=tsconfig.json");

    // Check if TypeScript sources exist
    if !ts_dir.exists() {
        eprintln!("Warning: No TypeScript sources found at {ts_dir:?}");
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

            // Match pattern: name-hash.js (exclude source maps)
            if filename.ends_with(".js") && !filename.ends_with(".js.map") {
                // Extract original name (e.g., "index" from "index-a1b2c3d4.js")
                if let Some(dash_pos) = filename.rfind('-') {
                    let original_name = &filename[..dash_pos];
                    manifest.insert(format!("{original_name}.js"), json!(filename));
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
