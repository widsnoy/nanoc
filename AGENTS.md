# AGENTS.md - Airyc Compiler

Guidelines for AI coding agents working on the airyc compiler codebase.

## Project Overview

Airyc is a compiler for a SysY-based language with structure and pointer support. Compiles to LLVM IR and native executables.

### Crate Structure

- `compiler` (root) - Main compiler binary with CLI
- `crates/parser` - Lexer, parser, AST definitions using rowan
- `crates/analyzer` - Semantic analysis, type checking, symbol resolution
- `crates/codegen` - LLVM IR code generation using inkwell
- `crates/runtime` - Runtime library (C code compiled to static lib)
- `crates/test` - Integration test runner

## Build Commands

```bash
# Build all crates (debug)
cargo build --workspace

# Build release
cargo build --release --workspace

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
cargo test -p parser
cargo test -p analyzer
cargo test -p codegen

# Run a single test by name
cargo test -p parser test_declarations
cargo test -p parser test_if_statement
cargo test -p codegen test_function_call

# Run tests with output
cargo test --workspace -- --nocapture

# Update insta snapshots
cargo insta test --review
cargo insta accept

# Run integration tests (requires cargo-make)
cargo make test              # All tests with coverage
cargo make test-pku-minic    # Tests without perf
cargo make test-pointer      # Pointer functionality tests
cargo make test-struct       # Struct
cargo make test-ci           # CI test suite

# Run compiler manually
cargo build
./target/debug/airyc-compiler -i path/to/source
```

## Code Style Guidelines

### Rust Edition
- Edition 2024

### Imports
Group imports: std first, then external crates, then local crates. Use `crate::` for internal imports.

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
- Max line width: 100 characters

### Naming Conventions
- Types: `PascalCase` (e.g., `CompUnit`, `NType`, `SyntaxKind`)
- Functions/methods: `snake_case` (e.g., `compile_comp_unit`, `get_variable`)
- Constants: `SCREAMING_SNAKE_CASE` (e.g., `CONST_KW`, `INT_LITERAL`)
- Modules: `snake_case`
- Type aliases for IDs: `PascalCase` with `ID` suffix (e.g., `VariableID`, `ScopeID`)

### Type Definitions
- Use `thiserror` for error types with `#[derive(Debug, Error)]`
- Use `#[derive(Debug, Clone, PartialEq)]` for data types
- Prefer enums for AST node kinds and type representations
- Use `Box<T>` for recursive types
- Define `Result<T>` type alias in error modules

### Error Handling
- Use `Result<T, E>` for fallible operations
- Use `thiserror` for custom error types
- Use `.expect("message")` only when failure is a bug
- Propagate errors with `?` operator

### Comments and Documentation Language
- **All comments and documentation must be in Chinese**
- **Error messages in `Err()`, `panic!()`, `.expect()` must remain in English**
- Use `///` for doc comments on public items (in Chinese)
- Use `//` for inline explanations (in Chinese)

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
5. `cargo make test-ci` (integration tests)

All checks must pass before merging.

## Copilot Instructions

From `.github/copilot-instructions.md`:
- When performing code review, respond in Chinese
- Chinese comments are allowed in code

## Key Dependencies

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
