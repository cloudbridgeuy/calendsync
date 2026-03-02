use crate::prelude::*;
use error::Result;

pub mod error;
mod hooks;

// ---------------------------------------------------------------------------
// Functional Core — pure types, const data, and functions (no I/O)
// ---------------------------------------------------------------------------

/// Identifies each lint check in the pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckId {
    Fmt,
    Check,
    Clippy,
    Test,
    Rail,
    BiomeFrontend,
    TypecheckFrontend,
    TestFrontend,
    BiomeExample,
    TypecheckExample,
}

/// Declarative description of a single lint check.
#[derive(Debug)]
pub struct Check {
    pub id: CheckId,
    pub name: &'static str,
    pub program: &'static str,
    pub default_args: &'static [&'static str],
    pub optional: bool,
    pub cwd: Option<&'static str>,
}

/// Outcome of running a single check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckOutcome {
    Passed { output: String },
    Failed { output: String },
    Skipped,
}

/// A check paired with its outcome.
#[derive(Debug, Clone)]
pub struct CheckResult {
    pub name: &'static str,
    pub outcome: CheckOutcome,
}

/// All lint checks, executed in order: Rust checks first, then TypeScript checks.
const CHECKS: &[Check] = &[
    // Rust checks (1–5)
    Check {
        id: CheckId::Fmt,
        name: "cargo fmt --check",
        program: "cargo",
        default_args: &["fmt", "--check"],
        optional: false,
        cwd: None,
    },
    Check {
        id: CheckId::Check,
        name: "cargo check --all-targets",
        program: "cargo",
        default_args: &["check", "--all-targets"],
        optional: false,
        cwd: None,
    },
    Check {
        id: CheckId::Clippy,
        name: "cargo clippy ... -D warnings",
        program: "cargo",
        default_args: &["clippy", "--all-targets", "--", "-D", "warnings"],
        optional: false,
        cwd: None,
    },
    Check {
        id: CheckId::Test,
        name: "cargo test --all-targets",
        program: "cargo",
        default_args: &["test", "--all-targets"],
        optional: false,
        cwd: None,
    },
    Check {
        id: CheckId::Rail,
        name: "cargo rail unify --check",
        program: "cargo",
        default_args: &["rail", "unify", "--check"],
        optional: true,
        cwd: None,
    },
    // TypeScript checks (6–10)
    Check {
        id: CheckId::BiomeFrontend,
        name: "biome check (frontend)",
        program: "bunx",
        default_args: &["biome", "check", "--write", "--unsafe"],
        optional: false,
        cwd: Some("crates/frontend"),
    },
    Check {
        id: CheckId::TypecheckFrontend,
        name: "typecheck (frontend)",
        program: "bun",
        default_args: &["run", "typecheck"],
        optional: false,
        cwd: Some("crates/frontend"),
    },
    Check {
        id: CheckId::TestFrontend,
        name: "bun test (frontend)",
        program: "bun",
        default_args: &["test"],
        optional: false,
        cwd: Some("crates/frontend"),
    },
    Check {
        id: CheckId::BiomeExample,
        name: "biome check (example)",
        program: "bunx",
        default_args: &["biome", "check", "--write", "--unsafe"],
        optional: false,
        cwd: Some("crates/calendsync/examples/react-ssr"),
    },
    Check {
        id: CheckId::TypecheckExample,
        name: "typecheck (example)",
        program: "bun",
        default_args: &["run", "typecheck"],
        optional: false,
        cwd: Some("crates/calendsync/examples/react-ssr"),
    },
];

/// Determine whether a check should be skipped.
///
/// An optional check is skipped when its tool is not installed.
fn should_skip(optional: bool, tool_installed: bool) -> bool {
    optional && !tool_installed
}

