use crate::{module::StructID, value::Value};
use std::fmt::{self};
use syntax::SyntaxKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    I32,
    I8,
    F32,
    Bool,
    Void,
    Array(Box<Ty>, Option<i32>),
    Pointer { pointee: Box<Ty>, is_const: bool },
    Struct { id: StructID, name: String },
    Const(Box<Ty>),
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ty::I32 => write!(f, "i32"),
            Ty::I8 => write!(f, "i8"),
            Ty::F32 => write!(f, "f32"),
            Ty::Bool => write!(f, "bool"),
            Ty::Void => write!(f, "void"),
            Ty::Array(inner, size) => {
                if let Some(s) = size {
                    write!(f, "[{}; {}]", inner, s)
                } else {
                    write!(f, "[{}; ?]", inner)
                }
            }
            Ty::Pointer { pointee, is_const } => {
                if *is_const {
                    write!(f, "*const {}", pointee)
                } else {
                    write!(f, "*mut {}", pointee)
                }
            }
            Ty::Struct { name, .. } => write!(f, "struct {}", name),
            Ty::Const(inner) => write!(f, "const {}", inner),
        }
    }
}

impl Ty {
    /// 检查是否为数组类型（包括 Const(Array(...))）
    pub fn is_array(&self) -> bool {
        match self {
            Self::Array(_, _) => true,
            Self::Const(inner) => inner.is_array(),
            _ => false,
        }
    }

    /// 检查是否为指针类型（包括 Const(Pointer {...})）
    pub fn is_pointer(&self) -> bool {
        self.pointer_inner().is_some()
    }

    /// 检查是否为结构体类型
    pub fn is_struct(&self) -> bool {
        self.as_struct_id().is_some()
    }

