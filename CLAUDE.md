# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`fv` is a Rust binary project using Rust edition 2024.

## Common Commands

```bash
cargo build           # Build
cargo run             # Run
cargo test            # Run all tests
cargo test <name>     # Run a single test by name
cargo clippy          # Lint
cargo fmt             # Format code
cargo check           # Type-check without building
cargo build --release # Release build
```

## Architecture

Single-binary Rust project. Entry point is `src/main.rs`.