/// Compute the effective arguments for a check, accounting for `--fix` mode.
///
/// - `Fmt`: drops `--check` so `cargo fmt` runs in-place
/// - `Clippy`: adds `--fix` and `--allow-dirty` before the `--` separator
fn fix_args<'a>(id: CheckId, default_args: &'a [&'a str], fix_mode: bool) -> Vec<&'a str> {
    if !fix_mode {
        return default_args.to_vec();
    }

    match id {
        CheckId::Fmt => default_args
            .iter()
            .copied()
            .filter(|a| *a != "--check")
            .collect(),
        CheckId::Clippy => {
            let mut args = Vec::new();
            for &arg in default_args {
                if arg == "--" {
                    args.extend_from_slice(&["--fix", "--allow-dirty", "--"]);
                } else {
                    args.push(arg);
                }
            }
            args
        }
        _ => default_args.to_vec(),
    }
}

/// Format a display name for terminal output (e.g., "Running cargo fmt --check...").
fn check_display_name(name: &str) -> String {
    format!("Running {name}...")
}

/// Check whether a check should be skipped based on CLI skip flags.
fn should_skip_by_flag(id: CheckId, no_biome: bool, no_typecheck: bool, no_bun_test: bool) -> bool {
    match id {
        CheckId::BiomeFrontend | CheckId::BiomeExample => no_biome,
        CheckId::TypecheckFrontend | CheckId::TypecheckExample => no_typecheck,
        CheckId::TestFrontend => no_bun_test,
        _ => false,
    }
}

/// Heuristic: does the captured output indicate the command was not found?
fn is_tool_not_found(output: &str) -> bool {
    let lower = output.to_lowercase();
    lower.contains("command not found")
        || lower.contains("no such file or directory")
        || lower.contains("not found in path")
        || lower.contains("error: no such command")
        || lower.contains("error: no such subcommand")
}

/// Determine the outcome of a check from its exit code, output, and metadata.
fn determine_outcome(
    exit_ok: bool,
    output: String,
    optional: bool,
    tool_not_found: bool,
) -> CheckOutcome {
    if exit_ok {
        return CheckOutcome::Passed { output };
    }

    if optional && tool_not_found {
        return CheckOutcome::Skipped;
    }

    CheckOutcome::Failed { output }
}

/// Format a single check result as a log entry (for the log file).
fn format_log_entry(result: &CheckResult) -> String {
    match &result.outcome {
        CheckOutcome::Passed { output } => {
            format!(
                "--- {} [PASS] ---\n{}\n",
                result.name,
                if output.is_empty() {
                    "(no output)"
                } else {
                    output.as_str()
                }
            )
        }
        CheckOutcome::Failed { output } => {
            format!(
                "--- {} [FAIL] ---\n{}\n",
                result.name,
                if output.is_empty() {
                    "(no output)"
                } else {
                    output.as_str()
                }
            )
        }
        CheckOutcome::Skipped => {
            format!("--- {} [SKIP] ---\n", result.name)
        }
    }
}

/// Format the final "log: <path>" line printed to the terminal.
fn format_log_path_line(path: &std::path::Path) -> String {
    format!("log: {}", path.display())
}

// ---------------------------------------------------------------------------
// Public interface
// ---------------------------------------------------------------------------

/// Code quality checks and git hooks management
#[derive(Debug, clap::Parser)]
#[command(
    long_about = "Run code quality checks including formatting, compilation, linting, and dependency audits.

This command runs the following checks in order:

Rust checks:
 1. cargo fmt - Code formatting (auto-fix with --fix)
 2. cargo check - Compilation check
 3. cargo clippy - Linting with all warnings treated as errors
 4. cargo test - Run all tests including doctests
 5. cargo rail unify --check - Dependency unification, unused deps, dead features

TypeScript checks (crates/frontend):
 6. biome check --write --unsafe - Format and lint with auto-fix
 7. bun run typecheck - TypeScript type checking
 8. bun test - Run TypeScript tests

TypeScript checks (examples/hello-world):
 9. biome check --write --unsafe - Format and lint example TypeScript
10. bun run typecheck - Example TypeScript type checking

When used with --install-hooks, this command also manages git pre-commit hooks that
run these same checks automatically before each commit.

The pre-commit hook will:
- Only check staged Rust files by default
- Auto-fix formatting issues and re-stage files
- Block commits if checks fail
- Support --force flag to check all files"
)]
pub struct LintCommand {
    /// Auto-fix issues when possible (applies to fmt and clippy)
    #[arg(long)]
    pub fix: bool,

    /// Check all files instead of just staged files (for hooks)
    #[arg(long)]
    pub force: bool,

    /// Only check staged files (used by git hooks)
    #[arg(long, hide = true)]
    pub staged_only: bool,

    /// Install git pre-commit hooks
    #[arg(long, conflicts_with_all = &["uninstall_hooks", "hooks_status", "test_hooks"])]
    pub install_hooks: bool,

    /// Uninstall git pre-commit hooks
    #[arg(long, conflicts_with_all = &["install_hooks", "hooks_status", "test_hooks"])]
    pub uninstall_hooks: bool,

