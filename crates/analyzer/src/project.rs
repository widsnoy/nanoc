use std::{collections::HashMap, path::PathBuf};

use parser::parse::Parser;
use thunderdome::Arena;
use vfs::{FileID, Vfs};

use crate::{
    header::HeaderAnalyzer,
    module::{Module, ModuleID, StructID},
    r#type::NType::Void,
};

#[derive(Debug, Default)]
pub struct Project {
    pub modules: Arena<Module<'static>>,
    /// 如果是文件，代表分析单文件
    pub workspace: PathBuf,
    pub vfs: Vfs,
    pub file_index: HashMap<FileID, ModuleID>,
}

impl Project {
    pub fn initialize(&mut self, workspace: PathBuf, vfs: Vfs) {
        self.workspace = workspace;
        self.vfs = vfs;

        for (index, file) in self.vfs.files.iter() {
            // 语法分析
            let parser = Parser::new(&file.text);
            let (green_tree, _, _) = parser.parse(); // FIXME: 先忽略错误

            // 初始化 Module
            let id = self.modules.insert(Module::new(green_tree.clone()));
            let module_id = ModuleID(id);
            let module = self.modules.get_mut(id).unwrap();
            module.module_id = module_id;
            self.file_index.insert(FileID(index), module_id);

            // 符号分析：分配 ID
            Self::collect_symbols_for_module(module);
        }

        // 引用模块分析
        for (file_id, &module_id) in &self.file_index {
            let module = self.modules.get(module_id.0).unwrap();
            let module_imports = HeaderAnalyzer::collect_module_imports(
                module,
                *file_id,
                &self.vfs,
                &self.file_index,
                &self.modules,
            );

            let module = self.modules.get_mut(module_id.0).unwrap();
            HeaderAnalyzer::apply_module_imports(module, module_imports);
        }

        // 填充 struct 字段和 function 返回类型
        for (id, module) in self.modules.iter_mut() {
            Self::fill_definitions(module, ModuleID(id));
        }

        // 语义分析
        // 安全性：Project 在整个分析期间保持不变，指针在 analyze() 结束后被清除
        let project_ptr = self as *const Project;

        for (_, module) in self.modules.iter_mut() {
            // 设置 project 指针用于跨模块访问
            module.analyzing.project = Some(unsafe { &*project_ptr });
            module.analyze();
        }
    }

    /// 为模块收集符号并分配 ID
    fn collect_symbols_for_module(module: &mut Module) {
        use crate::module::{Function, Struct};
        use syntax::{
            AstNode as _, SyntaxNode,
            ast::{FuncDef, StructDef},
        };

        let root = SyntaxNode::new_root(module.green_tree.clone());
        let module_id = module.module_id;

        for ele in root.children() {
            if let Some(func_def) = FuncDef::cast(ele.clone()) {
                // 解析函数名
                if let Some(name) = func_def
                    .sign()
                    .and_then(|n| n.name())
                    .and_then(|n| n.var_name())
                {
                    let range = func_def.text_range();
                    let have_impl = func_def.block().is_some();

                    // 创建空的 Function（参数和返回类型稍后填充）
                    let function = Function {
                        name: name.clone(),
                        params: vec![],
                        ret_type: Void,
                        have_impl,
                        range,
                    };

                    let idx = module.functions.insert(function);
                    let func_id = crate::module::FunctionID::new(module_id, idx);

                    // 更新 function_map
                    module.function_map.insert(name, func_id);
                }
            } else if let Some(struct_def) = StructDef::cast(ele) {
                // 解析结构体名
                if let Some(name) = struct_def.name().and_then(|n| n.var_name()) {
                    let range = struct_def.text_range();

                    // 创建空的 Struct（字段稍后填充）
                    let struct_data = Struct {
                        name: name.clone(),
                        fields: vec![],
                        range,
                    };

                    let idx = module.structs.insert(struct_data);
                    let struct_id = StructID::new(module_id, idx);

                    // 更新 struct_map
                    module.struct_map.insert(name, struct_id);
                }
            }
        }
    }

    /// 填充模块的 struct 和 function 定义
    /// Struct: 填充字段
    /// Function: 只填充返回类型
    fn fill_definitions(module: &mut Module, module_id: ModuleID) {
        use crate::module::{Field, FieldID};
        use syntax::{
            AstNode as _, SyntaxNode,
            ast::{FuncDef, StructDef},
        };

        let root = SyntaxNode::new_root(module.green_tree.clone());

        // 填充 struct 定义
        let struct_defs: Vec<_> = root.children().filter_map(StructDef::cast).collect();

        for struct_def in struct_defs {
            if let Some(name) = struct_def.name().and_then(|n| n.var_name()) {
                let Some(&struct_id) = module.struct_map.get(&name) else {
                    continue;
                };

                let mut field_ids = Vec::new();
                for field_node in struct_def.fields() {
                    if let Some(field_name) = field_node.name().and_then(|n| n.var_name())
                        && let Some(ty_node) = field_node.ty()
                    {
                        // 使用 utils 中的类型解析函数（阶段 2：不进行常量折叠）
                        match crate::utils::parse_type_node(module, &ty_node, None) {
                            Ok(Some(field_ty)) => {
                                let field = Field {
                                    name: field_name,
                                    ty: field_ty,
                                    range: field_node.text_range(),
                                };

                                let idx = module.fields.insert(field);
                                let field_id = FieldID::new(module_id, idx);
                                field_ids.push(field_id);
                            }
                            Ok(None) => {}
                            Err(e) => {
                                module.semantic_errors.push(e);
                            }
                        }
                    }
                }

                if let Some(struct_data) = module.get_struct_mut_by_id(struct_id) {
                    struct_data.fields = field_ids;
                }
            }
        }

        // 填充 function 定义
        let func_defs: Vec<_> = root.children().filter_map(FuncDef::cast).collect();

        for func_def in func_defs {
            if let Some(sign) = func_def.sign()
                && let Some(name) = sign.name().and_then(|n| n.var_name())
            {
                let Some(&func_id) = module.function_map.get(&name) else {
                    continue;
                };

                let ret_type = if let Some(ty_node) = sign.ret_type() {
                    match crate::utils::parse_type_node(module, &ty_node, None) {
                        Ok(Some(ty)) => ty,
                        Ok(None) => {
                            continue;
                        }
                        Err(e) => {
                            module.semantic_errors.push(e);
                            Void
                        }
                    }
                } else {
                    Void
                };

                // 更新 function 定义
                if let Some(func_data) = module.get_function_mut_by_id(func_id) {
                    func_data.ret_type = ret_type;
                }
            }
        }
    }
}
