# AGENTS.md - Airyc Compiler

Guidelines for AI coding agents working on the airyc compiler codebase.

## Project Overview

Airyc is a compiler for a Rust-like language (evolved from SysY) with structure and pointer support. Compiles to LLVM IR and native executables.

### Crate Structure

- `compiler` (root) - Main compiler binary with CLI
- `crates/syntax` - SyntaxKind definitions and AST node types
- `crates/lexer` - Lexer using logos
- `crates/parser` - Parser using rowan for lossless syntax trees
- `crates/analyzer` - Semantic analysis, type checking, symbol resolution
- `crates/codegen` - LLVM IR code generation using inkwell
- `crates/runtime` - Runtime library (C code compiled to static lib)
- `crates/test` - Integration test runner

## Language Syntax

The language uses a Rust-like syntax. See `README.md` for the full grammar.

### C to Airyc Syntax Conversion

When converting test cases from C/SysY syntax to Airyc syntax:

#### Type Keywords
| C/SysY | Airyc |
|--------|-------|
| `int`  | `i32` |
| `float`| `f32` |
| `void` | `void` |

#### Variable Declarations
```c
// C/SysY
int x;
int x = 1;
const int x = 1;
float y = 1.0;
int a, b, c;

// Airyc
let x: i32;
let x: i32 = 1;
let x: const i32 = 1;
let y: f32 = 1.0;
let a;
let b;
let c;
```

#### Array Declarations
```c
// C/SysY
int arr[10];
int arr[2][3];
int arr[3] = {1, 2, 3};

// Airyc
let arr: [i32; 10];
let arr: [[i32; 3]; 2];  // Note: dimensions are reversed!
let arr: [i32; 3] = {1, 2, 3};
```

#### Pointer Declarations
```c
// C/SysY
int *p;
int * const p;
const int *p;

// Airyc
let p: *mut i32;           // mutable pointer to mutable i32
let p: const *mut i32;     // immutable pointer to mutable i32
let p: *const i32;         // mutable pointer to immutable i32
let p: const *const i32;   // immutable pointer to immutable i32
```

Note: `*` must be followed by `mut` or `const` to specify pointer mutability.

#### Function Definitions
```c
// C/SysY
int main() { return 0; }
void func() {}
int add(int a, int b) { return a + b; }
int *getPtr(int arr[][10]) {}

// Airyc
fn main() -> i32 { return 0; }
fn func() {}
fn add(a: i32, b: i32) -> i32 { return a + b; }
fn getPtr(arr: *mut [i32; 10]) -> *mut i32 {}
```

#### Struct Definitions
```c
// C/SysY
struct Point {
    int x;
    int y;
};
struct Point p;

// Airyc
struct Point { x: i32, y: i32 }
let p: struct Point;
```

#### Statements (unchanged)
- `if`, `else`, `while`, `break`, `continue`, `return` - same syntax
- Assignment: `x = 1;` - same syntax
- Expression statements: `func();` - same syntax

### Key Differences Summary
1. **Type comes after name**: `name: type` instead of `type name`
2. **`let` keyword** for all variable declarations
3. **`fn` keyword** for function definitions
4. **`->` for return type** instead of prefix type
5. **Array dimensions reversed**: `[inner; size]` is outermost
6. **`const` in type**: `let x: const i32` instead of `const int x`
7. **Struct fields use `,`** not `;` as separator
8. **Pointer requires `mut` or `const`**: `*mut T` or `*const T` instead of `*T`

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
```

### Testing Individual Files

**IMPORTANT**: When testing individual .airy or .c files with the compiler, always use `/tmp` as the working directory to avoid polluting the source tree with compilation artifacts (.ll, .s, .o, a.out files).

```bash
# Good: Test in /tmp
cd /tmp
/path/to/airyc-compiler -i /path/to/testcases/file.airy

# Bad: Test in source directory (creates artifacts in testcases/)
cd /path/to/airyc/testcases
../target/debug/airyc-compiler -i file.airy  # DON'T DO THIS

# Batch testing example (in /tmp)
cd /tmp
for f in /path/to/airyc/testcases/*/*.airy; do
  /path/to/airyc/target/debug/airyc-compiler -i "$f" >/dev/null 2>&1 || echo "Failed: $f"
done
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
- Test cases: `testcases/**/*.airy`