    /// 提取指针类型的内部类型，处理 Pointer {...} 和 Const(Pointer {...}) 两种情况
    pub fn pointer_inner(&self) -> Option<&Ty> {
        match self {
            Self::Pointer { pointee, .. } => Some(pointee.as_ref()),
            Self::Const(inner) => {
                if let Self::Pointer { pointee, .. } = inner.as_ref() {
                    Some(pointee.as_ref())
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn is_const(&self) -> bool {
        match self {
            Self::Array(inner, _) => inner.is_const(),
            Self::Const(_) => true,
            Self::Pointer { is_const, .. } => *is_const,
            _ => false,
        }
    }

    /// 去掉 Const 包装，返回内部类型
    pub fn unwrap_const(&self) -> Ty {
        match self {
            Self::Const(inner) => inner.as_ref().clone(),
            _ => self.clone(),
        }
    }

    /// 提取 struct ID（处理 Struct 和 Const(Struct) 两种情况）
    pub fn as_struct_id(&self) -> Option<StructID> {
        match self {
            Self::Struct { id, .. } => Some(*id),
            Self::Const(inner) => inner.as_struct_id(),
            _ => None,
        }
    }

    /// 提取 struct 指针的 struct ID（处理 Pointer{Struct} 和 Const(Pointer{Struct})）
    pub fn as_struct_pointer_id(&self) -> Option<StructID> {
        match self {
            Self::Pointer { pointee, .. } => pointee.as_struct_id(),
            Self::Const(inner) => inner.as_struct_pointer_id(),
            _ => None,
        }
    }

    /// 返回标量零值（int / float）
    pub fn const_zero(&self) -> Value {
        match self {
            Ty::I32 => Value::I32(0),
            Ty::I8 => Value::I8(0),
            Ty::F32 => Value::F32(0.0),
            Ty::Bool => Value::Bool(false),
            Ty::Void => Value::I32(0),
            Ty::Array(ntype, _) => ntype.const_zero(),
            Ty::Pointer { .. } => Value::Null,
            Ty::Struct { id, .. } => Value::StructZero(*id),
            Ty::Const(ntype) => ntype.const_zero(),
        }
    }

    /// 判断两种类型是否兼容
    pub fn assign_to_me_is_ok(&self, other: &Self) -> bool {
        match (self, other) {
            (Ty::Void, Ty::Void) => true,
            (Ty::I32, Ty::I32) => true,
            (Ty::I8, Ty::I8) => true,
            (Ty::F32, Ty::F32) => true,
            (Ty::Bool, Ty::Bool) => true,

            // 整数类型隐式转
            (Ty::I32, Ty::I8 | Ty::Bool) => true,
            (Ty::I8, Ty::Bool) => true,

            // 指针类型：*void 可以与任何指针互转
            (Ty::Pointer { pointee: p1, .. }, Ty::Pointer { pointee: p2, .. }) => {
                matches!(p1.as_ref(), Ty::Void) || matches!(p2.as_ref(), Ty::Void) || p1 == p2
            }

            (Ty::Struct { id: id1, .. }, Ty::Struct { id: id2, .. }) => id1 == id2,
            (Ty::Const(inner), Ty::Const(r_inner)) => inner.assign_to_me_is_ok(r_inner),
            (Ty::Const(inner), _) => inner.assign_to_me_is_ok(other),
            (_, Ty::Const(inner)) => self.assign_to_me_is_ok(inner),
            _ => false,
        }
    }

    /// 计算二元表达式的结果类型  
    /// 支持 i8/i32/bool 之间的隐式类型转换
    /// 结果总是非 const
    pub fn compute_binary_result_type(lhs: &Ty, rhs: &Ty, op: SyntaxKind) -> Option<Ty> {
        use SyntaxKind::*;

        // 先去掉 const 包装，统一处理
        let lhs_unwrapped = lhs.unwrap_const();
        let rhs_unwrapped = rhs.unwrap_const();

        match op {
            // 算术运算符: +, -, *, /, %
            PLUS | MINUS | STAR | SLASH | PERCENT => match (&lhs_unwrapped, &rhs_unwrapped) {
                // 指针算术：支持 i8 和 i32
                (l, Ty::I32 | Ty::I8) if l.is_pointer() && matches!(op, PLUS | MINUS) => {
                    Some(l.clone())
                }
                (Ty::I32 | Ty::I8, r) if r.is_pointer() && op == PLUS => Some(r.clone()),
                (l, r) if l.is_pointer() && r.is_pointer() && op == MINUS => Some(Ty::I32),

                // 相同类型保持不变
                (Ty::I32, Ty::I32) => Some(Ty::I32),
                (Ty::I8, Ty::I8) => Some(Ty::I8),
                (Ty::F32, Ty::F32) => Some(Ty::F32),

                // 混合类型提升规则
                (Ty::I32, Ty::I8 | Ty::Bool) | (Ty::I8 | Ty::Bool, Ty::I32) => Some(Ty::I32),
                (Ty::I8, Ty::Bool) | (Ty::Bool, Ty::I8) => Some(Ty::I8),

                // bool 算术运算提升到 i32
                (Ty::Bool, Ty::Bool) => Some(Ty::I32),

                // 其他情况不合法
                _ => None,
            },

            // 比较运算符: <, >, <=, >=, ==, !=
            LT | GT | LTEQ | GTEQ | EQEQ | NEQ => match (&lhs_unwrapped, &rhs_unwrapped) {
                // 整数/bool 比较：允许混合
                (Ty::I32 | Ty::I8 | Ty::Bool, Ty::I32 | Ty::I8 | Ty::Bool) => Some(Ty::Bool),

                // 浮点比较
                (Ty::F32, Ty::F32) => Some(Ty::Bool),

                // 指针比较
                (l, r) if l.is_pointer() && r.is_pointer() => Some(Ty::Bool),

                _ => None,
            },

            // 逻辑运算符: &&, ||
            AMPAMP | PIPEPIPE => match (&lhs_unwrapped, &rhs_unwrapped) {
                // 接受整数类型，返回 bool
                (Ty::I32 | Ty::I8 | Ty::Bool, Ty::I32 | Ty::I8 | Ty::Bool) => Some(Ty::Bool),
                _ => None,
            },

            // 未知操作符
            _ => None,
        }
        // 注意：结果总是非 const，不需要包装
    }

    /// 验证一元操作符并计算结果类型        
    /// 结果总是非 const
    pub fn validate_unary_op(&self, op: SyntaxKind) -> Option<Ty> {
        use SyntaxKind::*;

        // 先去掉 const 包装
        let unwrapped = self.unwrap_const();

        match (&unwrapped, op) {
            // 算术运算符: +, -
            (Ty::I32, PLUS | MINUS) => Some(Ty::I32),
            (Ty::I8, PLUS | MINUS) => Some(Ty::I8),
            (Ty::Bool, PLUS | MINUS) => Some(Ty::I32), // bool 提升到 i32
            (Ty::F32, PLUS | MINUS) => Some(Ty::F32),

            // 逻辑非: ! - 接受整数类型
            (Ty::Bool | Ty::I32 | Ty::I8, BANG) => Some(Ty::Bool),

            // 取地址: &
            // 注意：这里生成的指针类型是 *mut，不继承 const
            (ty, AMP) => Some(Ty::Pointer {
                pointee: Box::new(ty.clone()),
                is_const: false,
            }),

            // 解引用: *
            // 不检查指针的 const/mut 修饰符
            (Ty::Pointer { pointee, .. }, STAR) => Some((**pointee).clone()),

            // 其他情况不合法
            _ => None,
        }
        // 注意：结果总是非 const
    }
}
