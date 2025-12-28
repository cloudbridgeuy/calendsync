# cargo-rail Integration Design

This design integrates cargo-rail into calendsync's development workflow to address two problems: CI waste from testing unchanged crates, and dependency drift across the workspace. The integration follows a CI-first approach that replaces cargo-machete with cargo-rail's `unify` command while adding graph-aware test selection via the `affected` command.

## Overview and Goals

The workspace contains seven crates with varying dependency relationships. The `calendsync_core` crate sits at the foundation; changes there affect `calendsync`, `client`, and potentially `ssr_core`. The `frontend` crate, by contrast, has no downstream dependents. The `src-tauri` crate depends on `frontend` but nothing depends on `src-tauri`. This structure means focused PRs, which represent most changes, touch only 1-2 crates and their immediate dependents.

The integration achieves three goals. First, CI runs tests only for affected crates, reducing test time proportionally to the change scope. A PR touching only `frontend` tests one crate instead of seven. Second, dependency unification replaces cargo-machete and adds feature pruning and version synchronization that cargo-machete cannot provide. Third, the local development experience remains unchanged; developers continue using `cargo xtask lint` without installing cargo-rail locally, though they may optionally install it for deeper dependency analysis.

The design requires adding a `rail.toml` configuration file, modifying the CI workflow, and updating the xtask lint command. No changes to application code or Cargo.toml dependency declarations occur during initial integration; cargo-rail analyzes and reports but does not modify manifests unless explicitly invoked.

## Configuration

The integration requires a `rail.toml` configuration file at `.config/rail.toml`, which aligns with cargo-rail's default search path and keeps the workspace root clean. This file defines target platforms, unification behavior, and change detection rules.

The target configuration specifies the platforms calendsync supports. Based on the release workflow, which builds for `x86_64-apple-darwin` and `aarch64-apple-darwin`, these two targets form the initial set. Linux targets remain commented out since the release workflow has them disabled. When Linux support becomes active, adding the target to `rail.toml` enables multi-platform dependency analysis.

The unification section enables unused dependency detection and dead feature pruning while disabling transitive pinning. Transitive pinning replaces cargo-hakari's workspace-hack pattern, which calendsync does not use. MSRV computation remains enabled with `msrv_source = "max"`, meaning cargo-rail takes the higher of the workspace's declared rust-version and the minimum required by dependencies.

The change detection section defines infrastructure files that trigger full workspace rebuilds. Changes to CI workflows, the Cargo.lock, rust-toolchain.toml, or xtask code affect the entire workspace and should not use selective testing. The configuration also defines a custom `frontend` category for TypeScript-only changes, enabling CI to skip Rust tests entirely when only frontend code changes.

```toml
targets = ["x86_64-apple-darwin", "aarch64-apple-darwin"]

[unify]
detect_unused = true
remove_unused = false  # Report only; manual removal
prune_dead_features = true
msrv = true
msrv_source = "max"
pin_transitives = false

[change-detection]
infrastructure = [
    ".github/**",
    "Cargo.lock",
    "rust-toolchain.toml",
    "xtask/**",
]

[change-detection.custom]
frontend = ["crates/frontend/src/**", "crates/frontend/package.json"]
```

## CI Workflow Changes

The CI workflow transforms from testing everything to testing only affected crates. The current `ci.yml` runs four independent jobs: test, build, unused-deps, and typos. The new workflow restructures these into a detection phase followed by conditional execution.

A new `detect` job runs first and uses `cargo-rail-action` to determine which crates changed. This job produces outputs that downstream jobs consume: the list of affected crates in cargo-args format, flags indicating whether infrastructure changed or only documentation/frontend changed, and the count of affected crates. The action installs cargo-rail from pre-built binaries in approximately three seconds, adding minimal overhead.

The test job becomes conditional. When infrastructure files change, it runs `cargo test --workspace` as before. When only specific crates change, it runs `cargo test` with the `-p crate1 -p crate2` flags provided by the detection job. When only frontend files change and no Rust crates are affected, it skips Rust tests entirely. The clippy and fmt checks continue running on the full workspace since they execute quickly and catch issues in unchanged code that might affect changed code.

The unused-deps job transforms from cargo-machete to `cargo rail unify --check`. This change provides three improvements: detection of version mismatches across the workspace, identification of dead features that cargo-machete cannot find, and MSRV validation against the dependency graph. The check flag ensures the job only reports issues without modifying files; CI should never auto-fix.

The build and typos jobs remain unchanged. Build verification runs on macOS as before, and typo checking has no relationship to the dependency graph.

## xtask Lint Modifications

The xtask lint command requires one change: replacing the `cargo machete` step with `cargo rail unify --check`. The remaining nine checks stay intact. This substitution provides equivalent unused dependency detection while adding version unification and dead feature checks.

The implementation adds a new function `run_cargo_rail_unify` that mirrors the structure of `run_cargo_machete`. The function invokes `cargo rail unify --check` and interprets its exit code. Exit code zero indicates no drift; exit code one indicates issues found. The function reports success or failure using the existing logging helpers and returns a boolean like other check functions.

