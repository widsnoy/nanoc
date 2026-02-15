use analyzer::array::{ArrayTree, ArrayTreeValue};
use analyzer::r#type::Ty;
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValueEnum, IntValue, PointerValue};
use syntax::ast::*;
use utils::find_node_by_range;

use crate::error::{CodegenError, Result};
use crate::llvm_ir::Program;

impl<'a, 'ctx> Program<'a, 'ctx> {
    pub(crate) fn compile_var_def(&mut self, def: VarDef) -> Result<()> {
        let name_node = def.name().ok_or(CodegenError::Missing("variable name"))?;
        let name = name_node
            .var_name()
            .ok_or(CodegenError::Missing("variable name"))?;
        let name_range = name_node
            .var_range()
            .ok_or(CodegenError::Missing("variable range"))?;

        let var = self
            .analyzer
            .get_varaible_by_range(name_range)
            .ok_or(CodegenError::Missing("variable info"))?;
        let var_ty = &var.ty;
        let llvm_ty = self.convert_ntype_to_type(var_ty)?;

        let is_global = self.symbols.current_function.is_none();

        if is_global {
            // 全局变量
            let is_const = var_ty.is_const();
            let init_val = if is_const {
                // const 变量必须有初始值
                let init_node = def.init().ok_or(CodegenError::Missing("initial value"))?;
                self.get_const_var_value(&init_node, Some(llvm_ty))?
            } else {
                self.const_init_or_zero(def.init(), llvm_ty)?
            };

            let global = self.module.add_global(llvm_ty, None, &name);
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
            let alloca = self.create_entry_alloca(func, llvm_ty, &name)?;

            // 判断变量类型
            let ty = var_ty.unwrap_const();

            if let Some(init_node) = def.init() {
                let range = init_node.text_range();

                if let Some(expr) = init_node.expr() {
                    // 单值初始化
                    let init_val = self.compile_expr(expr.clone())?;

                    // 获取表达式类型和变量类型，进行类型转换
                    let expr_ty = self
                        .analyzer
                        .get_expr_type(expr.text_range())
                        .ok_or(CodegenError::Missing("expr type"))?;
                    let init_val_casted = self.cast_value(init_val, expr_ty, var_ty)?;

                    self.builder
                        .build_store(alloca, init_val_casted)
                        .map_err(|_| CodegenError::LlvmBuild("store failed"))?;
                } else if ty.is_array() {
                    // 数组初始化列表
                    let array_tree = self
                        .analyzer
                        .expand_array
                        .get(&range)
                        .ok_or(CodegenError::Missing("array init info"))?;

                    if self.analyzer.is_compile_time_constant(range) {
                        let init_val =
                            self.convert_array_tree_to_global_init(array_tree, llvm_ty)?;
                        self.builder
                            .build_store(alloca, init_val)
                            .map_err(|_| CodegenError::LlvmBuild("store failed"))?;
                    } else {
                        // 非常量数组：先 zero init，再逐个 store
                        self.builder
                            .build_store(alloca, llvm_ty.const_zero())
                            .map_err(|_| CodegenError::LlvmBuild("store failed"))?;
                        let mut indices = vec![self.context.i32_type().const_zero()];
                        // 提取数组元素类型
                        let element_ty = match &ty {
                            Ty::Array(inner, _) => inner.as_ref(),
                            _ => {
                                return Err(CodegenError::TypeMismatch(
                                    "expected array type".into(),
                                ));
                            }
                        };
                        self.store_on_array_tree(
                            array_tree,
                            &mut indices,
                            alloca,
                            llvm_ty,
                            element_ty,
                        )?;
                    }
                } else if ty.is_struct() {
                    // Struct 初始化列表
                    if self.analyzer.is_compile_time_constant(range) {
                        // 常量 struct：直接 store
                        let init_val = self.get_const_var_value(&init_node, Some(llvm_ty))?;
                        self.builder
                            .build_store(alloca, init_val)
                            .map_err(|_| CodegenError::LlvmBuild("store failed"))?;
                    } else {
                        // 非常量 struct：逐字段 store（struct 初始化要求完全覆盖，不需要 zero init）
                        self.store_struct_init(var_ty, init_node, alloca, llvm_ty)?;
                    }
                } else {
                    return Err(CodegenError::Unsupported("init list type".into()));
                }
            } else {
                // 无初始值，zero init
                self.builder
                    .build_store(alloca, llvm_ty.const_zero())
                    .map_err(|_| CodegenError::LlvmBuild("store failed"))?;
            }

            self.symbols.insert_var(name.to_string(), alloca, var_ty);
        }
        Ok(())
    }

