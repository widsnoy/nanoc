use std::collections::{HashMap, HashSet};

use crate::{checker::ProjectChecker, error::AnalyzeError, module::StructID, r#type::Ty};

#[derive(Debug, Default)]
pub struct RecursiveTypeChecker {
    pub edge: HashMap<StructID, Vec<StructID>>,

    dfn: HashMap<StructID, usize>,
    low: HashMap<StructID, usize>,
    in_stack: HashSet<StructID>,
    stack: Vec<StructID>,
    timestamp: usize,
}

impl ProjectChecker for RecursiveTypeChecker {
    fn check_project(
        &mut self,
        modules: &HashMap<vfs::FileID, crate::module::Module>,
    ) -> HashMap<vfs::FileID, Vec<AnalyzeError>> {
        // 构建图
        self.build_graph(modules);

        // 使用 Tarjan 算法找出所有强连通分量
        let sccs = self.tarjan();

        // 生成错误信息
        self.generate_errors(sccs, modules)
    }
}

impl RecursiveTypeChecker {
    fn build_graph(&mut self, modules: &HashMap<vfs::FileID, crate::module::Module>) {
        self.edge.clear();

        for (file_id, module) in modules {
            for (sc_id, sc) in module.structs.iter() {
                let from = StructID::new(*file_id, sc_id);
                let mut targets = Vec::new();

                for field_id in &sc.fields {
                    let Some(field) = module.get_field_by_id(*field_id) else {
                        continue;
                    };

                    let mut ty = &field.ty;
                    let to = loop {
                        match ty {
                            Ty::Const(inner) => ty = inner,
                            Ty::Pointer { .. } => break None,
                            Ty::Struct { id, .. } => break Some(id),
                            Ty::Array(inner, _) => ty = inner,
                            _ => break None,
                        }
                    };

                    if let Some(to) = to {
                        targets.push(*to);
                    }
                }

                if !targets.is_empty() {
                    self.edge.insert(from, targets);
                }
            }
        }
    }

    fn tarjan(&mut self) -> Vec<Vec<StructID>> {
        self.dfn.clear();
        self.low.clear();
        self.in_stack.clear();
        self.stack.clear();
        self.timestamp = 0;

        let mut sccs = Vec::new();
        let mut self_loop = HashSet::new();

        let nodes: Vec<StructID> = self.edge.keys().copied().collect();

        for node in nodes {
            if !self.dfn.contains_key(&node) {
                self.dfs(node, &mut sccs, &mut self_loop);
            }
        }

        for u in self_loop {
            sccs.push(vec![u]);
        }

        sccs
    }

    fn dfs(
        &mut self,
        u: StructID,
        sccs: &mut Vec<Vec<StructID>>,
        self_loop: &mut HashSet<StructID>,
    ) {
        self.timestamp += 1;
        self.dfn.insert(u, self.timestamp);
        self.low.insert(u, self.timestamp);

        self.stack.push(u);
        self.in_stack.insert(u);

        if let Some(neighbors) = self.edge.get(&u).cloned() {
            for v in neighbors {
                if u == v {
                    self_loop.insert(u);
                }
                if !self.dfn.contains_key(&v) {
                    // 未访问过，递归访问
                    self.dfs(v, sccs, self_loop);
                    // 更新 low[u]
                    let low_v = *self.low.get(&v).unwrap();
                    let low_u = self.low.get_mut(&u).unwrap();
                    *low_u = (*low_u).min(low_v);
                } else if self.in_stack.contains(&v) {
                    // 在栈中，说明是回边，更新 low[u]
                    let dfn_v = *self.dfn.get(&v).unwrap();
                    let low_u = self.low.get_mut(&u).unwrap();
                    *low_u = (*low_u).min(dfn_v);
                }
            }
        }

        if self.dfn.get(&u) == self.low.get(&u) {
            let mut scc = Vec::new();

            loop {
                let v = self.stack.pop().unwrap();
                self.in_stack.remove(&v);
                scc.push(v);

                if v == u {
                    break;
                }
            }

            if scc.len() > 1 {
                sccs.push(scc);
            }
        }
    }

    fn generate_errors(
        &self,
        sccs: Vec<Vec<StructID>>,
        modules: &HashMap<vfs::FileID, crate::module::Module>,
    ) -> HashMap<vfs::FileID, Vec<AnalyzeError>> {
        let mut errors: HashMap<vfs::FileID, Vec<AnalyzeError>> = HashMap::new();

        for scc in sccs {
            // 构建环的路径（struct 名字列表）
            let mut cycle_names = Vec::new();
            for struct_id in &scc {
                if let Some(module) = modules.get(&struct_id.module)
                    && let Some(struct_def) = module.structs.get(struct_id.index)
                {
                    cycle_names.push(struct_def.name.clone());
                }
            }

            // 为了让环路径更清晰，添加第一个元素到末尾形成闭环
            if let Some(first) = cycle_names.first() {
                cycle_names.push(first.clone());
            }

            // 为环中的每个 struct 生成一个错误
            for struct_id in &scc {
                if let Some(module) = modules.get(&struct_id.module)
                    && let Some(struct_def) = module.structs.get(struct_id.index)
                {
                    let error = AnalyzeError::RecursiveType {
                        struct_name: struct_def.name.clone(),
                        cycle: cycle_names.clone(),
                        range: struct_def.range,
                    };

                    errors.entry(struct_id.module).or_default().push(error);
                }
            }
        }

        errors
    }
}
