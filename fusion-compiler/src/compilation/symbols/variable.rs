use crate::typings::Type;
use std::sync::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    static ref VARIABLE_ID_GENERATOR: Mutex<VariableIdGenerator> = Mutex::new(VariableIdGenerator::new());
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Copy)]
pub struct VariableId {
    pub index: usize,
}

impl VariableId {
    pub fn new(index: usize) -> Self {
        VariableId {
            index,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VariableSymbol {
    pub name: String,
    pub ty: Type,
    pub id: VariableId,
}

impl VariableSymbol {
    pub fn new(name: String, ty: Type) -> Self {
        VariableSymbol {
            name,
            ty,
            id: VARIABLE_ID_GENERATOR.lock().unwrap().next(),
        }
    }
}

struct VariableIdGenerator {
    id: usize,
}

impl VariableIdGenerator {
    fn new() -> Self {
        VariableIdGenerator {
            id: 0,
        }
    }

    fn next(&mut self) -> VariableId {
        let id = self.id;
        self.id += 1;
        VariableId::new(id)
    }
}
