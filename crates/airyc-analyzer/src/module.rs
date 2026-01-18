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

#[derive(Debug, Default)]
pub struct Module {
    pub variables: Arena<Variable>,
    pub functions: Arena<Function>,
    pub scopes: Arena<Scope>,

    pub global_scope: ScopeID,
    /// Check if node is compile-time constant
    pub constant_nodes: HashSet<TextRange>,

    /// Store constants only
    pub value_table: HashMap<TextRange, Value>,

    /// Store expanded arrays
    pub expand_array: HashMap<TextRange, ArrayTree>,

    /// Variable index: TextRange -> VariableID
    pub variable_map: HashMap<TextRange, VariableID>,

    /// Analysis context, cleared after use
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
    pub fn mark_constant(&mut self, range: TextRange) {
        self.constant_nodes.insert(range);
    }

    pub fn is_constant(&self, range: TextRange) -> bool {
        self.constant_nodes.contains(&range)
    }

    /// Check if all ranges are constant, and if so, mark the parent range as constant
    pub fn check_and_mark_constant(
        &mut self,
        parent_range: TextRange,
        expr_range: Option<TextRange>,
        child_ranges: impl Iterator<Item = TextRange>,
    ) {
        if let Some(r) = expr_range
            && !self.is_constant(r)
        {
            return;
        }
        for r in child_ranges {
            if !self.is_constant(r) {
                return;
            }
        }
        self.mark_constant(parent_range);
    }

    pub fn get_value(&self, range: TextRange) -> Option<&Value> {
        self.value_table.get(&range)
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

/// Macro to define ID wrapper types for arena indices
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

    /// Lookup variable
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

    /// Lookup variable in current scope only
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

    /// Check if variable exists in current scope
    pub fn have_variable(&self, var_name: &str) -> bool {
        self.variables.contains_key(var_name)
    }
}
