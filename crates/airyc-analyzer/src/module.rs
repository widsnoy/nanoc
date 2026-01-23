use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
};

use text_size::TextRange;
use thunderdome::Arena;

use crate::{
    array::{ArrayInitError, ArrayTree},
    r#type::NType,
    value::Value,
};

/// 常量类型：区分编译时常量和运行时常量
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstKind {
    /// 编译时常量：值在编译时就能确定
    CompileTime,
    /// 运行时常量：值在运行时确定但之后不可变（如指针）
    Runtime,
}

#[derive(Debug, Default)]
pub struct Module {
    pub variables: Arena<Variable>,
    pub functions: Arena<Function>,
    pub scopes: Arena<Scope>,

    pub global_scope: ScopeID,
    /// 检查节点是否为常量（编译时或运行时）
    pub constant_nodes: HashMap<TextRange, ConstKind>,

    /// 仅存储编译时常量
    pub value_table: HashMap<TextRange, Value>,

    /// 存储展开的数组
    pub expand_array: HashMap<TextRange, ArrayTree>,

    /// 变量索引：TextRange -> VariableID
    pub variable_map: HashMap<TextRange, VariableID>,

    /// 表达式类型表：TextRange -> NType
    pub type_table: HashMap<TextRange, NType>,

    /// 分析上下文，使用后清除
    pub analyzing: AnalyzeContext,
}

#[derive(Debug, Default)]
pub struct AnalyzeContext {
    pub current_scope: ScopeID,
    pub errors: Vec<SemanticError>,
    pub current_base_type: Option<NType>,
}

#[derive(Debug)]
pub enum SemanticError {
    TypeMismatch {
        expected: NType,
        found: NType,
        range: TextRange,
    },
    ConstantExprExpected {
        range: TextRange,
    },
    VariableDefined {
        name: String,
        range: TextRange,
    },
    FunctionDefined {
        name: String,
        range: TextRange,
    },
    VariableUndefined {
        name: String,
        range: TextRange,
    },
    ExpectInitialVal {
        name: String,
        range: TextRange,
    },
    ArrayError {
        message: ArrayInitError,
        range: TextRange,
    },
}

impl Module {
    /// 分析完成后清除分析上下文
    pub fn finish_analysis(&mut self) {
        self.analyzing = AnalyzeContext::default();
    }

    pub fn mark_constant(&mut self, range: TextRange, kind: ConstKind) {
        // 如果已经标记为 CompileTime，不降级为 Runtime
        if let Some(existing) = self.constant_nodes.get(&range)
            && *existing == ConstKind::CompileTime
        {
            return;
        }
        self.constant_nodes.insert(range, kind);
    }

    /// 检查是否为编译时常量
    pub fn is_compile_time_constant(&self, range: TextRange) -> bool {
        self.constant_nodes
            .get(&range)
            .is_some_and(|k| *k == ConstKind::CompileTime)
    }

    /// 检查是否为常量（编译时或运行时）
    pub fn is_constant(&self, range: TextRange) -> bool {
        self.constant_nodes.contains_key(&range)
    }

    /// 获取常量类型
    pub fn get_const_kind(&self, range: TextRange) -> Option<ConstKind> {
        self.constant_nodes.get(&range).copied()
    }

    /// 检查所有范围是否为常量，如果是则将父范围标记为常量
    pub fn check_and_mark_constant(
        &mut self,
        parent_range: TextRange,
        expr_range: Option<TextRange>,
        child_ranges: impl Iterator<Item = TextRange>,
    ) {
        let mut weakest_kind = ConstKind::CompileTime;

        if let Some(r) = expr_range {
            match self.get_const_kind(r) {
                Some(ConstKind::Runtime) => weakest_kind = ConstKind::Runtime,
                Some(ConstKind::CompileTime) => {}
                None => return,
            }
        }
        for r in child_ranges {
            match self.get_const_kind(r) {
                Some(ConstKind::Runtime) => weakest_kind = ConstKind::Runtime,
                Some(ConstKind::CompileTime) => {}
                None => return,
            }
        }
        self.mark_constant(parent_range, weakest_kind);
    }

