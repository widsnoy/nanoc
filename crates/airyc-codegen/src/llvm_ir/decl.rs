use airyc_analyzer::array::{ArrayTree, ArrayTreeValue};
use airyc_parser::ast::*;
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValueEnum, IntValue, PointerValue};

use crate::error::{CodegenError, Result};
use crate::llvm_ir::Program;
use crate::utils::*;

impl<'a, 'ctx> Program<'a, 'ctx> {
    pub(super) fn compile_global_decl(&mut self, decl: VarDecl) -> Result<()> {
        self.compile_var_decl(decl)
    }

    pub(super) fn compile_local_decl(&mut self, decl: VarDecl) -> Result<()> {
        self.compile_var_decl(decl)
    }

    fn compile_var_decl(&mut self, decl: VarDecl) -> Result<()> {
        let is_const = decl.is_const();
        for def in decl.var_defs() {
            self.compile_var_def(def, is_const)?;
        }
        Ok(())
    }

    fn compile_var_def(&mut self, def: VarDef, is_const: bool) -> Result<()> {
        let name_token = get_ident_node(
            &def.index_val()
                .ok_or(CodegenError::Missing("variable name"))?,
        )
        .ok_or(CodegenError::Missing("identifier"))?;

        let var = self
            .analyzer
            .get_varaible(name_token.text_range())
            .ok_or(CodegenError::Missing("variable info"))?;
        let name = name_token.text();
        let var_ty = &var.ty;
        let basic_ty = self.convert_ntype_to_type(var_ty)?;

        let is_global = self.symbols.current_function.is_none();

        if is_global {
            // 全局变量
            let init_val = if is_const {
                // const 变量必须有初始值
                let init_node = def.init().ok_or(CodegenError::Missing("initial value"))?;
                self.compile_const_init_val(init_node, basic_ty)?
            } else {
                self.const_init_or_zero(def.init(), basic_ty)?
            };

            let global = self.module.add_global(basic_ty, None, name);
            global.set_initializer(&init_val);
            if is_const {
                global.set_constant(true);
            }
            self.symbols.globals.insert(
                name.to_string(),
                crate::llvm_ir::Symbol::new(global.as_pointer_value(), var_ty),
            );
        } else {
            // 局部变量
            let func = self
                .symbols
                .current_function
                .ok_or(CodegenError::Missing("current function"))?;
            let alloca = self.create_entry_alloca(func, basic_ty, name)?;

            let (init_val, array_tree) = if let Some(init_node) = def.init() {
                if let Some(expr) = init_node.expr() {
                    // 单值初始化
                    if is_const {
                        // const 变量尝试获取编译时常量值
                        if let Ok(const_val) = self.get_const_var_value(&expr) {
                            (Some(const_val), None)
                        } else {
                            // 运行时初始化
                            (Some(self.compile_expr(expr)?), None)
                        }
                    } else {
                        (Some(self.compile_expr(expr)?), None)
                    }
                } else {
                    // 数组初始化
                    let range = init_node.syntax().text_range();
                    let array_tree = self
                        .analyzer
                        .expand_array
                        .get(&range)
                        .ok_or(CodegenError::Missing("array init info"))?;
                    if self.analyzer.is_compile_time_constant(range) {
                        (
                            Some(self.convert_array_tree_to_global_init(array_tree, basic_ty)?),
                            None,
                        )
                    } else {
                        (None, Some(array_tree))
                    }
                }
            } else if is_const {
                return Err(CodegenError::Missing("const requires initial value"));
            } else {
                (Some(basic_ty.const_zero()), None)
            };

            if let Some(init_val) = init_val {
                self.builder
                    .build_store(alloca, init_val)
                    .map_err(|_| CodegenError::LlvmBuild("store failed"))?;
            } else {
                let array_tree = array_tree.ok_or(CodegenError::Missing("array tree"))?;
                self.builder
                    .build_store(alloca, basic_ty.const_zero())
                    .map_err(|_| CodegenError::LlvmBuild("store failed"))?;
                let mut indices = vec![self.context.i32_type().const_zero()];
                self.walk_on_array_tree(array_tree, &mut indices, alloca, basic_ty)?;
            }
            self.symbols.insert_var(name.to_string(), alloca, var_ty);
        }
        Ok(())
    }

    fn compile_const_init_val(
        &mut self,
        init: InitVal,
        ty: BasicTypeEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        if let Some(expr) = init.expr() {
            return self.get_const_var_value(&expr);
        }
        let range = init.syntax().text_range();
        let array_tree = self
            .analyzer
            .expand_array
            .get(&range)
            .ok_or(CodegenError::Missing("array init info"))?;
        self.convert_array_tree_to_global_init(array_tree, ty)
    }

    /// 遍历 ArrayTree 叶子节点并存储初始化值
    fn walk_on_array_tree(
        &mut self,
        array_tree: &ArrayTree,
        indices: &mut Vec<IntValue<'ctx>>,
        ptr: PointerValue<'ctx>,
        elem_ty: BasicTypeEnum<'ctx>,
    ) -> Result<()> {
        match array_tree {
            ArrayTree::Val(ArrayTreeValue::Expr(expr)) => {
                // 尝试获取编译时常量值，否则编译表达式
                let value = if let Ok(const_val) = self.get_const_var_value(expr) {
                    const_val
                } else {
                    self.compile_expr(expr.clone())?
                };
                let gep = unsafe {
                    self.builder
                        .build_gep(elem_ty, ptr, indices, "idx.gep")
                        .map_err(|_| CodegenError::LlvmBuild("gep failed"))?
                };
                self.builder
                    .build_store(gep, value)
                    .map_err(|_| CodegenError::LlvmBuild("store failed"))?;
            }
            ArrayTree::Children(children) => {
                let i32_type = self.context.i32_type();
                for (i, child) in children.iter().enumerate() {
                    indices.push(i32_type.const_int(i as u64, false));
                    self.walk_on_array_tree(child, indices, ptr, elem_ty)?;
                    indices.pop();
                }
            }
            ArrayTree::Val(ArrayTreeValue::Empty) => {
                // Empty 值已经被 zeroinitializer 处理，不需要额外操作
            }
        }
        Ok(())
    }

    /// Global variable initialization (default 0)
    fn const_init_or_zero(
        &mut self,
        init: Option<InitVal>,
        ty: BasicTypeEnum<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>> {
        let Some(init) = init else {
            return Ok(ty.const_zero());
        };
        let range = init.syntax().text_range();
        if let Some(value) = self.analyzer.get_value(range) {
            return self.convert_value(value);
        }
        if let Some(array_tree) = self.analyzer.expand_array.get(&range) {
            return self.convert_array_tree_to_global_init(array_tree, ty);
        }
        Err(CodegenError::Missing("init value"))
    }
}
