use lazy_static::lazy_static;
use crate::compilation::symbols::function::FunctionSymbol;
use crate::compilation::symbols::variable::VariableSymbol;
use std::sync::Mutex;
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ClassSymbol {
    pub name: String,
    pub fields: Vec<VariableSymbol>,
    pub methods: Vec<FunctionSymbol>,
    pub constructor: Option<Constructor>,
    pub id: ClassId,
}

lazy_static! {
    static ref CLASS_ID_GENERATOR: Mutex<ClassIdGenerator> = Mutex::new(ClassIdGenerator::new());
}

impl ClassSymbol {
    pub fn new(name: String, fields: Vec<VariableSymbol>, methods: Vec<FunctionSymbol>, constructor: Option<Constructor>) -> Self {
        ClassSymbol {
            name,
            fields,
            methods,
            constructor,
            id: CLASS_ID_GENERATOR.lock().unwrap().next(),
        }
    }

    pub fn lookup_field(&self, name: &str) -> Option<(&VariableSymbol, u32)> {
        for (index, field) in self.fields.iter().enumerate() {
            if field.name == name {
                return Some((field, index as u32));
            }
        }
        None
    }

    pub fn lookup_method(&self, name: &str) -> Option<&FunctionSymbol> {
        for method in self.methods.iter() {
            if method.name == name {
                return Some(method);
            }
        }
        None
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Copy)]
pub struct ClassId {
    index: usize,
}

impl ClassId {
    fn new(index: usize) -> Self {
        ClassId {
            index,
        }
    }
}

struct ClassIdGenerator {
    id: usize,
}

impl ClassIdGenerator {
    fn new() -> Self {
        ClassIdGenerator {
            id: 0,
        }
    }

    fn next(&mut self) -> ClassId {
        let id = self.id;
        self.id += 1;
        ClassId::new(id)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Constructor {
    pub parameters: Vec<VariableSymbol>,
}
