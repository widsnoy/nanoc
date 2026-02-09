use std::{
    collections::{BTreeMap, HashMap},
    ops::Deref,
    sync::Arc,
};

use dashmap::DashMap;
use rowan::GreenNode;
use syntax::SyntaxNode;
use syntax::Visitor;
use thunderdome::Arena;
use tools::TextRange;
use vfs::FileID;

use crate::{array::ArrayTree, error::SemanticError, r#type::NType, value::Value};

#[derive(Debug)]
pub struct Module {
    pub file_id: FileID,

    pub variables: Arena<Variable>,
    pub reference: Arena<Reference>,
    pub functions: Arena<Function>,
    pub structs: Arena<Struct>,
    pub fields: Arena<Field>,
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

    /// 各种索引
    pub index: ModuleIndex,

    /// 用于跨文件分析
    pub metadata: Option<Arc<DashMap<FileID, ThinModule>>>,
}

#[derive(Debug, Default)]
pub struct ThinModule {
    pub functions: Arena<Function>,
    pub structs: Arena<Struct>,
    pub fields: Arena<Field>,
}

impl ThinModule {
    pub fn new(module: &Module) -> Self {
        Self {
            functions: module.functions.clone(),
            structs: module.structs.clone(),
            fields: module.fields.clone(),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct AnalyzeContext {
    pub(crate) current_scope: ScopeID,
    pub(crate) current_function_ret_type: Option<NType>,
    pub(crate) loop_depth: usize,
}

#[derive(Debug, Default)]
pub struct ModuleIndex {
    pub variable_reference: HashMap<VariableID, Vec<ReferenceID>>,
    pub function_reference: HashMap<FunctionID, Vec<ReferenceID>>,
    pub scope_tree: HashMap<ScopeID, Vec<ScopeID>>,
}

impl Module {
    pub fn new(green_tree: GreenNode) -> Self {
        Self {
            green_tree,
            file_id: FileID::none(),
            variables: Default::default(),
            reference: Default::default(),
            functions: Default::default(),
            structs: Default::default(),
            fields: Default::default(),
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
            index: Default::default(),
            metadata: None,
        }
    }
    /// 分析
    pub fn analyze(&mut self) {
        let root = SyntaxNode::new_root(self.green_tree.clone());
        self.walk(&root);
        self.analyzing = AnalyzeContext::default();
        self.metadata = None;
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

    pub fn new_scope(&mut self, parent: Option<ScopeID>, range: TextRange) -> ScopeID {
        let scope = Scope {
            parent,
            variables: HashMap::new(),
            range,
        };
        let id = ScopeID(self.scopes.insert(scope));

        if let Some(pid) = parent {
            self.index.scope_tree.entry(pid).or_default().push(id);
        }

        id
    }

    pub fn new_function(
        &mut self,
        name: String,
        params: Vec<VariableID>,
        param_types: Vec<NType>,
        ret_type: NType,
        have_impl: bool,
        range: TextRange,
    ) -> FunctionID {
        let function = Function {
            name,
            params,
            param_types,
            ret_type,
            have_impl,
            range,
        };
        let id = self.functions.insert(function);
        FunctionID::new(self.file_id, id)
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
    /// TODO: 看看能不能优化
    pub fn get_struct_by_id(&self, id: StructID) -> Option<Struct> {
        if id.module == self.file_id {
            self.structs.get(id.index).cloned()
        } else {
            self.metadata
                .as_ref()?
                .get(&id.module)?
                .structs
                .get(id.index)
                .cloned()
        }
    }

    /// 获取可变 struct 定义
    /// 注意：只能获取本地模块的结构体
    pub fn get_struct_mut_by_id(&mut self, id: StructID) -> Option<&mut Struct> {
        debug_assert_eq!(
            id.module, self.file_id,
            "Cannot get mutable reference to struct in another module"
        );
        self.structs.get_mut(id.index)
    }

    /// 根据名称查找 struct
    pub fn get_struct_id_by_name(&self, name: &str) -> Option<StructID> {
        self.struct_map.get(name).copied()
    }

    /// 根据名称查找函数
    pub fn get_function_id_by_name(&self, name: &str) -> Option<FunctionID> {
        self.function_map.get(name).copied()
    }

    /// 获取函数定义
    /// TODO: 看看能不能优化
    pub fn get_function_by_id(&self, id: FunctionID) -> Option<Function> {
        if id.module == self.file_id {
            self.functions.get(id.index).cloned()
        } else {
            self.metadata
                .as_ref()?
                .get(&id.module)?
                .functions
                .get(id.index)
                .cloned()
        }
    }

    /// 获取函数定义（可变引用）
    /// 注意：只能获取本地模块的函数
    pub fn get_function_mut_by_id(&mut self, id: FunctionID) -> Option<&mut Function> {
        debug_assert_eq!(
            id.module, self.file_id,
            "Cannot get mutable reference to function in another module"
        );
        self.functions.get_mut(id.index)
    }

    /// 添加新的 struct 定义
    pub fn new_struct(&mut self, name: String, fields: Vec<FieldID>, range: TextRange) -> StructID {
        let struct_def = Struct {
            name,
            fields,
            range,
        };
        let id = self.structs.insert(struct_def);
        StructID::new(self.file_id, id)
    }

    pub fn new_field(&mut self, name: String, ty: NType, range: TextRange) -> FieldID {
        let field = Field { name, ty, range };
        let id = self.fields.insert(field);
        FieldID::new(self.file_id, id)
    }

    /// 获取字段定义（支持跨模块访问）
    pub fn get_field_by_id(&self, id: FieldID) -> Option<Field> {
        if id.module == self.file_id {
            self.fields.get(id.index).cloned()
        } else {
            self.metadata
                .as_ref()?
                .get(&id.module)?
                .fields
                .get(id.index)
                .cloned()
        }
    }

    /// 获取变量定义
    /// 注意：变量通常是局部的，不支持跨模块访问
    /// 但函数参数可能需要跨模块访问（当函数在另一个模块时）
    pub fn get_variable_by_id(&self, id: VariableID) -> Option<&Variable> {
        self.variables.get(*id)
    }

    /// 记录引用
    pub fn new_reference(&mut self, range: TextRange, tag: ReferenceTag) {
        let ref_var = Reference { range, tag };
        let ref_idx = self.reference.insert(ref_var);
        let ref_id = ReferenceID(ref_idx);

        self.reference_map.insert(range, ref_id);

        match tag {
            ReferenceTag::VarRead(variable_id) => self
                .index
                .variable_reference
                .entry(variable_id)
                .or_default()
                .push(ref_id),
            ReferenceTag::FuncCall(function_id) => self
                .index
                .function_reference
                .entry(function_id)
                .or_default()
                .push(ref_id),
        };
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
define_id_type!(ScopeID);
define_id_type!(ReferenceID);

macro_rules! define_module_id_type {
    ($name:ident) => {
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
        pub struct $name {
            pub module: FileID,
            pub index: thunderdome::Index,
        }

        impl $name {
            pub fn none() -> Self {
                $name {
                    module: FileID::none(),
                    index: thunderdome::Index::DANGLING,
                }
            }

            pub fn new(module: FileID, index: thunderdome::Index) -> Self {
                $name { module, index }
            }
        }

        impl From<(FileID, thunderdome::Index)> for $name {
            fn from((module, index): (FileID, thunderdome::Index)) -> Self {
                $name { module, index }
            }
        }
    };
}

define_module_id_type!(StructID);
define_module_id_type!(FunctionID);
define_module_id_type!(FieldID);

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
#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<VariableID>,
    pub param_types: Vec<NType>,
    pub ret_type: NType,
    pub have_impl: bool,
    pub range: TextRange,
}

#[derive(Debug, Clone)]
pub struct Struct {
    pub name: String,
    pub fields: Vec<FieldID>,
    pub range: TextRange,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: NType,
    pub range: TextRange,
}

impl Struct {
    /// 根据字段名查找字段索引
    pub fn field_index(&self, module: &Module, name: &str) -> Option<u32> {
        self.fields
            .iter()
            .position(|field_id| {
                module
                    .fields
                    .get(field_id.index)
                    .map(|field| field.name == name)
                    .unwrap_or(false)
            })
            .map(|i| i as u32)
    }

    /// 根据字段名查找字段 ID
    pub fn field(&self, module: &Module, name: &str) -> Option<FieldID> {
        self.fields
            .iter()
            .find(|field_id| {
                module
                    .fields
                    .get(field_id.index)
                    .map(|field| field.name == name)
                    .unwrap_or(false)
            })
            .copied()
    }

    /// 根据索引获取字段 ID
    pub fn field_at(&self, index: usize) -> Option<FieldID> {
        self.fields.get(index).copied()
    }
}

#[derive(Debug)]
pub struct Scope {
    pub parent: Option<ScopeID>,
    pub variables: HashMap<String, VariableID>,
    pub range: TextRange,
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