A prerequisite check for the `cargo-rail` binary joins the existing checks for `cargo`, `cargo-machete`, and `bun` at the start of `run_lint_checks`. The error message directs users to install via `cargo install cargo-rail` or `cargo binstall cargo-rail`. Since cargo-rail is an optional enhancement for local development, the check could alternatively skip the unify step if cargo-rail is not installed, but requiring installation maintains parity with CI behavior.

The removal of `cargo machete` simplifies dependencies. The `require_command` call for `cargo-machete` disappears, and developers no longer need that tool installed. The CLAUDE.md documentation updates to reflect this change, removing cargo-machete from the lint checks description and adding cargo-rail.

The lint command's help text updates to describe the new step: "cargo rail unify --check - Dependency unification, unused deps, dead features" replaces the cargo-machete description. The numbered list in the long help shifts accordingly.

## Error Handling and Failure Modes

Both CI and local lint runs must handle cargo-rail failures gracefully. The tool can fail for several reasons, and each requires appropriate handling.

When `cargo rail unify --check` detects drift, it exits with code one and prints a report describing the issues: unused dependencies, version mismatches, dead features, or undeclared features. In CI, this fails the job and the report appears in the job output. Locally, xtask lint displays the failure alongside other check failures and suggests running `cargo rail unify` to auto-fix. The fix remains manual; neither CI nor the pre-commit hook should modify Cargo.toml files automatically.

When cargo-rail cannot parse the workspace, it exits with a non-zero code and prints an error. This occurs if Cargo.toml files contain syntax errors or if the workspace structure is invalid. The error propagates as a lint failure, blocking the PR or commit. This behavior matches how cargo check failures propagate today.

When the `cargo-rail` binary is missing locally, xtask lint reports the missing dependency and exits early. The error message provides installation instructions. This matches the existing pattern for missing `cargo-machete` and `bun` dependencies.

When cargo-rail-action fails in CI, the detect job fails and downstream jobs do not run. This conservative behavior prevents testing with incomplete information. The action's error output indicates whether the failure stems from installation issues, git history problems (insufficient fetch depth), or cargo-rail execution errors.

Network failures during cargo-rail-action's binary download cause installation fallback to `cargo install`, which takes longer but succeeds if the Rust toolchain is available.

## File Changes Summary

The integration requires creating one new file, modifying three existing files, and removing one dependency. No application code changes occur.

**New file: `.config/rail.toml`**

The configuration file described in the Configuration section. This file commits to the repository and defines the workspace's cargo-rail behavior for both CI and local development.

**Modified file: `.github/workflows/ci.yml`**

The CI workflow gains a `detect` job using `cargo-rail-action`, conditional logic in the `test` job based on affected crates, and replacement of the `unused-deps` job's cargo-machete invocation with `cargo rail unify --check`. The build and typos jobs remain unchanged.

**Modified file: `xtask/src/lint/mod.rs`**

The lint module replaces `run_cargo_machete` with `run_cargo_rail_unify`, updates the prerequisite check from `cargo-machete` to `cargo-rail`, and adjusts help text. The function count stays the same; only the implementation of one function changes.

**Modified file: `CLAUDE.md`**

The documentation updates the lint checks description, replacing cargo-machete with cargo-rail in the numbered list. The Progressive Disclosure table already references `.claude/context/cargo-rail.md` from the earlier documentation work.

**Removed dependency: `cargo-machete`**

Developers no longer need cargo-machete installed. The CI workflow removes the `bnjbvr/cargo-machete@main` action. This simplifies the toolchain requirements.

The total change scope is small: approximately 50-80 lines of YAML changes in CI, 30-40 lines of Rust changes in xtask, 20 lines of configuration, and minor documentation updates.

## Testing the Integration

Before merging, the integration requires validation across three scenarios: CI behavior, local lint behavior, and edge cases.

**CI validation** involves creating a test branch with the new workflow and verifying the detect job correctly identifies affected crates. A commit touching only `crates/frontend/src` should report only `calendsync_frontend` as affected. A commit touching `crates/core/src` should report `calendsync_core` plus its dependents: `calendsync`, `client`, and any others that depend on core. A commit touching `.github/workflows/ci.yml` should set `rebuild-all` to true and trigger full workspace tests. The `cargo rail unify --check` step should pass on a clean workspace and fail if an unused dependency is temporarily added.

**Local lint validation** requires installing cargo-rail locally and running `cargo xtask lint`. The output should show the new unify check in place of machete. Intentionally adding an unused dependency to a crate's Cargo.toml should cause the check to fail with a clear message. Running `cargo rail unify` directly should fix the issue, and subsequent lint runs should pass.

**Edge case validation** covers the failure scenarios described earlier. Running lint without cargo-rail installed should produce a helpful error message. A malformed Cargo.toml should fail both cargo check and cargo-rail with understandable errors. The CI workflow with `fetch-depth: 0` should provide sufficient git history for change detection; shallow clones would cause the action to fail with a clear message about fetch depth.

The existing test suite requires no changes since this integration affects only tooling, not application behavior.
