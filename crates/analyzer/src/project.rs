use std::sync::Arc;

use dashmap::DashMap;
use parser::parse::Parser;
use syntax::{
    AstNode as _, SyntaxNode,
    ast::{FuncDef, StructDef},
};
use vfs::{FileID, Vfs};

use crate::{
    header::HeaderAnalyzer,
    module::{Field, FieldID, Module, ThinModule},
    r#type::NType,
};

#[derive(Debug)]
pub struct Project {
    pub modules: DashMap<FileID, Module>,
    pub metadata: Arc<DashMap<FileID, ThinModule>>,
    pub vfs: Vfs,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            modules: DashMap::new(),
            metadata: Arc::new(DashMap::new()),
            vfs: Vfs::default(),
        }
    }
}

impl Project {
    pub fn initialize(&mut self, vfs: Vfs) {
        self.vfs = vfs;

        // 初始化所有 module，语法分析
        for (index, file) in self.vfs.files.iter() {
            let file_id = FileID(index);

            let parser = Parser::new(&file.text);
            let (green_tree, errors) = parser.parse();

            let mut module = Module::new(green_tree.clone());
            module.file_id = file_id;
            errors.into_iter().for_each(|e| {
                module
                    .semantic_errors
                    .push(crate::error::SemanticError::ParserError(e))
            });

            Self::collect_symbols_for_module(&mut module);

            self.modules.insert(file_id, module);
        }

        // 分析头文件
        for entry in self.modules.iter() {
            let file_id = *entry.key();
            let module = entry.value();
            let module_imports =
                HeaderAnalyzer::collect_module_imports(module, file_id, &self.vfs, &self.modules);

            drop(entry);

            if let Some(mut module) = self.modules.get_mut(&file_id) {
                HeaderAnalyzer::apply_module_imports(&mut module, module_imports);
            }
        }

        // 预处理元数据，跨文件使用
        for mut entry in self.modules.iter_mut() {
            Self::fill_definitions(entry.value_mut());
        }

        for entry in self.modules.iter() {
            self.metadata
                .insert(*entry.key(), ThinModule::new(entry.value()));
        }

        // 语法分析
        let metadata_arc = Arc::clone(&self.metadata);
        for mut entry in self.modules.iter_mut() {
            let module = entry.value_mut();
            module.metadata = Some(Arc::clone(&metadata_arc));
            module.analyze();
            module.metadata = None;
        }

        // 重新拷贝分析完成的元数据
        // TODO：看看能不能优化
        for entry in self.modules.iter() {
            self.metadata
                .insert(*entry.key(), ThinModule::new(entry.value()));
        }
    }

    /// 为代码生成准备：重新设置所有模块的 MetaData
    pub fn prepare_for_codegen(&mut self) {
        let metadata_arc = Arc::clone(&self.metadata);
        for mut entry in self.modules.iter_mut() {
            entry.value_mut().metadata = Some(Arc::clone(&metadata_arc));
        }
    }

    /// 为模块收集符号并分配 ID
    pub fn collect_symbols_for_module(module: &mut Module) {
        let root = SyntaxNode::new_root(module.green_tree.clone());
        for ele in root.children() {
            if let Some(func_def) = FuncDef::cast(ele.clone()) {
                if let Some((name, range)) = func_def
                    .sign()
                    .and_then(|n| n.name())
                    .and_then(|n| utils::extract_name_and_range(&n))
                {
                    let func_id = module.new_function(
                        name.clone(),
                        vec![],
                        vec![],
                        NType::Void,
                        false,
                        range,
                    );
                    module.function_map.insert(name, func_id);
                }
            } else if let Some(struct_def) = StructDef::cast(ele)
                && let Some((name, range)) = struct_def
                    .name()
                    .and_then(|n| utils::extract_name_and_range(&n))
            {
                let struct_id = module.new_struct(name.clone(), vec![], range);
                module.struct_map.insert(name, struct_id);
            }
        }
    }

    /// 填充模块的 struct 和 function 定义
    /// Struct: 字段
    /// Function: 返回类型
    pub fn fill_definitions(module: &mut Module) {
        let root = SyntaxNode::new_root(module.green_tree.clone());

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
                        match crate::utils::parse_type_node(module, &ty_node, None) {
                            Ok(Some(field_ty)) => {
                                let field = Field {
                                    name: field_name,
                                    ty: field_ty,
                                    range: field_node.text_range(),
                                };

                                let idx = module.fields.insert(field);
                                let field_id = FieldID::new(module.file_id, idx);
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
                            NType::Void
                        }
                    }
                } else {
                    NType::Void
                };

                if let Some(func_data) = module.get_function_mut_by_id(func_id) {
                    func_data.ret_type = ret_type;
                }
            }
        }
    }
}