    /// Show git hooks installation status
    #[arg(long, conflicts_with_all = &["install_hooks", "uninstall_hooks", "test_hooks"])]
    pub hooks_status: bool,

    /// Test installed git hooks
    #[arg(long, conflicts_with_all = &["install_hooks", "uninstall_hooks", "hooks_status"])]
    pub test_hooks: bool,

    /// Skip biome checks
    #[arg(long)]
    pub no_biome: bool,

    /// Skip TypeScript type checking
    #[arg(long)]
    pub no_typecheck: bool,

    /// Skip bun test checks
    #[arg(long)]
    pub no_bun_test: bool,
}

// ---------------------------------------------------------------------------
// Imperative Shell — pipeline orchestration, command execution, file I/O
// ---------------------------------------------------------------------------

/// Entry point called from `main.rs`. Async because hooks are async.
pub async fn run(command: LintCommand, global: crate::Global) -> Result<()> {
    // Handle hooks management commands (async — delegates to hooks module)
    if command.install_hooks {
        return hooks::install_hooks(&global).await;
    }
    if command.uninstall_hooks {
        return hooks::uninstall_hooks(&global).await;
    }
    if command.hooks_status {
        return hooks::show_status().await;
    }
    if command.test_hooks {
        return hooks::test_hooks().await;
    }

    // Run the synchronous lint pipeline
    run_pipeline(&command, &global)
}

/// Resolve the log file path (`target/xtask-lint.log`).
fn resolve_log_path() -> std::path::PathBuf {
    project_root().join("target/xtask-lint.log")
}

