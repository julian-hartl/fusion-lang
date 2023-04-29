use std::collections::HashMap;
use crate::ast::ASTStmtId;
use crate::compilation::symbols::class::{ClassSymbol, Constructor};
use crate::compilation::symbols::function::{FunctionModifier, FunctionSymbol};
use crate::compilation::symbols::variable::VariableSymbol;
use crate::typings::{SymbolKind, Type};

pub struct GlobalScope {
    variables: HashMap<String, VariableSymbol>,
    pub functions: HashMap<String, FunctionSymbol>,
    pub classes: HashMap<String, ClassSymbol>,
}

impl GlobalScope {
    pub fn new() -> Self {
        GlobalScope {
            variables: HashMap::new(),
            functions: HashMap::new(),
            classes: HashMap::new(),
        }
    }

    pub fn declare_variable(&mut self, identifier: &str, ty: Type) -> &VariableSymbol {
        let variable = VariableSymbol::new(identifier.to_string(), ty);
        self.variables.insert(identifier.to_string(), variable);
        self.variables.get(identifier).unwrap()
    }

    pub fn lookup_variable(&self, identifier: &str) -> Option<&VariableSymbol> {
        self.variables.get(identifier)
    }

    pub fn declare_function(&mut self, identifier: &str, function_body_id: Option<&ASTStmtId>, parameters: Vec<VariableSymbol>, return_type: Type, modifiers: Vec<FunctionModifier>) -> Result<(), ()> {
        if self.functions.contains_key(identifier) {
            return Err(());
        }
        self.replace_function(identifier, function_body_id, parameters, return_type, modifiers);
        Ok(())
    }

    pub fn replace_function(&mut self, identifier: &str, function_body_id: Option<&ASTStmtId>, parameters: Vec<VariableSymbol>, return_type: Type, modifiers: Vec<FunctionModifier>) {
        let function = FunctionSymbol::new(parameters, function_body_id.cloned(), return_type, identifier.to_string(), modifiers);

        self.functions.insert(identifier.to_string(), function);
    }

    pub fn lookup_function(&self, identifier: &str) -> Option<&FunctionSymbol> {
        self.functions.get(identifier)
    }

    pub fn declare_class(&mut self, identifier: &str, fields: Vec<VariableSymbol>, methods: Vec<FunctionSymbol>,constructor: Option<Constructor>) -> Result<(), ()> {
        if self.classes.contains_key(identifier) {
            return Err(());
        }
        self.replace_class(identifier, fields, methods,constructor);
        Ok(())
    }

    pub fn replace_class(&mut self, identifier: &str, fields: Vec<VariableSymbol>, methods: Vec<FunctionSymbol>, constructor: Option<Constructor>) {
        let class = ClassSymbol::new(identifier.to_string(), fields, methods,constructor);
        self.classes.insert(identifier.to_string(), class);
    }

    pub fn lookup_class(&self, identifier: &str) -> Option<&ClassSymbol> {
        self.classes.get(identifier)
    }

    pub fn lookup_class_member(&self, class_name: &str, member_name: &str) -> (Option<&VariableSymbol>, Option<&FunctionSymbol>) {
        let class = self.lookup_class(class_name);
        let field = class.map(|class| class.fields.iter().find(|field| field.name == member_name)).flatten();
        let method = class.map(|class| class.methods.iter().find(|method| method.name == member_name)).flatten();
        (field, method)
    }

    pub fn lookup_type(&self, name: &str) -> Option<Type> {

        if let Some(class) = self.lookup_class(name) {
            return Some(Type::Class(class.name.clone()));
        }
        None

    }
}
