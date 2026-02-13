//! 函数相关的语义分析

use syntax::visitor::FuncVisitor;

use syntax::ast::*;

use crate::error::AnalyzeError;
use crate::module::Module;
use crate::r#type::Ty;

impl FuncVisitor for Module {
    fn enter_func_def(&mut self, node: FuncDef) {
        self.analyzing.current_scope =
            self.new_scope(Some(self.analyzing.current_scope), node.text_range());
    }

    fn leave_func_def(&mut self, _: FuncDef) {
        let Some(scope) = self.scopes.get(*self.analyzing.current_scope) else {
            return;
        };
        let Some(parent_scope) = scope.parent else {
            return;
        };
        self.analyzing.current_scope = parent_scope;
        self.analyzing.current_function_ret_type = None;
    }

    fn leave_func_sign(&mut self, node: FuncSign) {
        let mut param_list = vec![];
        let mut meta_type_list = vec![];
        let mut is_variadic = false;

        let Some(scope) = self.scopes.get(*self.analyzing.current_scope) else {
            return;
        };

        // 收集参数 VariableID 和检测可变参数
        if let Some(params) = node.params() {
            for param in params.params() {
                // 检查是否为可变参数
                if param.is_variadic() {
                    is_variadic = true;
                    break;
                }

                let Some(name_node) = param.name() else {
                    return;
                };
                let Some(ident) = name_node.ident() else {
                    return;
                };
                let name = ident.text();
                let Some(vid) = scope.look_up_variable(self, name) else {
                    return;
                };
                let Some(var) = self.get_varaible_by_id(vid) else {
                    return;
                };
                param_list.push(vid);
                meta_type_list.push((name.to_string(), var.ty.clone()));
            }
        }

        let ret_type = if let Some(ty_node) = node.ret_type() {
            match crate::utils::parse_type_node(self, &ty_node, Some(&self.value_table)) {
                Ok(Some(ty)) => ty,
                Ok(None) => {
                    return;
                }
                Err(e) => {
                    self.new_error(e);
                    return;
                }
            }
        } else {
            Ty::Void
        };

        let Some(name) = node.name().and_then(|n| n.var_name()) else {
            return;
        };

        let have_impl = node
            .syntax()
            .parent()
            .and_then(FuncDef::cast)
            .and_then(|x| x.block())
            .is_some();

        if let Some(&func_id) = self.function_map.get(&name) {
            // 更新现有的 Function，填充参数
            if let Some(func_data) = self.get_function_mut_by_id(func_id) {
                func_data.params = param_list;
                func_data.meta_types = meta_type_list;
                func_data.ret_type = ret_type.clone();
                func_data.have_local_impl = have_impl;
                func_data.is_variadic = is_variadic;
            }
        } else {
            debug_assert!(false);
        }
        self.analyzing.current_function_ret_type = Some(ret_type);
    }

    fn leave_func_f_param(&mut self, node: FuncFParam) {
        // 如果是可变参数，直接返回（不需要创建变量）
        if node.is_variadic() {
            return;
        }

        let Some((name, range)) = node.name().and_then(|n| utils::extract_name_and_range(&n))
        else {
            return;
        };
        let Some(ty_node) = node.ty() else {
            return;
        };
        let param_type =
            match crate::utils::parse_type_node(self, &ty_node, Some(&self.value_table)) {
                Ok(Some(t)) => t,
                Ok(None) => {
                    return;
                }
                Err(e) => {
                    self.semantic_errors.push(e);
                    return;
                }
            };

        // 检查是否为非法的 void 使用
        if param_type.is_invalid_void_usage() {
            self.new_error(AnalyzeError::InvalidVoidUsage {
                range: ty_node.text_range(),
            });
            return;
        }

        let scope = self.scopes.get_mut(*self.analyzing.current_scope).unwrap();

        if scope.have_variable_def(&name) {
            self.new_error(AnalyzeError::VariableDefined { name, range });
            return;
        }

        scope.new_variable(
            &mut self.variables,
            &mut self.variable_map,
            name,
            param_type,
            range,
        );
    }
    fn enter_func_attach(&mut self, node: FuncAttach) {
        // 新建作用域，导入函数签名的变量
        self.analyzing.current_scope =
            self.new_scope(Some(self.analyzing.current_scope), node.text_range());

        let Some((func_name, func_var)) =
            node.name().and_then(|n| utils::extract_name_and_range(&n))
        else {
            return;
        };

        let Some(func_id) = self.get_function_id_by_name(&func_name) else {
            self.new_error(AnalyzeError::FunctionUndefined {
                name: func_name,
                range: func_var,
            });
            return;
        };

        // 不能实现外部函数
        if func_id.module != self.file_id {
            self.new_error(AnalyzeError::ImplementExternalFunction {
                name: func_name,
                range: func_var,
            });
            return;
        }

        let Some(func) = self.functions.get_mut(func_id.index) else {
            return;
        };

        if func.have_local_impl {
            self.new_error(AnalyzeError::FunctionImplemented {
                name: func_name,
                range: func_var,
            });
            return;
        }

        func.have_local_impl = true;

        let scope = self.scopes.get_mut(*self.analyzing.current_scope).unwrap();
        for (var_id, var_name) in func.params.iter().zip(func.meta_types.iter().map(|f| &f.0)) {
            scope.variables.insert(var_name.clone(), *var_id);
        }

        self.analyzing.current_function_ret_type = Some(func.ret_type.clone());
    }

    fn leave_func_attach(&mut self, _node: FuncAttach) {
        let Some(scope) = self.scopes.get(*self.analyzing.current_scope) else {
            return;
        };
        let Some(parent_scope) = scope.parent else {
            return;
        };
        self.analyzing.current_scope = parent_scope;
        self.analyzing.current_function_ret_type = None;
    }
}
