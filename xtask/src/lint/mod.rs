use crate::prelude::*;
use error::Result;

pub mod error;
mod hooks;

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
5. cargo machete - Unused dependencies detection

TypeScript checks (crates/frontend):
6. biome check --write --unsafe - Format and lint TypeScript (auto-fix)
7. bun run typecheck - TypeScript type checking
8. bun test - Run TypeScript tests

TypeScript checks (examples/react-ssr):
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
}

pub async fn run(command: LintCommand, global: crate::Global) -> Result<()> {
    // Handle hooks management commands
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

    // Run lint checks
    run_lint_checks(&command, &global).await
}

async fn run_lint_checks(command: &LintCommand, global: &crate::Global) -> Result<()> {
    use error::require_command;

    // Check required dependencies
    require_command("cargo", "Required for Rust development: https://rustup.rs/")?;
    require_command(
        "cargo-machete",
        "Required for unused dependency checks: cargo install cargo-machete",
    )?;
    require_command("bun", "Required for TypeScript: https://bun.sh/")?;

    if !global.is_silent() {
        aprintln!("{}", p_b("Running code quality checks..."));
        aprintln!();
    }

    let mut all_passed = true;

    // Rust checks
    // 1. Run cargo fmt
    if !run_cargo_fmt(command, global).await? {
        all_passed = false;
    }

    // 2. Run cargo check
    if !run_cargo_check(global).await? {
        all_passed = false;
    }

    // 3. Run cargo clippy
    if !run_cargo_clippy(global).await? {
        all_passed = false;
    }

    // 4. Run cargo test
    if !run_cargo_test(global).await? {
        all_passed = false;
    }

    // 5. Run cargo machete
    if !run_cargo_machete(global).await? {
        all_passed = false;
    }

    // TypeScript checks (crates/frontend)
    // 6. Run biome check (format + lint with auto-fix)
    if !run_biome_check(global).await? {
        all_passed = false;
    }

    // 7. Run bun typecheck
    if !run_bun_typecheck(global).await? {
        all_passed = false;
    }

    // 8. Run bun test
    if !run_bun_test(global).await? {
        all_passed = false;
    }

    // TypeScript checks (examples/react-ssr)
    // 9. Run biome check on example
    if !run_example_biome_check(global).await? {
        all_passed = false;
    }

    // 10. Run bun typecheck on example
    if !run_example_typecheck(global).await? {
        all_passed = false;
    }

    aprintln!();
    if all_passed {
        aprintln!("{} {}", p_g("✅"), p_g("All checks passed!"));
        Ok(())
    } else {
        aprintln!("{} {}", p_r("❌"), p_r("Some checks failed"));
        aprintln!();
        if !global.is_silent() {
            aprintln!("{}", p_b("Quick fixes:"));
            aprintln!("  • {} - Format code", p_c("cargo xtask lint --fix"));
            aprintln!("  • {} - Auto-fix clippy issues", p_c("cargo clippy --fix"));
            aprintln!("  • {} - Check compilation", p_c("cargo check"));
        }
        Err(error::LintError::ChecksFailed)?
    }
}

async fn run_cargo_fmt(command: &LintCommand, global: &crate::Global) -> Result<bool> {
    if !global.is_silent() {
        aprintln!("{} {}", p_b("🔧"), p_b("Running cargo fmt..."));
    }

    // First check if formatting is needed
    let check_output = tokio::process::Command::new("cargo")
        .args(["fmt", "--check"])
        .output()
        .await?;

    if check_output.status.success() {
        if !global.is_silent() {
            aprintln!("{} {}", p_g("✅"), "Code formatting is correct");
        }
        return Ok(true);
    }

    // If --fix is enabled or we're in staged_only mode, auto-fix
    if command.fix || command.staged_only {
        if global.is_verbose() {
            aprintln!(
                "{} {}",
                p_y("⚠️"),
                "Code formatting issues found. Auto-fixing..."
            );
        }

        let fmt_status = tokio::process::Command::new("cargo")
            .arg("fmt")
            .status()
            .await?;

        if fmt_status.success() {
            if command.staged_only {
                // Re-stage formatted files in git hook mode
                restage_rust_files(global).await?;
                if !global.is_silent() {
                    aprintln!("{} {}", p_g("✅"), "Code formatted and re-staged");
                }
            } else if !global.is_silent() {
                aprintln!("{} {}", p_g("✅"), "Code formatted");
            }
            Ok(true)
        } else {
            aprintln!("{} {}", p_r("❌"), "cargo fmt failed");
            Ok(false)
        }
    } else {
        aprintln!(
            "{} {}",
            p_r("❌"),
            "Code formatting check failed. Run with --fix to auto-format"
        );
        Ok(false)
    }
}

