# AGENTS.md - Airyc Compiler

Guidelines for AI coding agents working on the airyc compiler codebase.

## Project Overview

Airyc is a toy programming language. The compiler compiles to LLVM IR and native executables.

### Crate Structure

- `compiler` (root) - Main compiler binary with CLI
- `crates/syntax` - SyntaxKind definitions and AST node types
- `crates/lexer` - Lexer using logos
- `crates/parser` - Parser using rowan for lossless syntax trees
- `crates/analyzer` - Semantic analysis, type checking, symbol resolution
- `crates/codegen` - LLVM IR code generation using inkwell
- `crates/runtime` - Runtime library (C code compiled to static lib)
- `crates/language_server` - LSP server for IDE support
- `crates/utils` - Shared utility functions
- `crates/test` - Integration test runner

## Language Syntax

The language uses Rust-like syntax. See `README.md` for the full grammar.

### Key Syntax Differences from C/SysY

| Feature | C/SysY | Airyc |
|---------|--------|-------|
| Types | `int`, `float`, `void` | `i32`, `f32`, `void` |
| Variables | `int x = 1;` | `let x: i32 = 1;` |
| Constants | `const int x = 1;` | `let x: const i32 = 1;` |
| Arrays | `int arr[2][3];` | `let arr: [[i32; 3]; 2];` (reversed!) |
| Pointers | `int *p;` | `let p: *mut i32;` or `let p: *const i32;` |
| Functions | `int add(int a) {}` | `fn add(a: i32) -> i32 {}` |
| Structs | `struct P { int x; };` | `struct P { x: i32 }` (comma-separated) |

## Build Commands

```bash
cargo build --workspace                    # Build all crates (debug)
cargo build --release --workspace          # Build release
cargo make build                           # Build compiler + runtime (cargo-make)
```

## Lint Commands

```bash
cargo fmt --all -- --check                 # Format check (CI enforced)
cargo fmt --all                            # Format code
cargo clippy --workspace --all-targets --all-features -- -D warnings  # Clippy (CI enforced)
```

## Test Commands

```bash
# Unit tests
cargo test --workspace                     # Run all unit tests
cargo test -p parser                       # Run tests for specific crate
cargo test -p parser test_declarations     # Run single test by name
cargo test --workspace -- --nocapture      # Run with output

# Snapshot testing (insta)
cargo insta test --review                  # Update snapshots
cargo insta accept                         # Accept all snapshots

# Integration tests (requires cargo-make)
cargo make test                            # All integration tests
```

### Testing Individual Files

**IMPORTANT**: Always use `/tmp` as working directory to avoid polluting source tree with artifacts.

```bash
cd /tmp
/path/to/airyc-compiler -i /path/to/testcases/file.airy
```

## Code Style Guidelines

### Rust Edition
- Edition 2024

### Imports
Group imports: std first, then external crates, then local crates.

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

## CI Requirements

The CI pipeline runs (all must pass):
1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
3. `cargo build --verbose --workspace`
4. `cargo test --verbose --workspace`
5. `cargo make test` (integration tests)

## Key Dependencies

- `logos` - Lexer generator
- `rowan` - Lossless syntax trees
- `inkwell` - LLVM bindings (requires LLVM 21.1)
- `thunderdome` - Arena allocator for AST nodes
- `thiserror` - Error derive macros
- `insta` - Snapshot testing
- `clap` - CLI argument parsing

## File Patterns

- Source: `src/**/*.rs`, `crates/*/src/**/*.rs`
- Tests: `**/test.rs`, `**/tests/*.rs`
- Snapshots: `**/snapshots/*.snap`
- Test cases: `testcases/**/*.airy`
