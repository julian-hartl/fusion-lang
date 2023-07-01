use std::fmt;
use std::fmt::{Display, Formatter};

use fusion_compiler::{idx};
use crate::ast::lexer::token::Token;

use crate::hir::{FieldIdx, FunctionIdx, StructIdx, VariableIdx};
use crate::modules::scopes::GlobalScope;
use crate::typings::{Layout, Type};

use fusion_compiler::Idx;

idx!(ModuleIdx);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct QualifiedName {
    pub name: String,
    pub module: ModuleIdx,
}

impl QualifiedName {
    pub fn unqualified_name(&self) -> &str {
        self.name.split("::").last().unwrap()
    }
}

impl Display for QualifiedName {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Variable {
    pub name: String,
    pub ty: Type,
    pub is_mutable: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Function {
    pub name: QualifiedName,
    pub parameters: Vec<VariableIdx>,
    pub return_type: Type,
    pub modifiers: Vec<FunctionModifier>,
}

impl Function {
    pub fn is_extern(&self) -> bool {
        self.modifiers.contains(&FunctionModifier::Extern)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum FunctionModifier {
    Extern,
}

#[derive(Debug, Clone)]
pub struct Struct {
    pub name: QualifiedName,
    pub fields: Vec<FieldIdx>,
    pub decl_token: Token,
    pub decl_in_module: ModuleIdx,
}

impl Struct {

    pub fn layout(&self, scope: &GlobalScope) -> Layout {
        let mut size = 0;
        let mut alignment = 0;
        for field in self.fields.iter() {
            let field = scope.get_field(field);
            let field_layout = field.ty.layout(scope);
            size += field_layout.size;
            alignment = alignment.max(field_layout.alignment);
        }
        Layout {
            size,
            alignment,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct StructField {
    pub name: String,
    pub ty: Type,
    pub struct_id: StructIdx,
}
