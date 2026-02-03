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
    pub structs: Arena<Struct>,
    pub scopes: Arena<Scope>,

    pub global_scope: ScopeID,

    /// 存储编译时能计算的表达式
    pub value_table: HashMap<TextRange, Value>,

    /// 存储展开的数组
    pub expand_array: HashMap<TextRange, ArrayTree>,

    /// 变量索引：TextRange -> VariableID
    pub variable_map: HashMap<TextRange, VariableID>,

    /// Struct 索引：Name -> StructID
    pub struct_map: HashMap<String, StructID>,

    /// Function 索引
    pub function_map: HashMap<String, FunctionID>,

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
    pub current_var_type: Option<NType>,
}

impl AnalyzeContext {
    pub(crate) fn new_error(&mut self, error: SemanticError) {
        self.errors.push(error)
    }
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
        message: Box<ArrayInitError>,
        range: TextRange,
    },
    StructDefined {
        name: String,
        range: TextRange,
    },
    TypeUndefined {
        range: TextRange,
    },
    FieldNotFound {
        struct_name: String,
        field_name: String,
        range: TextRange,
    },
    NotAStruct {
        ty: NType,
        range: TextRange,
    },
    NotAStructPointer {
        ty: NType,
        range: TextRange,
    },
    /// Struct 初始化列表字段数量不匹配
    StructInitFieldCountMismatch {
        expected: usize,
        found: usize,
        range: TextRange,
    },
    /// 不能对 type 应用某种 op
    CantApplyOpOnType {
        ty: NType,
        op: &'static str,
    },
}

impl Module {
    /// 分析完成后清除分析上下文
    pub fn finish_analysis(&mut self) {
        self.analyzing = AnalyzeContext::default();
    }

    /// 检查是否为编译时常量
    pub fn is_compile_time_constant(&self, range: TextRange) -> bool {
        self.value_table.contains_key(&range)
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

    /// 获取 struct 定义
    pub fn get_struct(&self, id: StructID) -> Option<&Struct> {
        self.structs.get(*id)
    }

    /// 获取可变 struct 定义
    pub fn get_struct_mut(&mut self, id: StructID) -> Option<&mut Struct> {
        self.structs.get_mut(*id)
    }

    /// 根据名称查找 struct
    pub fn find_struct(&self, name: &str) -> Option<StructID> {
        self.struct_map.get(name).copied()
    }

    /// 根据名称查找函数
    pub fn find_function(&self, name: &str) -> Option<FunctionID> {
        self.function_map.get(name).copied()
    }

    /// 获取函数定义
    pub fn get_function(&self, id: FunctionID) -> Option<&Function> {
        self.functions.get(*id)
    }

    /// 添加新的 struct 定义
    pub fn new_struct(
        &mut self,
        name: String,
        fields: Vec<StructField>,
        range: TextRange,
    ) -> StructID {
        let struct_def = Struct {
            name,
            fields,
            range,
        };
        let id = self.structs.insert(struct_def);
        StructID(id)
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
define_id_type!(StructID);

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

#[derive(Debug, Clone)]
pub struct Struct {
    pub name: String,
    pub fields: Vec<StructField>,
    pub range: TextRange,
}

impl Struct {
    /// 根据字段名查找字段索引
    pub fn field_index(&self, name: &str) -> Option<u32> {
        self.fields
            .iter()
            .position(|f| f.name == name)
            .map(|i| i as u32)
    }

    /// 根据字段名查找字段
    pub fn field(&self, name: &str) -> Option<&StructField> {
        self.fields.iter().find(|f| f.name == name)
    }
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub ty: NType,
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

    /// 检查当前作用域是否存在变量
    pub fn have_variable(&self, var_name: &str) -> bool {
        self.variables.contains_key(var_name)
    }
}
