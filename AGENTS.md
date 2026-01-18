# AGENTS.md - Airyc Compiler

This document provides guidelines for AI coding agents working on the airyc compiler codebase.

## Project Overview

Airyc is a compiler for a SysY-based language with added structure and pointer support. It compiles to LLVM IR and native executables.

### Crate Structure

- `airyc-compiler` (root) - Main compiler binary with CLI
- `crates/airyc-parser` - Lexer, parser, AST definitions using rowan
- `crates/airyc-analyzer` - Semantic analysis, type checking, symbol resolution
- `crates/airyc-codegen` - LLVM IR code generation using inkwell
- `crates/airyc-runtime` - Runtime library (C code compiled to static lib)

## Build Commands

```bash
# Build all crates (debug)
cargo build --workspace

# Build release
cargo build --release --workspace

# Build only the compiler binary
cargo build --bin airyc-compiler

# Build with cargo-make (release compiler + runtime)
cargo make build
```

## Lint Commands

```bash
# Format check (CI enforced)
cargo fmt --all -- --check

# Format code
cargo fmt --all

# Clippy (CI enforced, warnings are errors)
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Test Commands

```bash
# Run all unit tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p airyc-parser
cargo test -p airyc-analyzer
cargo test -p airyc-codegen

# Run a single test by name
cargo test -p airyc-parser test_declarations
cargo test -p airyc-parser test_if_statement

# Run tests with output
cargo test --workspace -- --nocapture

# Update insta snapshots
cargo insta test --review
cargo insta accept

# Run integration tests (requires built compiler)
cd test && bash ./test.sh lv1 lv3 lv4 lv5 lv6 lv7 lv8 lv9
```

## Code Style Guidelines

### Rust Edition
- Edition 2024

### Imports
- Group imports: std first, then external crates, then local crates
- Use `crate::` for internal imports
- Prefer explicit imports over glob imports
- Example:
```rust
use std::collections::HashMap;

use inkwell::context::Context;
use rowan::SyntaxNode;

use crate::syntax_kind::SyntaxKind;
use airyc_parser::ast::*;
```

### Formatting
- Use `cargo fmt` (rustfmt defaults)
- 4-space indentation
- Max line width: 100 characters (default)

### Naming Conventions
- Types: `PascalCase` (e.g., `CompUnit`, `NType`, `SyntaxKind`)
- Functions/methods: `snake_case` (e.g., `compile_comp_unit`, `get_variable`)
- Constants: `SCREAMING_SNAKE_CASE` (e.g., `CONST_KW`, `INT_LITERAL`)
- Modules: `snake_case`
- Type aliases for IDs: `PascalCase` with `ID` suffix (e.g., `VariableID`, `ScopeID`)

### Type Definitions
- Use `thiserror` for error types
- Use `#[derive(Debug, Clone, PartialEq)]` for data types
- Prefer enums for AST node kinds and type representations
- Use `Box<T>` for recursive types

### Error Handling
- Use `Result<T, E>` for fallible operations
- Use `thiserror` for custom error types
- Use `.expect("message")` only when failure is a bug
- Propagate errors with `?` operator

### Comments
- Chinese comments are allowed (per copilot-instructions.md)
- Use `///` for doc comments on public items
- Use `//` for inline explanations

### Testing
- Use `insta` for snapshot testing (parser, codegen)
- Place tests in `src/test.rs` module with `#[cfg(test)]`
- Test function naming: `test_<feature_name>`

### AST and Syntax
- Uses rowan for lossless syntax trees
- `SyntaxKind` enum defines all token/node types
- AST nodes wrap `SyntaxNode` with typed accessors
- Visitor pattern for tree traversal

### LLVM Codegen
- Uses inkwell (safe LLVM bindings)
- Requires LLVM 21.1 (see inkwell features)
- `Program` struct holds compilation context
- Scoped symbol tables for variables

## CI Requirements

The CI pipeline runs:
1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. `cargo build --verbose --workspace`
4. `cargo test --verbose --workspace`
5. Integration tests via `test/test.sh`

All checks must pass before merging.

## Copilot Instructions

From `.github/copilot-instructions.md`:
- When performing code review, respond in Chinese
- Chinese comments are allowed in code

## Dependencies

Key dependencies:
- `logos` - Lexer generator
- `rowan` - Lossless syntax trees
- `inkwell` - LLVM bindings (requires LLVM 21.1)
- `thunderdome` - Arena allocator for AST nodes
- `text-size` - Text span utilities
- `thiserror` - Error derive macros
- `insta` - Snapshot testing
- `clap` - CLI argument parsing

## File Patterns

- Source files: `src/**/*.rs`, `crates/*/src/**/*.rs`
- Tests: `**/test.rs`, `**/tests/*.rs`
- Snapshots: `**/snapshots/*.snap`
- Test cases: `test/compiler-dev-test-cases/testcases/**/*.c`
