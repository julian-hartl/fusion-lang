use std::fmt::{Display, Formatter};

use crate::ast::lexer::TokenKind;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    I64,
    Bool,
    Char,
    Void,
    Ptr(Box<Type>),
    Class(String),
    Function(FunctionType),
    Unresolved,
    Error,
}

#[derive(Debug)]
pub struct Layout {
    pub size: usize,
    pub alignment: usize,
}

impl Layout {

    pub const POINTER_SIZE: usize = 8;

}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::I64 => write!(f, "i64"),
            Type::Bool => write!(f, "bool"),
            Type::Unresolved => write!(f, "unresolved"),
            Type::Void => write!(f, "void"),
            Type::Error => write!(f, "?"),
            Type::Class(name) => write!(f, "{}", name),
            Type::Function(FunctionType { parameters, return_type, name }) => {
                let parameters = parameters.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ");
                let return_type = return_type.to_string();
                write!(f, "({}) -> {}", parameters, return_type)
            }
            Type::Ptr(ty) => {
                write!(f, "*{}", ty)
            }
            Type::Char => write!(f, "char"),
        }
    }
}

impl Type {

    pub fn StringSlice() -> Self {
        Type::Ptr(Box::new(Type::Char))
    }

    pub fn is_assignable_to(&self, other: &Type) -> bool {
        if self == other {
            return true;
        }
        match (self, other) {
            (Type::Error, _) => true,
            (_, Type::Error) => true,
            _ => false,
        }
    }

    pub fn get_builtin_type(name: &str) -> Option<Type> {
        match name {
            "i64" => Some(Type::I64),
            "bool" => Some(Type::Bool),
            "char" => Some(Type::Char),
            "void" => Some(Type::Void),
            _ => None,
        }
    }

    pub fn deref(&self) -> Option<Type> {
        match self {
            Type::Ptr(ty) => Some(*ty.clone()),
            _ => None,
        }
    }

    pub fn layout(&self) -> Layout {
        match self {
            Type::I64 => Layout {
                size: 8,
                alignment: 8,
            },
            Type::Bool => Layout {
                size: 1,
                alignment: 1,
            },
            Type::Void => Layout {
                size: 0,
                alignment: 0,
            },
            Type::Ptr(_) => Layout {
                size: Layout::POINTER_SIZE,
                alignment: Layout::POINTER_SIZE,
            },
            Type::Char => Layout {
                size: 1,
                alignment: 1,
            },
            _ => unimplemented!("layout for type {:?}", self),
        }
    }
}