    /// 处理非常量 struct 初始化，逐个字段 store
    fn store_struct_init(
        &mut self,
        struct_ty: &Ty,
        init_node: InitVal,
        ptr: PointerValue<'ctx>,
        llvm_ty: BasicTypeEnum<'ctx>,
    ) -> Result<()> {
        let struct_id = struct_ty
            .as_struct_id()
            .ok_or(CodegenError::TypeMismatch("expected struct type".into()))?;
        let struct_def = self
            .analyzer
            .get_struct_by_id(struct_id)
            .ok_or(CodegenError::NotImplemented("undefined struct"))?;

        // Safety: struct_def 的生命周期与 self.analyzer 相同，
        // 而 self.analyzer 在整个编译过程中都是有效的。
        // 这里用裸指针避免 clone 开销，因为我们只需要读取 fields。
        let field_ids: *const [analyzer::module::FieldID] = &struct_def.fields[..];

        let inits: Vec<_> = init_node.inits().collect();

        for (idx, (init, &field_id)) in inits
            .into_iter()
            .zip(unsafe { &*field_ids }.iter())
            .enumerate()
        {
            let field = self.analyzer.get_field_by_id(field_id).unwrap();
            let field_llvm_ty = self.convert_ntype_to_type(&field.ty)?;

            // 获取字段指针
            let field_ptr = self
                .builder
                .build_struct_gep(llvm_ty, ptr, idx as u32, &format!("field.{}", field.name))
                .map_err(|_| CodegenError::LlvmBuild("struct gep failed"))?;

            // 根据字段类型处理初始化
            let inner_field_ty = field.ty.unwrap_const();

            if let Some(expr) = init.expr() {
                // 单值表达式
                let value = if let Ok(const_val) = self.get_const_var_value(&expr, None) {
                    const_val
                } else {
                    self.compile_expr(expr.clone())?
                };

                // 获取表达式类型并进行隐式类型转换
                let expr_ty = self
                    .analyzer
                    .get_expr_type(expr.text_range())
                    .ok_or(CodegenError::Missing("expr type"))?;
                let value_casted = self.cast_value(value, expr_ty, &field.ty)?;

                self.builder
                    .build_store(field_ptr, value_casted)
                    .map_err(|_| CodegenError::LlvmBuild("store failed"))?;
            } else if inner_field_ty.is_array() {
                // 数组字段：使用 ArrayTree 解析
                if self.analyzer.is_compile_time_constant(init.text_range()) {
                    let init_val = self.get_const_var_value(&init, Some(field_llvm_ty))?;
                    self.builder
                        .build_store(field_ptr, init_val)
                        .map_err(|_| CodegenError::LlvmBuild("store failed"))?;
                } else {
                    // 非常量数组需要先 zero init
                    self.builder
                        .build_store(field_ptr, field_llvm_ty.const_zero())
                        .map_err(|_| CodegenError::LlvmBuild("store failed"))?;
                    let mut indices = vec![self.context.i32_type().const_zero()];
                    let array_tree = self.analyzer.expand_array.get(&init.text_range()).unwrap();
                    // 提取数组元素类型
                    let element_ty = match &field.ty {
                        Ty::Array(inner, _) => inner.as_ref(),
                        Ty::Const(inner) => match inner.as_ref() {
                            Ty::Array(inner, _) => inner.as_ref(),
                            _ => {
                                return Err(CodegenError::TypeMismatch(
                                    "expected array type".into(),
                                ));
                            }
                        },
                        _ => return Err(CodegenError::TypeMismatch("expected array type".into())),
                    };
                    self.store_on_array_tree(
                        array_tree,
                        &mut indices,
                        field_ptr,
                        field_llvm_ty,
                        element_ty,
                    )?;
                }
            } else if inner_field_ty.is_struct() {
                // 嵌套 struct 字段：递归处理（不需要 zero init）
                if self.analyzer.is_compile_time_constant(init.text_range()) {
                    let init_val = self.get_const_var_value(&init, Some(field_llvm_ty))?;
                    self.builder
                        .build_store(field_ptr, init_val)
                        .map_err(|_| CodegenError::LlvmBuild("store failed"))?;
                } else {
                    self.store_struct_init(&field.ty, init, field_ptr, field_llvm_ty)?;
                }
            } else {
                return Err(CodegenError::Unsupported(
                    "unsupported field init type".into(),
                ));
            }
        }

        Ok(())
    }

