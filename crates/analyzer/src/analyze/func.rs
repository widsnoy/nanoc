//! 函数相关的语义分析

use syntax::visitor::FuncVisitor;

use syntax::ast::*;

use crate::error::SemanticError;
use crate::module::Module;
use crate::r#type::NType;

impl FuncVisitor for Module {
    fn enter_func_def(&mut self, _: FuncDef) {
        self.analyzing.current_scope = self.new_scope(Some(self.analyzing.current_scope));
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
        let mut param_list = Vec::new();

        let Some(scope) = self.scopes.get(*self.analyzing.current_scope) else {
            return;
        };

        if let Some(params) = node.params() {
            for param in params.params() {
                let Some(name_node) = param.name() else {
                    return;
                };
                let Some(ident) = name_node.ident() else {
                    return;
                };
                let name = ident.text();
                let Some(v) = scope.look_up_variable(self, name) else {
                    return;
                };
                param_list.push(v);
            }
        }

        let ret_type = if let Some(ty_node) = node.ret_type() {
            if let Some(ty) = self.get_expr_type(ty_node.text_range()) {
                ty.clone()
            } else {
                self.new_error(SemanticError::TypeUndefined {
                    range: ty_node.text_range(),
                });
                return;
            }
        } else {
            NType::Void
        };

        let Some(name_node) = node.name() else {
            return;
        };
        let Some(name) = name_node.var_name() else {
            return;
        };
        let Some(range) = name_node.var_range() else {
            return;
        };

        let func_id = self.new_function(name.clone(), param_list, ret_type.clone(), range);
        self.function_map.insert(name, func_id);

        self.analyzing.current_function_ret_type = Some(ret_type);
    }

    fn leave_func_f_param(&mut self, node: FuncFParam) {
        let Some(ty_node) = node.ty() else {
            return;
        };

        let param_type = if let Some(ty) = self.get_expr_type(ty_node.text_range()) {
            ty.clone()
        } else {
            self.new_error(SemanticError::TypeUndefined {
                range: ty_node.text_range(),
            });
            return;
        };

        let Some(name_node) = node.name() else {
            return;
        };
        let Some(name) = name_node.var_name() else {
            return;
        };
        let Some(range) = name_node.var_range() else {
            return;
        };
        let scope = self.scopes.get_mut(*self.analyzing.current_scope).unwrap();

        if scope.have_variable_def(&name) {
            self.new_error(SemanticError::VariableDefined { name, range });
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
}
