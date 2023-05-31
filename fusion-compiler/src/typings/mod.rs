use std::fmt::{Display, Formatter};
use std::ops::DerefMut;

use crate::ast::lexer::TokenKind;
use crate::hir::StructIdx;
use crate::modules::scopes::GlobalScope;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionType {
    pub parameters: Vec<Type>,
    pub return_type: Box<Type>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    Class(String),
}

impl Display for SymbolKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SymbolKind::Class(name) => write!(f, "{}", name),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum IntSize {
    I8,
    I16,
    I32,
    I64,
    ISize,
}

impl Display for IntSize {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IntSize::I8 => write!(f, "i8"),
            IntSize::I16 => write!(f, "i16"),
            IntSize::I32 => write!(f, "i32"),
            IntSize::I64 => write!(f, "i64"),
            IntSize::ISize => write!(f, "isize"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Integer(IntSize),
    Bool,
    Char,
    Void,
    Ptr(Box<Type>, bool),
    Struct(StructIdx),
    Function(FunctionType),
    Unresolved,
    Error,
}


impl Type {

    pub fn get_integer_types() -> [Type; 4] {
        return [
            Type::Integer(IntSize::I8),
            Type::Integer(IntSize::I16),
            Type::Integer(IntSize::I32),
            Type::Integer(IntSize::I64),
        ]
    }

    pub fn get_built_in_types() -> [Type; 7] {
        return [
            Type::Integer(IntSize::I8),
            Type::Integer(IntSize::I16),
            Type::Integer(IntSize::I32),
            Type::Integer(IntSize::I64),
            Type::Bool,
            Type::Char,
            Type::Void,
        ]
    }

}

#[derive(Debug, Copy, Clone)]
pub struct Layout {
    pub size: u32,
    pub alignment: u32,
}

impl Layout {
    pub const POINTER_SIZE: u32 = 8;
}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Integer(size) => write!(f, "{}", size),
            Type::Bool => write!(f, "bool"),
            Type::Unresolved => write!(f, "unresolved"),
            Type::Void => write!(f, "void"),
            Type::Error => write!(f, "?"),
            Type::Function(FunctionType { parameters, return_type, name }) => {
                let parameters = parameters.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ");
                let return_type = return_type.to_string();
                write!(f, "({}) -> {}", parameters, return_type)
            }
            Type::Ptr(ty, is_mut) => {
                if *is_mut {
                    write!(f, "*mut {}", ty)
                } else {
                    write!(f, "*{}", ty)
                }
            }
            Type::Char => write!(f, "char"),
            Type::Struct(id) => {
                // todo: use the name of the struct
                write!(f, "{:?}", id)
            }
        }
    }
}

impl Type {
    pub fn StringSlice(as_mut: bool) -> Self {
        Type::Ptr(Box::new(Type::Char), as_mut)
    }

    pub fn is_assignable_to(&self, other: &Type) -> bool {
        if self == other {
            return true;
        }
        match (self, other) {
            (Type::Ptr(ty1, is_mutable_1), Type::Ptr(ty2, is_mutable_2)) => {
                if *is_mutable_2 {
                    return *is_mutable_1;
                }
                if **ty1 == Type::Void || **ty2 == Type::Void {
                    return true;
                }
                ty1.is_assignable_to(ty2)
            }
            (Type::Error, _) => true,
            (_, Type::Error) => true,
            _ => false,
        }
    }

    pub fn get_builtin_type(name: &str) -> Option<Type> {
        match name {
            "isize" => Some(Type::Integer(IntSize::ISize)),
            "i64" => Some(Type::Integer(IntSize::I64)),
            "i32" => Some(Type::Integer(IntSize::I32)),
            "i16" => Some(Type::Integer(IntSize::I16)),
            "i8" => Some(Type::Integer(IntSize::I8)),
            "bool" => Some(Type::Bool),
            "char" => Some(Type::Char),
            "void" => Some(Type::Void),
            _ => None,
        }
    }

    pub fn deref(&self) -> Option<Type> {
        match self {
            Type::Ptr(ty, _) => Some(*ty.clone()),
            _ => None,
        }
    }

    pub fn layout(&self, scope: &GlobalScope) -> Layout {
        match self {
            Type::Integer(size) => match size {
                IntSize::I8 => Layout {
                    size: 1,
                    alignment: 1,
                },
                IntSize::I16 => Layout {
                    size: 2,
                    alignment: 2,
                },
                IntSize::I32 => Layout {
                    size: 4,
                    alignment: 4,
                },
                IntSize::I64 => Layout {
                    size: 8,
                    alignment: 8,
                },
                IntSize::ISize => Layout {
                    size: Layout::POINTER_SIZE,
                    alignment: Layout::POINTER_SIZE,
                },
            },
            Type::Bool => Layout {
                size: 1,
                alignment: 1,
            },
            Type::Void => Layout {
                size: 0,
                alignment: 0,
            },
            Type::Ptr(_, _) => Layout {
                size: Layout::POINTER_SIZE,
                alignment: Layout::POINTER_SIZE,
            },
            Type::Char => Layout {
                size: 1,
                alignment: 1,
            },
            Type::Struct(id) => {
                let struct_ = scope.get_struct(id);
                struct_.layout(scope)
            }
            _ => unimplemented!("layout for type {:?}", self),
        }
    }
}