    /// 遍历 ArrayTree 叶子节点并存储初始化值
    fn store_on_array_tree(
        &mut self,
        array_tree: &ArrayTree,
        indices: &mut Vec<IntValue<'ctx>>,
        ptr: PointerValue<'ctx>,
        llvm_ty: BasicTypeEnum<'ctx>,
        element_ty: &Ty,
    ) -> Result<()> {
        match array_tree {
            ArrayTree::Val(ArrayTreeValue::Expr(expr_range)) => {
                let syntax_tree = SyntaxNode::new_root(self.analyzer.get_green_tree());
                let expr = find_node_by_range::<Expr>(&syntax_tree, *expr_range)
                    .ok_or(CodegenError::Missing("expr node not found"))?;
                let value = self.compile_expr(expr.clone())?;

                // 获取表达式类型并进行隐式类型转换
                let expr_ty = self
                    .analyzer
                    .get_expr_type(expr.text_range())
                    .ok_or(CodegenError::Missing("expr type"))?;
                let value_casted = self.cast_value(value, expr_ty, element_ty)?;

                let gep = unsafe {
                    self.builder
                        .build_gep(llvm_ty, ptr, indices, "idx.gep")
                        .map_err(|_| CodegenError::LlvmBuild("gep failed"))?
                };
                self.builder
                    .build_store(gep, value_casted)
                    .map_err(|_| CodegenError::LlvmBuild("store failed"))?;
            }
            ArrayTree::Val(ArrayTreeValue::Struct {
                struct_id: id,
                init_list: list_range,
            }) => {
                let struct_name = self
                    .analyzer
                    .get_struct_by_id(*id)
                    .map(|s| s.name)
                    .unwrap_or_else(|| format!("struct#{:?}", id.index));
                let struct_ty = Ty::Struct {
                    id: *id,
                    name: struct_name,
                };
                let llvm_ty = self.convert_ntype_to_type(&struct_ty)?;
                let gep = unsafe {
                    self.builder
                        .build_gep(llvm_ty, ptr, indices, "idx.gep")
                        .map_err(|_| CodegenError::LlvmBuild("gep failed"))?
                };
                if let Ok(const_val) = self.get_const_var_value_by_range(*list_range, Some(llvm_ty))
                {
                    self.builder
                        .build_store(gep, const_val)
                        .map_err(|_| CodegenError::LlvmBuild("store failed"))?;
                } else {
                    let syntax_tree = SyntaxNode::new_root(self.analyzer.get_green_tree());
                    let list = find_node_by_range::<InitVal>(&syntax_tree, *list_range)
                        .ok_or(CodegenError::Missing("init_val node not found"))?;
                    self.store_struct_init(&struct_ty, list, gep, llvm_ty)?;
                }
            }
            ArrayTree::Children(children) => {
                let i32_type = self.context.i32_type();
                // 对于多维数组，element_ty 本身也是数组类型
                // 需要提取下一层的元素类型
                for (i, child) in children.iter().enumerate() {
                    indices.push(i32_type.const_int(i as u64, false));
                    // 根据当前 element_ty 确定下一层的类型
                    match element_ty {
                        Ty::Array(inner, _) => {
                            self.store_on_array_tree(child, indices, ptr, llvm_ty, inner.as_ref())?;
                        }
                        _ => {
                            self.store_on_array_tree(child, indices, ptr, llvm_ty, element_ty)?;
                        }
                    }
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
        let range = init.text_range();
        if let Some(value) = self.analyzer.get_value_by_range(range) {
            return self.convert_value(value, Some(ty));
        }
        Err(CodegenError::Missing("init value"))
    }
}
