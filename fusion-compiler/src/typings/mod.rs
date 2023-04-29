use std::fmt::{Display, Formatter};
use crate::ast::lexer::TokenKind;
use crate::compilation::symbols::function::FunctionSymbol;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionType {
    pub parameters: Vec<Type>,
    pub return_type: Box<Type>,
    pub name: String,
}

impl From<&FunctionSymbol> for FunctionType {
    fn from(symbol: &FunctionSymbol) -> Self {

        let parameters = symbol.parameters.iter().map(|p| p.ty.clone()).collect::<Vec<_>>();
        let return_type = symbol.return_type.clone();
        let name = symbol.name.clone();
        Self {
            parameters,
            return_type: Box::new(return_type),
            name,
        }

    }
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
    Str,
    Void,
    Class(String),
    Function(FunctionType),
    Unresolved,
    Error,
}

impl Display for Type {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::I64 => write!(f, "int"),
            Type::Bool => write!(f, "bool"),
            Type::Unresolved => write!(f, "unresolved"),
            Type::Void => write!(f, "void"),
            Type::Error => write!(f, "?"),
            Type::Str => write!(f, "string"),
            Type::Class(name) => write!(f, "{}", name),
            Type::Function(FunctionType { parameters, return_type, name }) => {
                let parameters = parameters.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ");
                let return_type = return_type.to_string();
                write!(f, "({}) -> {}", parameters, return_type)
            }
        }

    }
}

impl Type {
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

    pub fn from_token_kind(s: &TokenKind) -> Option<Type> {
        match s {
            TokenKind::I64 => Some(Type::I64),
            TokenKind::Bool => Some(Type::Bool),
            TokenKind::Void => Some(Type::Void),
            TokenKind::Str => Some(Type::Str),
            _ => None,
        }
    }
}