    pub fn get_value(&self, range: TextRange) -> Option<&Value> {
        self.value_table.get(&range)
    }

    pub fn set_expr_type(&mut self, range: TextRange, ty: NType) {
        self.type_table.insert(range, ty);
    }

    pub fn get_expr_type(&self, range: TextRange) -> Option<&NType> {
        self.type_table.get(&range)
    }

    pub fn new_scope(&mut self, parent: Option<ScopeID>) -> ScopeID {
        let scope = Scope {
            parent,
            variables: HashMap::new(),
        };
        let id = self.scopes.insert(scope);
        ScopeID(id)
    }

    pub fn new_function(
        &mut self,
        name: String,
        params: Vec<VariableID>,
        ret_type: NType,
    ) -> FunctionID {
        let function = Function {
            name,
            params,
            ret_type,
        };
        let id = self.functions.insert(function);
        FunctionID(id)
    }

    pub fn get_varaible(&self, range: TextRange) -> Option<&Variable> {
        self.variable_map
            .get(&range)
            .and_then(|f| self.variables.get(**f))
    }
}

/// 定义 ID 包装类型的宏，用于 arena 索引
macro_rules! define_id_type {
    ($name:ident) => {
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
        pub struct $name(pub thunderdome::Index);

        impl $name {
            pub fn none() -> Self {
                $name(thunderdome::Index::DANGLING)
            }
        }

        impl From<thunderdome::Index> for $name {
            fn from(index: thunderdome::Index) -> Self {
                $name(index)
            }
        }

        impl Deref for $name {
            type Target = thunderdome::Index;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };
}

define_id_type!(VariableID);
define_id_type!(FunctionID);
define_id_type!(ScopeID);

impl Default for ScopeID {
    fn default() -> Self {
        Self::none()
    }
}

#[derive(Debug, Clone)]
pub struct Variable {
    pub name: String,
    pub ty: NType,
    pub range: TextRange,
    pub tag: VariableTag,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum VariableTag {
    Define,
    Write,
    Read,
}

impl Variable {
    pub fn is_const(&self) -> bool {
        self.ty.is_const()
    }
}

#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub params: Vec<VariableID>,
    pub ret_type: NType,
}

#[derive(Debug)]
pub struct Scope {
    pub parent: Option<ScopeID>,
    pub variables: HashMap<String, HashSet<VariableID>>,
}

impl Scope {
    pub fn new_variable(
        &mut self,
        variables: &mut Arena<Variable>,
        variable_map: &mut HashMap<TextRange, VariableID>,
        name: String,
        ty: NType,
        range: TextRange,
        tag: VariableTag,
    ) -> VariableID {
        let idx = variables.insert(Variable {
            name: name.clone(),
            ty,
            range,
            tag,
        });
        let var_id = VariableID(idx);
        let entry = self.variables.entry(name).or_default();
        entry.insert(var_id);
        variable_map.insert(range, var_id);
        var_id
    }

    /// 查找变量
    pub fn look_up(&self, m: &Module, var_name: &str, var_tag: VariableTag) -> Option<VariableID> {
        let mut u_opt = Some(self);
        while let Some(u) = u_opt {
            if let Some(entry) = u.variables.get(var_name)
                && let Some(idx) = entry.iter().find(|x| {
                    let var = m.variables.get(***x).unwrap();
                    var.tag == var_tag
                })
            {
                return Some(*idx);
            }
            u_opt = u.parent.map(|x| m.scopes.get(*x).unwrap());
        }
        None
    }

    /// 仅在当前作用域查找变量
    pub fn look_up_locally(
        &self,
        m: &Module,
        var_name: &str,
        var_tag: VariableTag,
    ) -> Option<VariableID> {
        if let Some(entry) = self.variables.get(var_name)
            && let Some(idx) = entry.iter().find(|x| {
                let var = m.variables.get(***x).unwrap();
                var.tag == var_tag
            })
        {
            Some(*idx)
        } else {
            None
        }
    }

    /// 检查当前作用域是否存在变量
    pub fn have_variable(&self, var_name: &str) -> bool {
        self.variables.contains_key(var_name)
    }
}
