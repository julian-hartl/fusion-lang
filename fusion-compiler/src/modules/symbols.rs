use std::fmt;
use std::fmt::{Display, Formatter};
use fusion_compiler::{id, id_generator};
use crate::hir::{FieldId, FunctionId, StructId, VariableId};
use crate::modules::scopes::GlobalScope;
use crate::typings::{Layout, Type};

id!(ModuleId);
id_generator!(ModuleIdGenerator, ModuleId);

#[derive(Debug, Clone, Eq, PartialEq, Hash)]

pub struct QualifiedName {
    pub name: String,
    pub module: ModuleId,
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

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Variable {
    pub name: String,
    pub ty: Type,
    pub id: VariableId,
    pub is_mutable: bool,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Function {
    pub name: QualifiedName,
    pub parameters: Vec<VariableId>,
    pub return_type: Type,
    pub id: FunctionId,
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

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Struct {
    pub name: QualifiedName,
    pub fields: Vec<FieldId>,
    pub id: StructId,
}

impl Struct {
    pub fn ty(&self) -> Type {
        Type::Struct(self.id)
    }

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

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct StructField {
    pub name: String,
    pub ty: Type,
    pub id: FieldId,
    pub struct_id: StructId,
}
