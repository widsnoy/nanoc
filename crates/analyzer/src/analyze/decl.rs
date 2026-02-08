//! 声明相关的语义分析

use syntax::ast::*;
use syntax::visitor::DeclVisitor;

use crate::array::ArrayTree;
use crate::error::SemanticError;
use crate::module::{FieldID, Module};
use crate::r#type::NType;
use crate::value::Value;

impl DeclVisitor for Module<'_> {
    fn enter_comp_unit(&mut self, node: CompUnit) {
        self.analyzing.current_scope = self.new_scope(None, node.text_range());
        self.global_scope = self.analyzing.current_scope;
    }

    fn leave_struct_def(&mut self, node: StructDef) {
        // 获取 struct 名称
        let Some(Some(name)) = node.name().map(|n| n.var_name()) else {
            return;
        };
        // 获取 struct id
        let Some(struct_id) = self.get_struct_id_by_name(&name) else {
            return;
        };

        let field_ids: *const [FieldID] = if let Some(struct_def) = self.get_struct_by_id(struct_id)
        {
            &struct_def.fields[..]
        } else {
            return;
        };

        let mut field_names = std::collections::HashSet::new();
        for field_id in unsafe { &*field_ids } {
            if let Some(field) = self.get_field_by_id(*field_id)
                && !field_names.insert(field.name.clone())
            {
                self.new_error(SemanticError::VariableDefined {
                    name: field.name.clone(),
                    range: field.range,
                });
            }
        }

        for field_id in unsafe { &*field_ids } {
            if let Some(field) = self.get_field_by_id(*field_id) {
                let mut ty = &field.ty;
                let self_refer = loop {
                    match ty {
                        NType::Struct { id: idx, .. } if *idx == struct_id => break true,
                        NType::Array(inner, _) => ty = inner,
                        _ => break false,
                    }
                };

                if self_refer {
                    self.new_error(SemanticError::StructSelfRef {
                        name: field.name.clone(),
                        range: field.range,
                    });
                    return;
                }
            }
        }
    }

    fn leave_var_def(&mut self, def: VarDef) {
        let Some((var_name, var_range)) =
            def.name().and_then(|n| utils::extract_name_and_range(&n))
        else {
            return;
        };
        let Some(ty_node) = def.ty() else {
            return;
        };

        let var_type = match crate::utils::parse_type_node(self, &ty_node, Some(&self.value_table))
        {
            Ok(Some(ty)) => ty,
            Ok(None) => {
                return;
            }
            Err(e) => {
                self.new_error(e);
                return;
            }
        };

        let current_scope = self.analyzing.current_scope;
        let scope = self.scopes.get_mut(*current_scope).unwrap();
        let is_global = current_scope == self.global_scope;
        let is_const = var_type.is_const();
        if scope.have_variable_def(&var_name) {
            self.new_error(SemanticError::VariableDefined {
                name: var_name,
                range: var_range,
            });
            return;
        }

        // 处理初始值
        if let Some(init_val_node) = def.init() {
            // 如果是表达式，已经在 expr 处理，所以只用考虑 Array 和 Struct 类型
            let init_range = init_val_node.text_range();
            let init_range_trimmed = utils::trim_node_text_range(&init_val_node);
            // 如果 InitVal 包含一个表达式，使用表达式的范围
            let expr_range = init_val_node
                .expr()
                .map(|e| e.text_range())
                .unwrap_or(init_range);
            if var_type.is_array() {
                let (array_tree, is_const_list) =
                    match ArrayTree::new(self, &var_type, init_val_node) {
                        Ok(s) => s,
                        Err(e) => {
                            self.new_error(SemanticError::ArrayError {
                                message: Box::new(e),
                                range: init_range_trimmed,
                            });
                            return;
                        }
                    };
                if is_const_list {
                    self.value_table
                        .insert(init_range, Value::Array(array_tree.clone()));
                }
                self.expand_array.insert(init_range, array_tree);
            } else if var_type.is_struct() {
                let struct_id = var_type.as_struct_id().unwrap();
                match self.process_struct_init_value(struct_id, init_val_node) {
                    Ok(Some(value)) => {
                        self.value_table.insert(init_range, value);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        self.new_error(e);
                        return;
                    }
                }
            }

            match self.value_table.get(&expr_range) {
                Some(v) => {
                    // 如果是 const ，给变量设置一下初值
                    if is_const {
                        self.value_table.insert(var_range, v.clone());
                    }
                }
                None => {
                    // global 变量必须编译时能求值
                    if is_global {
                        self.new_error(SemanticError::ConstantExprExpected {
                            range: init_range_trimmed,
                        });
                        return;
                    }
                }
            }
        } else if is_const {
            // 如果是 const 必须要有初始化列表:w
            self.new_error(SemanticError::ExpectInitialVal {
                name: var_name,
                range: var_range,
            });
            return;
        }

        let scope = self.scopes.get_mut(*self.analyzing.current_scope).unwrap();
        let _ = scope.new_variable(
            &mut self.variables,
            &mut self.variable_map,
            var_name,
            var_type,
            var_range,
        );
    }
}
