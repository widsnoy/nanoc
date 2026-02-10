use std::{collections::HashMap, sync::Arc};

use parser::parse::Parser;
use syntax::{
    AstNode as _, SyntaxNode,
    ast::{FuncDef, StructDef},
};
use vfs::{FileID, Vfs};

use crate::{
    checker::ProjectChecker,
    header::HeaderAnalyzer,
    module::{CiterInfo, Field, FieldID, Module, ModuleIndex, ThinModule},
    r#type::NType,
};

#[derive(Default, Debug)]
pub struct Project {
    pub modules: HashMap<FileID, Module>,
    pub metadata: Arc<HashMap<FileID, ThinModule>>,
    pub(crate) checker: Vec<Box<dyn ProjectChecker>>,
}

impl Project {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_checker<T: ProjectChecker + Default + 'static>(mut self) -> Self {
        self.checker.push(Box::new(T::default()));
        self
    }

    /// 全量初始化
    pub fn full_initialize(&mut self, vfs: &Vfs) {
        // 初始化所有 module，语法分析
        vfs.for_each_file(|file_id, file| {
            let parser = Parser::new(&file.text);
            let (green_tree, errors) = parser.parse();

            let mut module = Module::new(green_tree.clone());
            module.file_id = file_id;
            errors.into_iter().for_each(|e| {
                module
                    .semantic_errors
                    .push(crate::error::AnalyzeError::ParserError(e))
            });

            // 收集符号并分配 ID
            Self::allocate_module_symbols(&mut module);

            self.modules.insert(file_id, module);
        });

        // 分析头文件
        let mut all_imports = Vec::with_capacity(self.modules.len());
        for (file_id, module) in &self.modules {
            let module_imports =
                HeaderAnalyzer::collect_module_imports(module, *file_id, vfs, &self.modules);
            all_imports.push((*file_id, module_imports));
        }
        for (file_id, module_imports) in all_imports {
            if let Some(module) = self.modules.get_mut(&file_id) {
                HeaderAnalyzer::apply_module_imports(module, module_imports);
            }
        }

        // 预处理元数据，跨文件使用
        for module in self.modules.values_mut() {
            Self::fill_definitions(module);
        }

        let mut metadata: HashMap<FileID, ThinModule> = HashMap::new();
        for (file_id, module) in &self.modules {
            metadata.insert(*file_id, ThinModule::new(module));
        }

        // 语义分析
        let metadata_rc = Arc::new(metadata);
        for module in self.modules.values_mut() {
            module.metadata = Some(Arc::clone(&metadata_rc));
            module.analyze();
            module.metadata = None;
        }

        // 重新拷贝分析完成的元数据
        let mut metadata: HashMap<FileID, ThinModule> = HashMap::new();
        for (file_id, module) in &self.modules {
            metadata.insert(*file_id, ThinModule::new(module));
        }
        self.metadata = Arc::new(metadata);

        // 构建索引
        let mut temp: HashMap<FileID, ModuleIndex> = Default::default();
        for module in self.modules.values() {
            for (_, refer) in &module.reference {
                match refer.tag {
                    crate::module::ReferenceTag::VarRead(variable_id) => {
                        let target_file_id = module.file_id;
                        let index = temp.entry(target_file_id).or_default();
                        index
                            .variable_reference
                            .entry(variable_id)
                            .or_default()
                            .push(CiterInfo::new(module.file_id, refer.range));
                    }
                    crate::module::ReferenceTag::FieldRead(field_id) => {
                        let target_file_id = field_id.module;
                        let index = temp.entry(target_file_id).or_default();
                        index
                            .field_reference
                            .entry(field_id)
                            .or_default()
                            .push(CiterInfo::new(module.file_id, refer.range));
                    }
                    crate::module::ReferenceTag::FuncCall(function_id) => {
                        let target_file_id = function_id.module;
                        let index = temp.entry(target_file_id).or_default();
                        index
                            .function_reference
                            .entry(function_id)
                            .or_default()
                            .push(CiterInfo::new(module.file_id, refer.range));
                    }
                }
            }
        }

        for (file_id, module) in &mut self.modules {
            module.index = temp.remove(file_id).unwrap_or_default();
            module.metadata = Some(Arc::clone(&self.metadata));
        }

        // checker
        for check in &mut self.checker {
            let result = check.check_project(&self.modules);
            for (file_id, errors) in result {
                if let Some(module) = self.modules.get_mut(&file_id) {
                    module.semantic_errors.extend(errors);
                }
            }
        }
    }

    /// 为模块收集符号并分配 ID
    pub fn allocate_module_symbols(module: &mut Module) {
        let root = SyntaxNode::new_root(module.green_tree.clone());
        for ele in root.children() {
            if let Some(func_def) = FuncDef::cast(ele.clone()) {
                if let Some((name, range)) = func_def
                    .sign()
                    .and_then(|n| n.name())
                    .and_then(|n| utils::extract_name_and_range(&n))
                {
                    if module.function_map.contains_key(&name) {
                        module
                            .new_error(crate::error::AnalyzeError::FunctionDefined { name, range });
                        continue;
                    }

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
                if module.struct_map.contains_key(&name) {
                    module.new_error(crate::error::AnalyzeError::StructDefined { name, range });
                    continue;
                }
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