/// Collect staged `.rs` files for re-staging after `cargo fmt` in `--staged-only` mode.
fn collect_staged_rust_files() -> Vec<String> {
    let output = std::process::Command::new("git")
        .args(["diff", "--cached", "--name-only", "--diff-filter=ACM"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            text.lines()
                .filter(|l| l.ends_with(".rs"))
                .map(|l| l.to_string())
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Re-stage previously-staged `.rs` files after `cargo fmt` modifies them.
fn restage_files(files: &[String], verbose: bool) {
    if files.is_empty() {
        return;
    }

    let status = std::process::Command::new("git")
        .arg("add")
        .args(files)
        .status();

    if verbose {
        if let Ok(s) = status {
            if s.success() {
                aprintln!("{} Re-staged {} Rust files", p_b("info:"), files.len());
            }
        }
    }
}

/// Check whether `cargo-rail` is installed.
fn is_cargo_rail_installed() -> bool {
    std::process::Command::new("cargo")
        .args(["rail", "--version"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Resolve the project root from `CARGO_MANIFEST_DIR`.
fn project_root() -> &'static std::path::Path {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    std::path::Path::new(manifest_dir)
        .parent()
        .expect("xtask manifest should have parent directory")
}

/// Resolve an optional relative `cwd` to an absolute path against the project root.
fn resolve_cwd(cwd: Option<&str>) -> Option<std::path::PathBuf> {
    cwd.map(|dir| project_root().join(dir))
}

/// Check whether a resolved cwd directory exists.
///
/// Returns `true` when `cwd` is `Some` and the resolved path is not an existing directory,
/// meaning the check should be skipped gracefully.
fn should_skip_missing_cwd(cwd: Option<&std::path::Path>) -> bool {
    match cwd {
        Some(dir) => !dir.is_dir(),
        None => false,
    }
}

/// Execute a single check via `duct`, capturing all output.
///
/// If `cwd` is `Some`, the relative path is resolved against the project root.
fn run_check(program: &str, args: &[&str], cwd: Option<&str>) -> (bool, String) {
    let resolved_cwd = resolve_cwd(cwd);

    let expr = duct::cmd(program, args)
        .stderr_to_stdout()
        .stdout_capture()
        .unchecked();

    let expr = if let Some(ref dir) = resolved_cwd {
        expr.dir(dir)
    } else {
        expr
    };

    match expr.run() {
        Ok(output) => {
            let text = String::from_utf8_lossy(&output.stdout).to_string();
            (output.status.success(), text)
        }
        Err(e) => (false, format!("Failed to execute {program}: {e}")),
    }
}

/// The synchronous lint pipeline.
///
/// NOTE: This pipeline uses quiet-by-default (only `--verbose` prints passes).
/// The `--silent` global flag is intentionally ignored here — other xtask modules
/// (hooks, dev, seed) still use it, but the lint pipeline's capture-first design
/// makes it redundant.
fn run_pipeline(command: &LintCommand, global: &crate::Global) -> Result<()> {
    use std::io::Write;

    let log_path = resolve_log_path();
    let mut log_file = std::fs::File::create(&log_path)?;

    // Collect staged files before any check modifies them
    let staged_files = if command.staged_only {
        collect_staged_rust_files()
    } else {
        Vec::new()
    };

    let rail_installed = is_cargo_rail_installed();
    let mut failed = false;

    for check in CHECKS {
        // Skip logic: pre-skip (tool probe) and post-skip (determine_outcome
        // detects "not found" in output) provide defense-in-depth for optional checks.
        let tool_installed = match check.id {
            CheckId::Rail => rail_installed,
            _ => true,
        };

        if should_skip(check.optional, tool_installed)
            || should_skip_by_flag(
                check.id,
                command.no_biome,
                command.no_typecheck,
                command.no_bun_test,
            )
        {
            let result = CheckResult {
                name: check.name,
                outcome: CheckOutcome::Skipped,
            };
            write!(log_file, "{}", format_log_entry(&result))?;
            if global.is_verbose() {
                aprintln!("{} {} {}", p_y("--"), check.name, p_y("[skip]"));
            }
            continue;
        }

        // Skip checks whose cwd directory does not exist (e.g., example dir missing)
        let resolved = resolve_cwd(check.cwd);
        if should_skip_missing_cwd(resolved.as_deref()) {
            let result = CheckResult {
                name: check.name,
                outcome: CheckOutcome::Skipped,
            };
            write!(log_file, "{}", format_log_entry(&result))?;
            if global.is_verbose() {
                aprintln!(
                    "{} {} {} (directory not found)",
                    p_y("--"),
                    check.name,
                    p_y("[skip]")
                );
            }
            continue;
        }

        // Show "Running ..." if verbose
        if global.is_verbose() {
            aprintln!("{}", p_b(&check_display_name(check.name)));
        }

        // Compute effective args (staged_only implies fix for Fmt so hooks auto-format)
        let fix_mode = command.fix || (command.staged_only && check.id == CheckId::Fmt);
        let args = fix_args(check.id, check.default_args, fix_mode);

        // Execute
        let (exit_ok, output) = run_check(check.program, &args, check.cwd);

        // Determine outcome
        let tool_not_found = is_tool_not_found(&output);
        let outcome = determine_outcome(exit_ok, output, check.optional, tool_not_found);

        let result = CheckResult {
            name: check.name,
            outcome,
        };

        // Write to log
        write!(log_file, "{}", format_log_entry(&result))?;

        // Terminal routing
        match &result.outcome {
            CheckOutcome::Passed { .. } => {
                if global.is_verbose() {
                    aprintln!("{} {} {}", p_g("ok"), check.name, p_g("[pass]"));
                }
            }
            CheckOutcome::Failed { output } => {
                aprintln!("{} {} {}", p_r("!!"), check.name, p_r("[fail]"));
                aprintln!("{output}");
                failed = true;
            }
            CheckOutcome::Skipped => {
                if global.is_verbose() {
                    aprintln!("{} {} {}", p_y("--"), check.name, p_y("[skip]"));
                }
            }
        }

        // Re-stage after fmt in staged-only mode
        if check.id == CheckId::Fmt && command.staged_only && !failed {
            restage_files(&staged_files, global.is_verbose());
        }

        // Early exit on first failure
        if failed {
            aprintln!("{}", format_log_path_line(&log_path));
            return Err(error::LintError::ChecksFailed);
        }
    }

    // All passed
    if global.is_verbose() {
        aprintln!();
        aprintln!("{}", p_g("All checks passed."));
    }
    aprintln!("{}", format_log_path_line(&log_path));

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests — Functional Core only
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- should_skip ---

    #[test]
    fn required_check_never_skipped() {
        assert!(!should_skip(false, true));
        assert!(!should_skip(false, false));
    }

    #[test]
    fn optional_check_skipped_when_tool_missing() {
        assert!(should_skip(true, false));
    }

    #[test]
    fn optional_check_runs_when_tool_present() {
        assert!(!should_skip(true, true));
    }

    // -- fix_args ---

    #[test]
    fn fix_args_no_fix_returns_default() {
        let defaults = &["fmt", "--check"];
        assert_eq!(fix_args(CheckId::Fmt, defaults, false), defaults.to_vec());
    }

    #[test]
    fn fix_args_fmt_drops_check_flag() {
        let defaults = &["fmt", "--check"];
        let result = fix_args(CheckId::Fmt, defaults, true);
        assert_eq!(result, vec!["fmt"]);
    }

    #[test]
    fn fix_args_clippy_inserts_fix_before_separator() {
        let defaults = &["clippy", "--all-targets", "--", "-D", "warnings"];
        let result = fix_args(CheckId::Clippy, defaults, true);
        assert_eq!(
            result,
            vec![
                "clippy",
                "--all-targets",
                "--fix",
                "--allow-dirty",
                "--",
                "-D",
                "warnings"
            ]
        );
    }

    #[test]
    fn fix_args_other_check_unchanged() {
        let defaults = &["test", "--all-targets"];
        assert_eq!(fix_args(CheckId::Test, defaults, true), defaults.to_vec());
    }

    // -- check_display_name ---

    #[test]
    fn display_name_formats_correctly() {
        assert_eq!(
            check_display_name("cargo fmt --check"),
            "Running cargo fmt --check..."
        );
    }

    // -- is_tool_not_found ---

    #[test]
    fn detects_command_not_found() {
        assert!(is_tool_not_found("error: no such subcommand: `rail`"));
        assert!(is_tool_not_found("bash: cargo-rail: command not found"));
        assert!(is_tool_not_found("No such file or directory"));
    }

    #[test]
    fn normal_output_not_flagged() {
        assert!(!is_tool_not_found("Compiling calendsync v0.1.0"));
        assert!(!is_tool_not_found("error[E0308]: mismatched types"));
        assert!(!is_tool_not_found(""));
    }

    // -- determine_outcome ---

    #[test]
    fn exit_ok_yields_passed() {
        let outcome = determine_outcome(true, "ok".into(), false, false);
        assert_eq!(
            outcome,
            CheckOutcome::Passed {
                output: "ok".into()
            }
        );
    }

    #[test]
    fn exit_fail_required_yields_failed() {
        let outcome = determine_outcome(false, "err".into(), false, false);
        assert_eq!(
            outcome,
            CheckOutcome::Failed {
                output: "err".into()
            }
        );
    }

    #[test]
    fn exit_fail_optional_tool_missing_yields_skipped() {
        let outcome = determine_outcome(false, "not found".into(), true, true);
        assert_eq!(outcome, CheckOutcome::Skipped);
    }

    #[test]
    fn exit_fail_optional_real_failure_yields_failed() {
        let outcome = determine_outcome(false, "real error".into(), true, false);
        assert_eq!(
            outcome,
            CheckOutcome::Failed {
                output: "real error".into()
            }
        );
    }

    // -- format_log_entry ---

    #[test]
    fn log_entry_pass() {
        let result = CheckResult {
            name: "cargo fmt --check",
            outcome: CheckOutcome::Passed {
                output: String::new(),
            },
        };
        let entry = format_log_entry(&result);
        assert!(entry.contains("[PASS]"));
        assert!(entry.contains("cargo fmt --check"));
        assert!(entry.contains("(no output)"));
    }

    #[test]
    fn log_entry_fail_with_output() {
        let result = CheckResult {
            name: "cargo clippy ... -D warnings",
            outcome: CheckOutcome::Failed {
                output: "warning: unused variable".into(),
            },
        };
        let entry = format_log_entry(&result);
        assert!(entry.contains("[FAIL]"));
        assert!(entry.contains("warning: unused variable"));
    }

    #[test]
    fn log_entry_skip() {
        let result = CheckResult {
            name: "cargo rail unify --check",
            outcome: CheckOutcome::Skipped,
        };
        let entry = format_log_entry(&result);
        assert!(entry.contains("[SKIP]"));
    }

    // -- format_log_path_line ---

    #[test]
    fn log_path_line_format() {
        let path = std::path::Path::new("/tmp/xtask-lint.log");
        assert_eq!(format_log_path_line(path), "log: /tmp/xtask-lint.log");
    }

    // -- CHECKS const ---

    #[test]
    fn checks_has_ten_entries() {
        assert_eq!(CHECKS.len(), 10);
    }

    #[test]
    fn checks_order_is_correct() {
        let ids: Vec<CheckId> = CHECKS.iter().map(|c| c.id).collect();
        assert_eq!(
            ids,
            vec![
                CheckId::Fmt,
                CheckId::Check,
                CheckId::Clippy,
                CheckId::Test,
                CheckId::Rail,
                CheckId::BiomeFrontend,
                CheckId::TypecheckFrontend,
                CheckId::TestFrontend,
                CheckId::BiomeExample,
                CheckId::TypecheckExample,
            ]
        );
    }

    #[test]
    fn only_rail_is_optional() {
        for check in CHECKS {
            if check.id == CheckId::Rail {
                assert!(check.optional, "Rail should be optional");
            } else {
                assert!(!check.optional, "{} should not be optional", check.name);
            }
        }
    }

    #[test]
    fn typescript_checks_have_cwd() {
        let ts_ids = [
            CheckId::BiomeFrontend,
            CheckId::TypecheckFrontend,
            CheckId::TestFrontend,
            CheckId::BiomeExample,
            CheckId::TypecheckExample,
        ];
        for check in CHECKS {
            if ts_ids.contains(&check.id) {
                assert!(check.cwd.is_some(), "{} should have a cwd", check.name);
            }
        }
    }

    #[test]
    fn rust_checks_have_no_cwd() {
        let rust_ids = [
            CheckId::Fmt,
            CheckId::Check,
            CheckId::Clippy,
            CheckId::Test,
            CheckId::Rail,
        ];
        for check in CHECKS {
            if rust_ids.contains(&check.id) {
                assert!(check.cwd.is_none(), "{} should have no cwd", check.name);
            }
        }
    }

    // -- should_skip_missing_cwd ---

    #[test]
    fn skip_missing_cwd_none_returns_false() {
        assert!(!should_skip_missing_cwd(None));
    }

    #[test]
    fn skip_missing_cwd_existing_dir_returns_false() {
        // Use a directory guaranteed to exist
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(!should_skip_missing_cwd(Some(dir)));
    }

    #[test]
    fn skip_missing_cwd_nonexistent_dir_returns_true() {
        let dir = std::path::Path::new("/nonexistent/path/that/should/not/exist");
        assert!(should_skip_missing_cwd(Some(dir)));
    }

    // -- resolve_cwd ---

    #[test]
    fn resolve_cwd_none_returns_none() {
        assert!(resolve_cwd(None).is_none());
    }

    #[test]
    fn resolve_cwd_some_returns_absolute_path() {
        let resolved = resolve_cwd(Some("crates/frontend"));
        assert!(resolved.is_some());
        let path = resolved.unwrap();
        assert!(path.is_absolute());
        assert!(path.ends_with("crates/frontend"));
    }

    // -- fix_args for TypeScript checks ---

    #[test]
    fn fix_args_typescript_check_unchanged() {
        let defaults = &["biome", "check", "--write", "--unsafe"];
        assert_eq!(
            fix_args(CheckId::BiomeFrontend, defaults, true),
            defaults.to_vec()
        );
    }

    // -- should_skip_by_flag ---

    #[test]
    fn skip_by_flag_no_flags_skips_nothing() {
        for check in CHECKS {
            assert!(
                !should_skip_by_flag(check.id, false, false, false),
                "{} should not be skipped without flags",
                check.name
            );
        }
    }

    #[test]
    fn skip_by_flag_no_biome_skips_biome_checks() {
        assert!(should_skip_by_flag(
            CheckId::BiomeFrontend,
            true,
            false,
            false
        ));
        assert!(should_skip_by_flag(
            CheckId::BiomeExample,
            true,
            false,
            false
        ));
        assert!(!should_skip_by_flag(
            CheckId::TypecheckFrontend,
            true,
            false,
            false
        ));
        assert!(!should_skip_by_flag(CheckId::Fmt, true, false, false));
    }

    #[test]
    fn skip_by_flag_no_typecheck_skips_typecheck_checks() {
        assert!(should_skip_by_flag(
            CheckId::TypecheckFrontend,
            false,
            true,
            false
        ));
        assert!(should_skip_by_flag(
            CheckId::TypecheckExample,
            false,
            true,
            false
        ));
        assert!(!should_skip_by_flag(
            CheckId::BiomeFrontend,
            false,
            true,
            false
        ));
    }

    #[test]
    fn skip_by_flag_no_bun_test_skips_test_frontend() {
        assert!(should_skip_by_flag(
            CheckId::TestFrontend,
            false,
            false,
            true
        ));
        assert!(!should_skip_by_flag(CheckId::Test, false, false, true));
    }
}
