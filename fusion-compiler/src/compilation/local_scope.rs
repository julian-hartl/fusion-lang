use std::collections::HashMap;
use crate::compilation::symbols::class::ClassSymbol;
use crate::compilation::symbols::function::FunctionSymbol;
use crate::compilation::symbols::variable::VariableSymbol;
use crate::typings::Type;

pub struct LocalScope {
    pub variables: HashMap<String, VariableSymbol>,
    // todo: make reference
    pub function: Option<FunctionSymbol>,
    pub class: Option<ClassSymbol>,
}

impl LocalScope {
    pub fn new(
        function: Option<FunctionSymbol>,
        class: Option<ClassSymbol>,
    ) -> Self {
        LocalScope {
            variables: HashMap::new(),
            function,
            class,
        }
    }

    pub fn declare_variable(&mut self, identifier: &str, ty: Type) -> &VariableSymbol {
        let variable = VariableSymbol::new(identifier.to_string(), ty);
        self.variables.insert(identifier.to_string(), variable);
        self.variables.get(identifier).unwrap()
    }

    pub  fn add_variable(&mut self, variable: &VariableSymbol) -> &VariableSymbol {
        self.variables.insert(variable.name.clone(), variable.clone());
        self.variables.get(&variable.name).unwrap()
    }

    pub fn lookup_variable(&self, identifier: &str) -> Option<&VariableSymbol> {
        self.variables.get(identifier)
    }
}
