//! 声明相关的语义分析

use syntax::ast::*;
use syntax::visitor::DeclVisitor;

use crate::array::ArrayTree;
use crate::error::AnalyzeError;
use crate::module::Module;
use crate::utils::parse_type_node;
use crate::value::Value;

impl DeclVisitor for Module {
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

        let mut field_names = std::collections::HashSet::new();
        let mut field_list = vec![];
        for field in node.fields() {
            let Some((name, range)) = field.name().and_then(|n| utils::extract_name_and_range(&n))
            else {
                continue;
            };

            let Some(ty_node) = field.ty() else {
                continue;
            };

            let ty = match parse_type_node(self, &ty_node, Some(&self.value_table)) {
                Ok(Some(ty)) => ty,
                Ok(None) => {
                    return;
                }
                Err(e) => {
                    self.new_error(e);
                    return;
                }
            };

            if !field_names.insert(name.clone()) {
                self.new_error(AnalyzeError::VariableDefined { name, range });
                continue;
            }
            let field_id = self.new_field(name.clone(), ty, range);
            field_list.push(field_id);
        }

        // TODO: 跨文件的后置分析循环引用
        let Some(struct_def) = self.get_struct_mut_by_id(struct_id) else {
            return;
        };
        struct_def.fields = field_list;
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
            self.new_error(AnalyzeError::VariableDefined {
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
                            self.new_error(AnalyzeError::ArrayError {
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
                        self.new_error(AnalyzeError::ConstantExprExpected {
                            range: init_range_trimmed,
                        });
                        return;
                    }
                }
            }
        } else if is_const {
            // 如果是 const 必须要有初始化列表:w
            self.new_error(AnalyzeError::ExpectInitialVal {
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