async fn run_cargo_check(global: &crate::Global) -> Result<bool> {
    if !global.is_silent() {
        aprintln!("{} {}", p_b("🔧"), p_b("Running cargo check..."));
    }

    let mut cmd = tokio::process::Command::new("cargo");
    cmd.args(["check", "--all-targets"]);

    if !global.is_verbose() {
        cmd.arg("--quiet");
    }

    let status = cmd.status().await?;

    if status.success() {
        if !global.is_silent() {
            aprintln!("{} {}", p_g("✅"), "Cargo check passed");
        }
        Ok(true)
    } else {
        aprintln!("{} {}", p_r("❌"), "Cargo check failed");
        aprintln!("{}", p_r("Please fix compilation errors before proceeding"));
        Ok(false)
    }
}

async fn run_cargo_clippy(global: &crate::Global) -> Result<bool> {
    if !global.is_silent() {
        aprintln!("{} {}", p_b("🔧"), p_b("Running cargo clippy..."));
    }

    let mut cmd = tokio::process::Command::new("cargo");
    cmd.args(["clippy", "--all-targets"]);

    if !global.is_verbose() {
        cmd.arg("--quiet");
    }

    cmd.args(["--", "-D", "warnings"]);

    let status = cmd.status().await?;

    if status.success() {
        if !global.is_silent() {
            aprintln!("{} {}", p_g("✅"), "Clippy checks passed");
        }
        Ok(true)
    } else {
        aprintln!("{} {}", p_r("❌"), "Clippy checks failed");
        aprintln!("{}", p_r("Please fix clippy warnings before proceeding"));
        if !global.is_silent() {
            aprintln!(
                "{} Run {} to auto-fix some issues",
                p_b("Tip:"),
                p_c("cargo clippy --fix")
            );
        }
        Ok(false)
    }
}

async fn run_cargo_test(global: &crate::Global) -> Result<bool> {
    if !global.is_silent() {
        aprintln!("{} {}", p_b("🔧"), p_b("Running cargo test..."));
    }

    let mut cmd = tokio::process::Command::new("cargo");
    cmd.args(["test", "--all-targets"]);

    if !global.is_verbose() {
        cmd.arg("--quiet");
    }

    let status = cmd.status().await?;

    if status.success() {
        if !global.is_silent() {
            aprintln!("{} {}", p_g("✅"), "All tests passed");
        }
        Ok(true)
    } else {
        aprintln!("{} {}", p_r("❌"), "Tests failed");
        aprintln!("{}", p_r("Please fix failing tests before proceeding"));
        Ok(false)
    }
}

async fn run_cargo_machete(global: &crate::Global) -> Result<bool> {
    if !global.is_silent() {
        aprintln!("{} {}", p_b("🔧"), p_b("Running cargo machete..."));
    }

    let output = tokio::process::Command::new("cargo")
        .arg("machete")
        .output()
        .await?;

    // cargo machete outputs to both stdout and stderr
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // cargo machete can fail with exit code 2 for various reasons including
    // trying to analyze non-existent directories mentioned in error messages
    // We should ignore "No such file or directory" errors from stderr
    let has_io_error_only =
        stderr.contains("No such file or directory") && !stderr.contains("unused");

    // Check if there are any unused dependencies
    // cargo machete reports "didn't find any unused dependencies" on success
    let found_success_message =
        stdout.contains("didn't find any unused") || stderr.contains("didn't find any unused");

    if (output.status.success() || has_io_error_only) && found_success_message {
        if !global.is_silent() {
            aprintln!("{} {}", p_g("✅"), "No unused dependencies found");
        }
        Ok(true)
    } else if has_io_error_only {
        // IO errors but no unused deps mentioned - treat as success
        if !global.is_silent() {
            aprintln!("{} {}", p_g("✅"), "No unused dependencies found");
        }
        Ok(true)
    } else {
        aprintln!("{} {}", p_r("❌"), "Unused dependencies detected");
        aprintln!(
            "{}",
            p_r("Please remove unused dependencies from Cargo.toml")
        );
        // Show the actual output if verbose
        if global.is_verbose() {
            if !stdout.is_empty() {
                aprintln!("Output: {}", stdout);
            }
            if !stderr.is_empty() {
                aprintln!("Errors: {}", stderr);
            }
        }
        Ok(false)
    }
}

