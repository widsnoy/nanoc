# Airyc Compiler - Issues & Improvements

## 1. Duplicate Code Patterns (RESOLVED)

All duplicate code patterns have been refactored:

- [x] codegen/llvm_ir.rs: Merged `compile_expr` and `compile_expr_func_call` into `compile_expr_inner`
- [x] codegen/llvm_ir.rs: Extracted `build_int_cmp` helper for int compare operations
- [x] codegen/llvm_ir.rs: Extracted `build_float_cmp` helper for float compare operations
- [x] codegen/llvm_ir.rs: Extracted `branch_if_no_terminator` helper for if/while stmt terminator checks
- [x] analyzer/analyze.rs: Unified `leave_const_init_val` and `leave_init_val` via `check_and_mark_constant`
- [x] parser/parsing.rs: Unified `parse_const_init_val` and `parse_init_val` via `parse_init_val_generic`
- [x] analyzer/module.rs: Created `define_id_type!` macro for ID types boilerplate

## 2. Design Issues (RESOLVED)

### codegen/llvm_ir.rs
- [x] `Program` struct split into `Program` + `SymbolTable` for clearer responsibilities

### analyzer/module.rs
- [x] `analyzing: AnalyzeContext` decoupled via `finish_analysis()` method

### parser/ast.rs
- [x] `BinaryOp::op()` and `UnaryOp::op()` unified via `OpNode` trait

## 3. Not Implemented Features

| Location | Feature |
|----------|---------|
| codegen | `DerefExpr` compilation |
| codegen | Struct type support |
| analyzer | Struct `is_const` check |
| analyzer | Pointer/Struct `const_zero` |

## 4. Completed Improvements

- [x] Error handling in codegen module (replaced `unwrap`/`panic` with `Result`)
- [x] Unified error type `CodegenError` with thiserror
- [x] `anyhow` for main.rs error handling
- [x] All comments/errors/docs converted to English
