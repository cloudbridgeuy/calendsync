# VNTANA References Cleanup Design

## Overview

This document describes the removal of all VNTANA, vntana, and vnt references from the calendsync codebase. The cleanup also removes the Environment enum and related GKE cluster configurations, which served VNTANA's deployment infrastructure and have no relevance to calendsync.

## Scope of Removal

The cleanup targets four categories of code.

**VNTANA References** include string literals like "VNTANA-3D/vntana-devops-cli", "vntana-devops-cli", and "VNTANA DevOps CLI" scattered across the xtask crate. These appear in documentation comments, GitHub repository constants, and the pre-commit hook template.

**The `install` Module** exists solely to build and install a `vnt` binary that doesn't exist in this project. The entire module, including its error types, is deleted since calendsync has no equivalent binary installation workflow.

**The `release` Module** is tightly coupled to the VNTANA project. It references `crates/vnt/Cargo.toml`, uses the VNTANA GitHub repository for workflow monitoring, and includes `vnt upgrade` functionality. The entire module is deleted.

**The `Environment` System** comprises the `Environment` enum with its eight VNTANA-specific variants (Development, Acceptance, Staging, Production, Accenture, Sony, SonyStaging, PrdPtc), the `environment` field in the `Global` struct, the `VNTANA_ENVIRONMENT` env var, and two helper functions for GKE cluster names. All of this infrastructure serves VNTANA's deployment needs and has no relevance to calendsync.

## Files Deleted

The `install` module consists of two files:
- `xtask/src/install/mod.rs`
- `xtask/src/install/error.rs`

The `release` module consists of seven files:
- `xtask/src/release/mod.rs`
- `xtask/src/release/error.rs`
- `xtask/src/release/git.rs`
- `xtask/src/release/github.rs`
- `xtask/src/release/validation.rs`
- `xtask/src/release/version.rs`
- `xtask/src/release/workflow.rs`

## Files Modified

### xtask/src/main.rs

Module declarations are reduced from seven to five, removing `install` and `release`. The documentation and command attributes reference calendsync instead of vntana-devops-cli.

The `Global` struct loses its `environment` field and the `get_environment` method. Only `silent` and `verbose` fields remain.

The `Commands` enum loses the `Install` and `Release` variants and their corresponding match arms in `main()`.

The `Environment` enum, its `Display` impl, `get_cluster_name_for_environment`, and `get_context_name_for_environment` are all deleted.

### xtask/src/lint/hooks.rs

The pre-commit hook template comment changes from "VNTANA DevOps CLI Pre-commit Hook" to "calendsync Pre-commit Hook".

### CLAUDE.md

The following are removed:
- `cargo xtask release create <version>` from the Build Commands section
- `cargo xtask install` and `cargo xtask release create <version>` from xtask Commands section
- The entire Release Process section

## What Remains

The xtask crate retains five functional modules:
- `dev` - Development server with hot-reload
- `dynamodb` - DynamoDB table management
- `integration` - Integration test runner
- `lint` - Code quality checks and git hooks
- `prelude` - Shared utilities

## Verification

After implementation:
1. `cargo check -p xtask` confirms compilation
2. `cargo xtask lint` verifies remaining commands work
3. Grep confirms no VNTANA/vntana/vnt references remain
