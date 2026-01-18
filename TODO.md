# Airyc Compiler - Issues & Improvements

## 1. Duplicate Code Patterns

### codegen/llvm_ir.rs
| Lines | Issue |
|-------|-------|
| 524-547 | `compile_expr` and `compile_expr_func_call` are nearly identical, differ only in one bool param |
| 612-670 | Int binary ops have repetitive `build_int_compare(...).unwrap()` + `bool_to_i32(cmp)` pattern |
| 681-710 | Float compare ops have repetitive `build_float_compare(...).unwrap().into()` pattern |
| 429-456 | if stmt then/else branch terminator check code is duplicated |

### analyzer/analyze.rs
| Lines | Issue |
|-------|-------|
| 91-103 vs 161-173 | `leave_const_init_val` and `leave_init_val` logic nearly identical |
| 21-89 vs 110-159 | `leave_const_def` and `leave_var_def` have similar variable definition handling |

### parser/parsing.rs
| Lines | Issue |
|-------|-------|
| 149-164 vs 166-181 | `parse_const_init_val` and `parse_init_val` have identical structure |

### analyzer/module.rs
| Lines | Issue |
|-------|-------|
| 121-139, 162-180, 189-216 | Three ID types (VariableID, FunctionID, ScopeID) have repetitive boilerplate, could use macro |

## 2. Design Issues

### codegen/llvm_ir.rs
- `Program` struct has too many fields, unclear responsibilities, should be split

### analyzer/analyze.rs
- Many empty enter/leave methods with `// todo!()` comments
- Visitor pattern implementation is verbose

### analyzer/module.rs
- `analyzing: AnalyzeContext` coupled in Module, only used during analysis

### parser/ast.rs
- `BinaryOp::op()` and `UnaryOp::op()` implementations are identical, should extract to trait

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
