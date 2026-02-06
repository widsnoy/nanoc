use std::{
    collections::{BTreeMap, HashMap},
    ops::Deref,
};

use parser::visitor::Visitor;
use rowan::GreenNode;
use syntax::SyntaxNode;
use thunderdome::Arena;
use tools::TextRange;

use crate::{array::ArrayTree, error::SemanticError, r#type::NType, value::Value};

#[derive(Debug)]
pub struct Module {
    pub variables: Arena<Variable>,
    pub reference: Arena<Reference>,
    pub functions: Arena<Function>,
    pub structs: Arena<Struct>,
    pub scopes: Arena<Scope>,

    pub global_scope: ScopeID,

    pub green_tree: GreenNode,

    /// 存储编译时能计算的表达式
    pub value_table: HashMap<TextRange, Value>,

    /// 存储展开的数组
    pub expand_array: HashMap<TextRange, ArrayTree>,

    /// 变量索引：TextRange -> VariableID
    pub variable_map: BTreeMap<TextRange, VariableID>,

    /// 引用索引：TextRange -> ReferenceID
    pub reference_map: BTreeMap<TextRange, ReferenceID>,

    /// Struct 索引：Name -> StructID
    pub struct_map: HashMap<String, StructID>,

    /// Function 索引
    pub function_map: HashMap<String, FunctionID>,

    /// 表达式类型表：TextRange -> NType
    pub type_table: HashMap<TextRange, NType>,

    /// 错误
    pub semantic_errors: Vec<SemanticError>,

    /// 分析上下文，使用后清除
    pub(crate) analyzing: AnalyzeContext,
}

#[derive(Debug, Default)]
pub struct AnalyzeContext {
    pub current_scope: ScopeID,
    pub current_var_type: Option<NType>,
    pub current_function_ret_type: Option<NType>,
    pub loop_depth: usize,
}

impl Module {
    pub fn new(green_tree: GreenNode) -> Self {
        Self {
            green_tree,
            variables: Default::default(),
            reference: Default::default(),
            functions: Default::default(),
            structs: Default::default(),
            scopes: Default::default(),
            global_scope: Default::default(),
            value_table: Default::default(),
            expand_array: Default::default(),
            variable_map: Default::default(),
            reference_map: Default::default(),
            struct_map: Default::default(),
            function_map: Default::default(),
            type_table: Default::default(),
            semantic_errors: Default::default(),
            analyzing: Default::default(),
        }
    }
    /// 分析
    pub fn analyze(&mut self) {
        let root = SyntaxNode::new_root(self.green_tree.clone());
        self.walk(&root);
        self.analyzing = AnalyzeContext::default();
    }

    /// 检查是否为编译时常量
    pub fn is_compile_time_constant(&self, range: TextRange) -> bool {
        self.value_table.contains_key(&range)
    }

    pub fn get_value_by_range(&self, range: TextRange) -> Option<&Value> {
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
        range: TextRange,
    ) -> FunctionID {
        let function = Function {
            name,
            params,
            ret_type,
            range,
        };
        let id = self.functions.insert(function);
        FunctionID(id)
    }

    pub fn get_varaible_by_id(&self, var_id: VariableID) -> Option<&Variable> {
        self.variables.get(*var_id)
    }

    pub fn get_varaible_by_range(&self, range: TextRange) -> Option<&Variable> {
        self.variable_map
            .get(&range)
            .and_then(|f| self.variables.get(**f))
    }

    pub fn get_reference_by_id(&self, ref_id: ReferenceID) -> Option<&Reference> {
        self.reference.get(*ref_id)
    }

    pub fn get_reference_by_range(&self, range: TextRange) -> Option<&Reference> {
        self.reference_map
            .get(&range)
            .and_then(|f| self.reference.get(**f))
    }

    /// 获取 struct 定义
    pub fn get_struct_by_id(&self, id: StructID) -> Option<&Struct> {
        self.structs.get(*id)
    }

    /// 获取可变 struct 定义
    pub fn get_struct_mut_by_id(&mut self, id: StructID) -> Option<&mut Struct> {
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
    pub fn get_function_by_id(&self, id: FunctionID) -> Option<&Function> {
        self.functions.get(*id)
    }

    /// 获取函数定义
    pub fn get_function_mut_by_id(&mut self, id: FunctionID) -> Option<&mut Function> {
        self.functions.get_mut(*id)
    }

    /// 添加新的 struct 定义
    pub fn new_struct(
        &mut self,
        name: String,
        fields: Vec<VariableID>,
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

    /// 记录引用
    pub fn new_reference(&mut self, range: TextRange, tag: ReferenceTag) {
        let ref_var = Reference { range, tag };
        let ref_idx = self.reference.insert(ref_var);
        let ref_id = ReferenceID(ref_idx);

        self.reference_map.insert(range, ref_id);
    }

    /// 查找变量定义，返回定义处的 VariableID
    pub fn find_variable_def(&self, var_name: &str) -> Option<VariableID> {
        let scope = self.scopes.get(*self.analyzing.current_scope)?;
        scope.look_up_variable(self, var_name)
    }

    pub(crate) fn new_error(&mut self, error: SemanticError) {
        self.semantic_errors.push(error)
    }

    pub fn get_green_tree(&self) -> GreenNode {
        self.green_tree.clone()
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
define_id_type!(ReferenceID);

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
}

impl Variable {
    pub fn is_const(&self) -> bool {
        self.ty.is_const()
    }
}

#[derive(Debug, Clone)]
pub struct Reference {
    pub tag: ReferenceTag,
    pub range: TextRange,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceTag {
    // TODO: VarWrite,
    VarRead(VariableID),
    FuncCall(FunctionID),
}
#[derive(Debug)]
pub struct Function {
    pub name: String,
    pub params: Vec<VariableID>,
    pub ret_type: NType,
    pub range: TextRange,
}

#[derive(Debug, Clone)]
pub struct Struct {
    pub name: String,
    pub fields: Vec<VariableID>,
    pub range: TextRange,
}

impl Struct {
    /// 根据字段名查找字段索引
    pub fn field_index(&self, module: &Module, name: &str) -> Option<u32> {
        self.fields
            .iter()
            .position(|field_id| {
                module
                    .variables
                    .get(**field_id)
                    .map(|var| var.name == name)
                    .unwrap_or(false)
            })
            .map(|i| i as u32)
    }

    /// 根据字段名查找字段 ID
    pub fn field(&self, module: &Module, name: &str) -> Option<VariableID> {
        self.fields
            .iter()
            .find(|field_id| {
                module
                    .variables
                    .get(***field_id)
                    .map(|var| var.name == name)
                    .unwrap_or(false)
            })
            .copied()
    }

    /// 根据索引获取字段 ID
    pub fn field_at(&self, index: usize) -> Option<VariableID> {
        self.fields.get(index).copied()
    }
}

#[derive(Debug)]
pub struct Scope {
    pub parent: Option<ScopeID>,
    pub variables: HashMap<String, VariableID>,
}

impl Scope {
    pub fn new_variable(
        &mut self,
        variables: &mut Arena<Variable>,
        variable_map: &mut BTreeMap<TextRange, VariableID>,
        name: String,
        ty: NType,
        range: TextRange,
    ) -> VariableID {
        let idx = variables.insert(Variable {
            name: name.clone(),
            ty,
            range,
        });
        let var_id = VariableID(idx);
        self.variables.insert(name, var_id);
        variable_map.insert(range, var_id);
        var_id
    }

    /// 查找变量
    pub fn look_up_variable(&self, m: &Module, var_name: &str) -> Option<VariableID> {
        let mut u_opt = Some(self);
        while let Some(u) = u_opt {
            if let Some(idx) = u.variables.get(var_name) {
                return Some(*idx);
            }
            u_opt = u.parent.map(|x| m.scopes.get(*x).unwrap());
        }
        None
    }

    /// 检查当前作用域是否存在变量定义
    pub fn have_variable_def(&self, var_name: &str) -> bool {
        self.variables.contains_key(var_name)
    }
}
