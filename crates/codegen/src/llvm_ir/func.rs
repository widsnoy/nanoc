use analyzer::r#type::NType;
use inkwell::types::BasicType;
use syntax::ast::*;

use crate::error::{CodegenError, Result};
use crate::llvm_ir::Program;

impl<'a, 'ctx> Program<'a, 'ctx> {
    /// 编译函数定义
    pub(super) fn compile_func_def(&mut self, func: FuncDef) -> Result<()> {
        self.compile_func_signature(func.sign().unwrap())?;
        self.compile_func_attach(func.sign().and_then(|n| n.name()), func.block())?;
        Ok(())
    }

    /// 编译函数签名（声明函数但不生成函数体）
    pub(super) fn compile_func_signature(&mut self, func: FuncSign) -> Result<()> {
        let name = func
            .name()
            .and_then(|n| n.var_name())
            .ok_or(CodegenError::Missing("function name"))?;

        // 直接从 analyzer 获取函数信息
        let func_id = self
            .analyzer
            .get_function_id_by_name(&name)
            .ok_or_else(|| CodegenError::UndefinedFunc(name.clone()))?;
        let func_info = self
            .analyzer
            .get_function_by_id(func_id)
            .ok_or_else(|| CodegenError::UndefinedFunc(name.clone()))?;

        let ret_ty = &func_info.ret_type;
        let is_void = matches!(ret_ty, NType::Void);

        // 从 func_info.params 获取参数信息
        let params: Vec<(String, &'a NType)> = func_info
            .params
            .iter()
            .map(|var_id| {
                let var = self.analyzer.variables.get(**var_id).unwrap();
                (var.name.clone(), &var.ty)
            })
            .collect();

        let basic_params = params
            .iter()
            .map(|(_, p)| self.convert_ntype_to_type(p).map(|t| t.into()))
            .collect::<Result<Vec<_>>>()?;

        let ret_llvm_ty = self.convert_ntype_to_type(ret_ty)?;
        let fn_type = if is_void {
            self.context.void_type().fn_type(&basic_params, false)
        } else {
            ret_llvm_ty.fn_type(&basic_params, false)
        };

        let function = self.module.add_function(&name, fn_type, None);
        self.symbols.functions.insert(name.clone(), function);

        Ok(())
    }

    /// 编译函数体（为已声明的函数附加实现）
    pub(super) fn compile_func_attach(
        &mut self,
        name: Option<Name>,
        block: Option<Block>,
    ) -> Result<()> {
        let Some(block) = block else {
            return Ok(());
        };
        let name = name
            .and_then(|n| n.var_name())
            .ok_or(CodegenError::Missing("function name"))?;

        // 获取已声明的函数
        let function = self
            .symbols
            .functions
            .get(&name)
            .copied()
            .ok_or_else(|| CodegenError::UndefinedFunc(name.clone()))?;

        // 从 analyzer 获取函数信息
        let func_id = self
            .analyzer
            .get_function_id_by_name(&name)
            .ok_or_else(|| CodegenError::UndefinedFunc(name.clone()))?;
        let func_info = self
            .analyzer
            .get_function_by_id(func_id)
            .ok_or_else(|| CodegenError::UndefinedFunc(name.clone()))?;

        let ret_ty = &func_info.ret_type;
        let is_void = matches!(ret_ty, NType::Void);

        // 从 func_info.params 获取参数信息
        let params: Vec<(String, &'a NType)> = func_info
            .params
            .iter()
            .map(|var_id| {
                let var = self.analyzer.variables.get(**var_id).unwrap();
                (var.name.clone(), &var.ty)
            })
            .collect();

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
                .map_err(|_| CodegenError::LlvmBuild("parameter store failed"))?;
            self.symbols.insert_var(pname, alloca, param_ty);
        }

        self.compile_block(block)?;

        let ret_llvm_ty = self.convert_ntype_to_type(ret_ty)?;
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
}
