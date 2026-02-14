use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use parser::parse::Parser;
use rayon::prelude::*;
use syntax::{
    AstNode as _, SyntaxNode,
    ast::{FuncDef, StructDef},
};
use vfs::{FileID, Vfs};

use crate::{
    checker::ProjectChecker,
    header::HeaderAnalyzer,
    module::{CiterInfo, Field, FieldID, Module, ModuleIndex, ThinModule},
    r#type::Ty,
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
        self.modules.clear();
        self.metadata = Default::default();

        // 初始化所有 module，语法分析
        let file_ids = vfs.file_ids();
        let modules = RwLock::new(HashMap::new());

        file_ids.par_iter().for_each(|&file_id| {
            if let Some(file) = vfs.get_file_by_file_id(&file_id) {
                let parser = Parser::new(&file.text);
                let (green_tree, errors) = parser.parse();

                let mut module = Module::new(green_tree.clone());
                module.file_id = file_id;
                errors.into_iter().for_each(|e| {
                    module
                        .semantic_errors
                        .push(crate::error::AnalyzeError::ParserError(Box::new(e)))
                });

                // 收集符号并分配 ID
                Self::allocate_module_symbols(&mut module);

                modules.write().unwrap().insert(file_id, module);
            }
        });

        self.modules = modules.into_inner().unwrap();

        // 分析头文件
        let all_imports: Vec<_> = self
            .modules
            .par_iter()
            .map(|(file_id, module)| {
                let module_imports =
                    HeaderAnalyzer::collect_module_imports(module, *file_id, vfs, &self.modules);
                (*file_id, module_imports)
            })
            .collect();
        for (file_id, module_imports) in all_imports {
            if let Some(module) = self.modules.get_mut(&file_id) {
                HeaderAnalyzer::apply_module_imports(module, module_imports);
            }
        }

        // 预处理元数据，跨文件使用
        self.modules.par_iter_mut().for_each(|(_, module)| {
            Self::fill_definitions(module);
        });

        let metadata: HashMap<FileID, ThinModule> = self
            .modules
            .par_iter()
            .map(|(file_id, module)| (*file_id, ThinModule::new(module)))
            .collect();

        // 语义分析
        let metadata_rc = Arc::new(metadata);
        self.modules.par_iter_mut().for_each(|(_, module)| {
            module.metadata = Some(Arc::clone(&metadata_rc));
            module.analyze();
            module.metadata = None;
        });

        // 重新拷贝分析完成的元数据
        let metadata: HashMap<FileID, ThinModule> = self
            .modules
            .par_iter()
            .map(|(file_id, module)| (*file_id, ThinModule::new(module)))
            .collect();
        self.metadata = Arc::new(metadata);

        // 构建索引（并行收集 + 串行合并）
        let local_indices: Vec<_> = self
            .modules
            .par_iter()
            .map(|(_file_id, module)| {
                let mut local_temp: HashMap<FileID, ModuleIndex> = HashMap::new();

                for (_, refer) in &module.reference {
                    match refer.tag {
                        crate::module::ReferenceTag::VarRead(variable_id) => {
                            let target_file_id = module.file_id;
                            let index = local_temp.entry(target_file_id).or_default();
                            index
                                .variable_reference
                                .entry(variable_id)
                                .or_default()
                                .push(CiterInfo::new(module.file_id, refer.range));
                        }
                        crate::module::ReferenceTag::FieldRead(field_id) => {
                            let target_file_id = field_id.module;
                            let index = local_temp.entry(target_file_id).or_default();
                            index
                                .field_reference
                                .entry(field_id)
                                .or_default()
                                .push(CiterInfo::new(module.file_id, refer.range));
                        }
                        crate::module::ReferenceTag::FuncCall(function_id) => {
                            let target_file_id = function_id.module;
                            let index = local_temp.entry(target_file_id).or_default();
                            index
                                .function_reference
                                .entry(function_id)
                                .or_default()
                                .push(CiterInfo::new(module.file_id, refer.range));
                        }
                    }
                }

                local_temp
            })
            .collect();

        // 串行合并所有本地索引
        let mut temp: HashMap<FileID, ModuleIndex> = HashMap::new();
        for local_temp in local_indices {
            for (file_id, local_index) in local_temp {
                let index = temp.entry(file_id).or_default();

                // 合并 variable_reference
                for (var_id, citers) in local_index.variable_reference {
                    index
                        .variable_reference
                        .entry(var_id)
                        .or_default()
                        .extend(citers);
                }

                // 合并 field_reference
                for (field_id, citers) in local_index.field_reference {
                    index
                        .field_reference
                        .entry(field_id)
                        .or_default()
                        .extend(citers);
                }

                // 合并 function_reference
                for (func_id, citers) in local_index.function_reference {
                    index
                        .function_reference
                        .entry(func_id)
                        .or_default()
                        .extend(citers);
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
                        Ty::Void,
                        false,
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

                // 提取参数类型信息和可变参数标志
                let mut meta_types = Vec::new();
                let mut is_variadic = false;

                if let Some(params_node) = sign.params() {
                    for param in params_node.params() {
                        // 检查是否为可变参数
                        if param.is_variadic() {
                            is_variadic = true;
                            break;
                        }

                        if let Some(param_name) = param.name().and_then(|n| n.var_name())
                            && let Some(ty_node) = param.ty()
                        {
                            match crate::utils::parse_type_node(module, &ty_node, None) {
                                Ok(Some(ty)) => {
                                    meta_types.push((param_name, ty));
                                }
                                Ok(None) => continue,
                                Err(e) => {
                                    module.semantic_errors.push(e);
                                    continue;
                                }
                            }
                        }
                    }
                }

                // 提取返回类型
                let ret_type = if let Some(ty_node) = sign.ret_type() {
                    match crate::utils::parse_type_node(module, &ty_node, None) {
                        Ok(Some(ty)) => ty,
                        Ok(None) => {
                            continue;
                        }
                        Err(e) => {
                            module.semantic_errors.push(e);
                            Ty::Void
                        }
                    }
                } else {
                    Ty::Void
                };

                // 更新函数定义
                if let Some(func_data) = module.get_function_mut_by_id(func_id) {
                    func_data.meta_types = meta_types;
                    func_data.ret_type = ret_type;
                    func_data.is_variadic = is_variadic;
                }
            }
        }
    }
}
