use std::{collections::HashSet, ops::Deref};

use text_size::{TextRange, TextSize};
use thunderdome::Arena;

use crate::ntype::NType;

#[derive(Default)]
pub struct Module {
    pub variables: Arena<Variable>, // 所有 scope 的都存在这里
    pub functions: Arena<Function>,
    pub scopes: Arena<Scope>,

    pub global_scope: ScopeID,
    // 检查是否是编译期可计算的常量节点
    pub constant_nodes: HashSet<TextRange>,

    // 分析的时候上下文，使用后清除
    pub(super) analyzing: AnalyzeContext,
}

#[derive(Default)]
pub(super) struct AnalyzeContext {
    pub(super) current_scope: ScopeID,
    pub(super) errors: Vec<SemanticError>,
}

pub enum SemanticError {
    TypeMismatch {
        expected: NType,
        found: NType,
        range: TextRange,
    },
    ConstantExprExpected {
        range: TextRange,
    },
}

impl Module {
    pub fn mark_constant(&mut self, range: TextRange) {
        self.constant_nodes.insert(range);
    }

    pub fn is_constant(&self, range: TextRange) -> bool {
        self.constant_nodes.contains(&range)
    }

    pub fn new_scope(&mut self, parent: Option<ScopeID>) -> ScopeID {
        let scope = Scope {
            parent,
            variables: Vec::new(),
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
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct VariableID(pub thunderdome::Index);
impl VariableID {
    pub fn none() -> Self {
        VariableID(thunderdome::Index::DANGLING)
    }
}
impl From<thunderdome::Index> for VariableID {
    fn from(index: thunderdome::Index) -> Self {
        VariableID(index)
    }
}
impl Deref for VariableID {
    type Target = thunderdome::Index;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct Variable {
    pub name: String,
    pub ty: NType,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct FunctionID(pub thunderdome::Index);
impl FunctionID {
    pub fn none() -> Self {
        FunctionID(thunderdome::Index::DANGLING)
    }
}
impl From<thunderdome::Index> for FunctionID {
    fn from(index: thunderdome::Index) -> Self {
        FunctionID(index)
    }
}
impl Deref for FunctionID {
    type Target = thunderdome::Index;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct Function {
    pub name: String,
    pub params: Vec<VariableID>,
    pub ret_type: NType,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct ScopeID(pub thunderdome::Index);

impl ScopeID {
    pub fn none() -> Self {
        ScopeID(thunderdome::Index::DANGLING)
    }
}

impl Default for ScopeID {
    fn default() -> Self {
        Self::none()
    }
}

impl From<thunderdome::Index> for ScopeID {
    fn from(index: thunderdome::Index) -> Self {
        ScopeID(index)
    }
}

impl Deref for ScopeID {
    type Target = thunderdome::Index;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct Scope {
    pub parent: Option<ScopeID>,
    pub variables: Vec<VariableID>,
}

impl Scope {
    pub fn new_variable(
        &mut self,
        variables: &mut Arena<Variable>,
        name: String,
        ty: NType,
    ) -> VariableID {
        let idx = variables.insert(Variable { name, ty });
        let var_id = VariableID(idx);
        self.variables.push(var_id);
        var_id
    }
}
