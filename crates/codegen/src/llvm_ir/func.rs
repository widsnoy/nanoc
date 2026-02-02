use analyzer::r#type::NType;
use inkwell::types::BasicType;
use parser::ast::*;

use crate::error::{CodegenError, Result};
use crate::llvm_ir::Program;
use crate::utils::*;

impl<'a, 'ctx> Program<'a, 'ctx> {
    /// 编译函数定义
    pub(super) fn compile_func_def(&mut self, func: FuncDef) -> Result<()> {
        let name = name_text(&func.name().ok_or(CodegenError::Missing("function name"))?)
            .ok_or(CodegenError::Missing("identifier"))?;

        let (ret_ty, is_void) = func
            .func_type()
            .map(|t| self.compile_func_type(t))
            .transpose()?
            .unwrap_or((NType::Int, false));

        let params: Vec<(String, &'a NType)> = func
            .params()
            .map(|ps| {
                ps.params()
                    .map(|p| -> Result<_> {
                        Ok((
                            name_text(&p.name().ok_or(CodegenError::Missing("param name"))?)
                                .ok_or(CodegenError::Missing("identifier"))?,
                            self.compile_func_f_param(p)?,
                        ))
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()?
            .unwrap_or_default();

        let basic_params = params
            .iter()
            .map(|(_, p)| self.convert_ntype_to_type(p).map(|t| t.into()))
            .collect::<Result<Vec<_>>>()?;

        let ret_llvm_ty = self.convert_ntype_to_type(&ret_ty)?;
        let fn_type = if is_void {
            self.context.void_type().fn_type(&basic_params, false)
        } else {
            ret_llvm_ty.fn_type(&basic_params, false)
        };

        let function = self.module.add_function(&name, fn_type, None);
        self.symbols.functions.insert(name.clone(), function);

        let entry = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry);

        let prev_func = self.symbols.current_function;
        self.symbols.current_function = Some(function);
        self.symbols.push_scope();

        for (i, (pname, param_ty)) in params.into_iter().enumerate() {
            let param_val = function
                .get_nth_param(i as u32)
                .ok_or(CodegenError::Missing("parameter"))?;
            param_val.set_name(&pname);

            let alloc_ty = param_val.get_type();
            let alloca = self.create_entry_alloca(function, alloc_ty, &pname)?;
            self.builder
                .build_store(alloca, param_val)
                .map_err(|_| CodegenError::LlvmBuild("parameterstore failed"))?;
            self.symbols.insert_var(pname, alloca, param_ty);
        }

        if let Some(block) = func.block() {
            self.compile_block(block)?;
        }

        let has_term = self
            .builder
            .get_insert_block()
            .and_then(|bb| bb.get_terminator())
            .is_some();
        if !has_term {
            if is_void {
                self.builder.build_return(None).ok();
            } else {
                let zero = ret_llvm_ty.const_zero();
                self.builder.build_return(Some(&zero)).ok();
            }
        }

        self.symbols.pop_scope();
        self.symbols.current_function = prev_func;
        Ok(())
    }

    fn compile_func_type(&mut self, ty: FuncType) -> Result<(NType, bool)> {
        // 从 analyzer 的 type_table 获取已计算的返回类型
        let range = ty.syntax().text_range();
        if let Some(ret_type) = self.analyzer.get_expr_type(range) {
            let is_void = matches!(ret_type, NType::Void);
            return Ok((ret_type.clone(), is_void));
        }

        Err(CodegenError::Missing(
            "calculate func return type in analyzer",
        ))
    }

    fn compile_func_f_param(&mut self, param: FuncFParam) -> Result<&'a NType> {
        let name_token = param
            .name()
            .and_then(|x| x.ident())
            .ok_or(CodegenError::Missing("param name"))?;
        let variable = self
            .analyzer
            .get_varaible(name_token.text_range())
            .ok_or(CodegenError::Missing("param info"))?;
        Ok(&variable.ty)
    }
}
