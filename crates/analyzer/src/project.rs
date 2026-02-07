use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use rowan::GreenNode;
use syntax::{AstNode as _, SyntaxNode, ast::Header};
use thunderdome::Arena;
use vfs::{FileID, Vfs};

use crate::{
    error::SemanticError,
    module::{FunctionID, Module, ModuleID, StructID},
};

#[derive(Debug, Default)]
pub struct Project {
    pub modules: Arena<Module>,
    pub vfs: Vfs,
    pub file_index: HashMap<FileID, ModuleID>,
    /// a->b, 代表 a 模块依赖 b
    pub reference_map: HashMap<ModuleID, HashSet<ModuleID>>,
    /// 记录被哪些模块依赖
    pub reference_map_rev: HashMap<ModuleID, HashSet<ModuleID>>,
}

impl Project {
    pub fn new(vfs: Vfs, green_trees: HashMap<FileID, GreenNode>) -> Self {
        let mut project = Project {
            vfs,
            ..Default::default()
        };
        for (file_id, green_tree) in green_trees {
            let id = project.modules.insert(Module::new(green_tree));
            let module = project.modules.get_mut(id).unwrap();
            let module_id = ModuleID(id);
            module.module_id = module_id;
            project.file_index.insert(file_id, module_id);
        }

        // 初始化 Module 并分析头文件
        for (module_id, module) in project.modules.iter_mut() {
            let file_id = project
                .file_index
                .iter()
                .find(|(_, mid)| **mid == ModuleID(module_id))
                .map(|(fid, _)| *fid)
                .unwrap();

            Self::first_analyze_header(
                &project.vfs,
                &project.file_index,
                module,
                ModuleID(module_id),
                file_id,
                &mut project.reference_map,
                &mut project.reference_map_rev,
            );
        }

        // 拓扑排序依赖图，并按顺序分析模块
        let mut sort = Vec::with_capacity(project.modules.len());
        let mut deg = Vec::new();
        let mut stack = vec![];

        // 初始化入度数组
        for (_index, _) in project.modules.iter() {
            deg.push(0);
        }

        // 计算每个模块的入度
        for (index, _) in project.modules.iter() {
            let module_id = ModuleID(index);
            let i = index.slot() as usize;
            if let Some(s) = project.reference_map.get(&module_id) {
                deg[i] = s.len();
            } else {
                stack.push(module_id);
            }
        }

        while let Some(module_id) = stack.pop() {
            sort.push(module_id);
            let Some(list) = project.reference_map_rev.get(&module_id) else {
                continue;
            };
            for v in list.iter() {
                let i = v.slot() as usize;
                deg[i] -= 1;
                if deg[i] == 0 {
                    stack.push(*v);
                }
            }
        }

        // 检测循环依赖
        if sort.len() != project.modules.len() {
            // 找出所有在环中的模块（入度不为 0 的模块）
            let modules_in_cycle: Vec<_> = project
                .modules
                .iter()
                .filter_map(|(index, _)| {
                    let i = index.slot() as usize;
                    if deg[i] > 0 { Some(index) } else { None }
                })
                .collect();

            // 为每个在环中的模块添加错误
            for index in modules_in_cycle {
                if let Some(module) = project.modules.get_mut(index) {
                    // 获取整个模块的 range（使用根节点的 range）
                    let root = SyntaxNode::new_root(module.green_tree.clone());
                    let range = root.text_range().into();
                    module.new_error(SemanticError::CircularDependency { range });
                }
            }
        }

        // 即使有循环依赖，也继续处理已排序的模块
        for module_id in sort {
            let Some(module) = project.modules.get_mut(*module_id) else {
                continue;
            };
            module.analyze();

            if let Some(dependent_modules) = project.reference_map_rev.get(&module_id) {
                for dependent_module_id in dependent_modules {
                    if let (Some(dependent_module), Some(current_module)) =
                        project.modules.get2_mut(**dependent_module_id, *module_id)
                    {
                        current_module.functions.iter().for_each(|(_, f)| {
                            let func_id = dependent_module.functions.insert(f.clone());
                            dependent_module
                                .function_map
                                .insert(f.name.clone(), FunctionID(func_id));
                        });

                        current_module.structs.iter().for_each(|(_, s)| {
                            let struct_id = dependent_module.structs.insert(s.clone());
                            dependent_module
                                .struct_map
                                .insert(s.name.clone(), StructID(struct_id));
                        });
                    }
                }
            }
        }

        project
    }

    /// 初次分析头文件，引用关系存到引用表
    pub fn first_analyze_header(
        vfs: &Vfs,
        file_index: &HashMap<FileID, ModuleID>,
        module: &mut Module,
        module_id: ModuleID,
        current_file_id: FileID,
        ref_map: &mut HashMap<ModuleID, HashSet<ModuleID>>,
        ref_map_rev: &mut HashMap<ModuleID, HashSet<ModuleID>>,
    ) {
        // 获取当前文件路径
        let current_file = vfs.get_file_by_file_id(&current_file_id).unwrap();
        let current_dir = current_file.path.parent().unwrap();

        let root = SyntaxNode::new_root(module.green_tree.clone());
        root.children().flat_map(Header::cast).for_each(|n| {
            if let Some(p) = n.path()
                && let Some(import_path_str) = p.ident().map(|i| i.to_string())
            {
                let mut import_path = PathBuf::from(&import_path_str);

                // 自动添加 .airy 后缀
                if import_path.extension().is_none() {
                    import_path = import_path.with_extension("airy");
                }

                // 相对于当前文件解析
                let resolved = current_dir.join(&import_path);

                // 规范化路径
                if let Ok(canonical) = resolved.canonicalize()
                    && let Some(file_id) = vfs.get_file_id_by_path(&canonical)
                    && let Some(other_id) = file_index.get(&file_id)
                {
                    ref_map.entry(module_id).or_default().insert(*other_id);
                    ref_map_rev.entry(*other_id).or_default().insert(module_id);
                } else {
                    module.new_error(SemanticError::InvalidPath {
                        range: utils::trim_node_text_range(&p),
                    });
                }
            }
        });
    }
}