async fn restage_rust_files(global: &crate::Global) -> Result<()> {
    // Get list of staged Rust files
    let output = tokio::process::Command::new("git")
        .args(["diff", "--cached", "--name-only", "--diff-filter=ACM"])
        .output()
        .await?;

    if !output.status.success() {
        return Ok(());
    }

    let files = String::from_utf8_lossy(&output.stdout);
    let rust_files: Vec<&str> = files.lines().filter(|line| line.ends_with(".rs")).collect();

    if !rust_files.is_empty() {
        let mut cmd = tokio::process::Command::new("git");
        cmd.arg("add");
        cmd.args(&rust_files);
        cmd.status().await?;

        if global.is_verbose() {
            aprintln!("{} Re-staged {} Rust files", p_b("Info:"), rust_files.len());
        }
    }

    Ok(())
}

/// Get the path to the frontend crate directory.
fn get_frontend_dir() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    std::path::Path::new(manifest_dir)
        .parent()
        .unwrap()
        .join("crates/frontend")
}

/// Get the path to the react-ssr example directory.
fn get_example_hello_world_dir() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    std::path::Path::new(manifest_dir)
        .parent()
        .unwrap()
        .join("crates/calendsync/examples/react-ssr")
}

async fn run_biome_check(global: &crate::Global) -> Result<bool> {
    if !global.is_silent() {
        aprintln!("{} {}", p_b("🔧"), p_b("Running biome check..."));
    }

    let frontend_dir = get_frontend_dir();

    // Check if frontend directory exists
    if !frontend_dir.exists() {
        if !global.is_silent() {
            aprintln!(
                "{} {}",
                p_y("⚠️"),
                "Frontend directory not found, skipping biome checks"
            );
        }
        return Ok(true);
    }

    // Run biome check with --write --unsafe to auto-fix all issues
    let output = tokio::process::Command::new("bunx")
        .args(["biome", "check", "--write", "--unsafe"])
        .current_dir(&frontend_dir)
        .output()
        .await?;

    if output.status.success() {
        if !global.is_silent() {
            aprintln!("{} {}", p_g("✅"), "Biome check passed");
        }
        Ok(true)
    } else {
        aprintln!("{} {}", p_r("❌"), "Biome check failed");
        // Show output on failure
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stdout.is_empty() {
            aprintln!("{}", stdout);
        }
        if !stderr.is_empty() {
            aprintln!("{}", stderr);
        }
        Ok(false)
    }
}

async fn run_bun_typecheck(global: &crate::Global) -> Result<bool> {
    if !global.is_silent() {
        aprintln!("{} {}", p_b("🔧"), p_b("Running bun typecheck..."));
    }

    let frontend_dir = get_frontend_dir();

    // Check if frontend directory exists
    if !frontend_dir.exists() {
        if !global.is_silent() {
            aprintln!(
                "{} {}",
                p_y("⚠️"),
                "Frontend directory not found, skipping TypeScript checks"
            );
        }
        return Ok(true);
    }

    let status = tokio::process::Command::new("bun")
        .args(["run", "typecheck"])
        .current_dir(&frontend_dir)
        .status()
        .await?;

    if status.success() {
        if !global.is_silent() {
            aprintln!("{} {}", p_g("✅"), "TypeScript type check passed");
        }
        Ok(true)
    } else {
        aprintln!("{} {}", p_r("❌"), "TypeScript type check failed");
        aprintln!(
            "{}",
            p_r("Please fix TypeScript type errors before proceeding")
        );
        Ok(false)
    }
}

