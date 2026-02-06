//! 声明相关的语义分析

use syntax::ast::*;
use syntax::visitor::DeclVisitor;

use crate::array::ArrayTree;
use crate::error::SemanticError;
use crate::module::Module;
use crate::value::Value;

impl DeclVisitor for Module {
    fn enter_comp_unit(&mut self, _node: CompUnit) {
        self.analyzing.current_scope = self.new_scope(None);
        self.global_scope = self.analyzing.current_scope;
    }

    fn enter_struct_def(&mut self, node: StructDef) {
        let Some(name_node) = node.name() else {
            return;
        };
        let Some(name) = name_node.var_name() else {
            return;
        };
        let Some(range) = name_node.var_range() else {
            return;
        };
        // 检查是否重复定义
        if self.find_struct(&name).is_some() {
            self.new_error(SemanticError::StructDefined {
                name: name.clone(),
                range,
            });
            return;
        }

        self.analyzing.current_scope = self.new_scope(Some(self.global_scope));

        // 提前创建占位，以支持自引用结构体
        let struct_id = self.new_struct(name.clone(), vec![], range);
        self.struct_map.insert(name, struct_id);
    }

    fn leave_struct_def(&mut self, node: StructDef) {
        // 获取 struct 名称
        let Some(Some(name)) = node.name().map(|n| n.var_name()) else {
            return;
        };
        // 获取已创建的 struct ID（在 enter_struct_def 中创建）
        let Some(struct_id) = self.find_struct(&name) else {
            return;
        };

        // 收集字段信息（先不创建变量）
        let mut field_infos = Vec::new();
        let mut field_names = std::collections::HashSet::new();

        for field_node in node.fields() {
            let Some(field_name_node) = field_node.name() else {
                continue;
            };
            let Some(field_name) = field_name_node.var_name() else {
                continue;
            };
            let Some(field_range) = field_name_node.var_range() else {
                continue;
            };
            let Some(ty_node) = field_node.ty() else {
                continue;
            };

            // 检查字段名是否重复
            if !field_names.insert(field_name.clone()) {
                self.new_error(SemanticError::VariableDefined {
                    name: field_name.clone(),
                    range: field_range,
                });
                continue;
            }

            // 获取字段类型（已经在 leave_type 中构建好）
            let field_ty = if let Some(ty) = self.get_expr_type(ty_node.text_range()) {
                ty.clone()
            } else {
                self.new_error(SemanticError::TypeUndefined {
                    range: utils::trim_node_text_range(&ty_node),
                });
                continue;
            };

            field_infos.push((field_name, field_ty, field_range));
        }

        // 创建字段变量
        let Some(scope) = self.scopes.get_mut(*self.analyzing.current_scope) else {
            return;
        };
        let Some(parent_scope) = scope.parent else {
            return;
        };
        let mut field_ids = Vec::new();
        for (field_name, field_ty, field_range) in field_infos {
            let field_id = scope.new_variable(
                &mut self.variables,
                &mut self.variable_map,
                field_name,
                field_ty,
                field_range,
            );
            field_ids.push(field_id);
        }

        // 更新 struct 定义的字段
        let struct_def = self.get_struct_mut_by_id(struct_id).unwrap();
        struct_def.fields = field_ids;

        self.analyzing.current_scope = parent_scope;
    }

    fn leave_var_def(&mut self, def: VarDef) {
        let Some(name_node) = def.name() else {
            return;
        };
        let Some(var_name) = name_node.var_name() else {
            return;
        };
        let Some(var_range) = name_node.var_range() else {
            return;
        };
        let Some(ty_node) = def.ty() else {
            return;
        };

        let var_type = if let Some(ty) = self.get_expr_type(ty_node.text_range()) {
            ty.clone()
        } else {
            self.new_error(SemanticError::TypeUndefined {
                range: utils::trim_node_text_range(&ty_node),
            });
            return;
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