async fn run_bun_test(global: &crate::Global) -> Result<bool> {
    if !global.is_silent() {
        aprintln!("{} {}", p_b("🔧"), p_b("Running bun test..."));
    }

    let frontend_dir = get_frontend_dir();

    // Check if frontend directory exists
    if !frontend_dir.exists() {
        if !global.is_silent() {
            aprintln!(
                "{} {}",
                p_y("⚠️"),
                "Frontend directory not found, skipping TypeScript tests"
            );
        }
        return Ok(true);
    }

    let output = tokio::process::Command::new("bun")
        .arg("test")
        .current_dir(&frontend_dir)
        .output()
        .await?;

    if output.status.success() {
        if !global.is_silent() {
            // Extract test summary from output
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = stdout.lines().find(|l| l.contains("pass")) {
                aprintln!("{} TypeScript tests: {}", p_g("✅"), line.trim());
            } else {
                aprintln!("{} {}", p_g("✅"), "TypeScript tests passed");
            }
        }
        Ok(true)
    } else {
        aprintln!("{} {}", p_r("❌"), "TypeScript tests failed");
        // Show test output on failure
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stdout.is_empty() {
            aprintln!("{}", stdout);
        }
        if !stderr.is_empty() {
            aprintln!("{}", stderr);
        }
        Ok(false)
    }
}

async fn run_example_biome_check(global: &crate::Global) -> Result<bool> {
    if !global.is_silent() {
        aprintln!(
            "{} {}",
            p_b("🔧"),
            p_b("Running biome check (examples/react-ssr)...")
        );
    }

    let example_dir = get_example_hello_world_dir();

    // Check if example directory exists
    if !example_dir.exists() {
        if !global.is_silent() {
            aprintln!(
                "{} {}",
                p_y("⚠️"),
                "Example react-ssr directory not found, skipping biome checks"
            );
        }
        return Ok(true);
    }

    // Run biome check with --write --unsafe to auto-fix all issues
    let output = tokio::process::Command::new("bunx")
        .args(["biome", "check", "--write", "--unsafe"])
        .current_dir(&example_dir)
        .output()
        .await?;

    if output.status.success() {
        if !global.is_silent() {
            aprintln!("{} {}", p_g("✅"), "Example biome check passed");
        }
        Ok(true)
    } else {
        aprintln!("{} {}", p_r("❌"), "Example biome check failed");
        // Show output on failure
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stdout.is_empty() {
            aprintln!("{}", stdout);
        }
        if !stderr.is_empty() {
            aprintln!("{}", stderr);
        }
        Ok(false)
    }
}

async fn run_example_typecheck(global: &crate::Global) -> Result<bool> {
    if !global.is_silent() {
        aprintln!(
            "{} {}",
            p_b("🔧"),
            p_b("Running bun typecheck (examples/react-ssr)...")
        );
    }

    let example_dir = get_example_hello_world_dir();

    // Check if example directory exists
    if !example_dir.exists() {
        if !global.is_silent() {
            aprintln!(
                "{} {}",
                p_y("⚠️"),
                "Example react-ssr directory not found, skipping TypeScript checks"
            );
        }
        return Ok(true);
    }

    // Check if node_modules exists, if not run bun install first
    if !example_dir.join("node_modules").exists() {
        if !global.is_silent() {
            aprintln!("{} {}", p_b("📦"), "Installing example dependencies...");
        }
        let install_status = tokio::process::Command::new("bun")
            .arg("install")
            .current_dir(&example_dir)
            .status()
            .await?;

        if !install_status.success() {
            aprintln!("{} {}", p_r("❌"), "Failed to install example dependencies");
            return Ok(false);
        }
    }

    let status = tokio::process::Command::new("bun")
        .args(["run", "typecheck"])
        .current_dir(&example_dir)
        .status()
        .await?;

    if status.success() {
        if !global.is_silent() {
            aprintln!("{} {}", p_g("✅"), "Example TypeScript type check passed");
        }
        Ok(true)
    } else {
        aprintln!("{} {}", p_r("❌"), "Example TypeScript type check failed");
        aprintln!(
            "{}",
            p_r("Please fix TypeScript type errors in examples/react-ssr")
        );
        Ok(false)
    }
}
