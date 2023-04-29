use crate::compilation::symbols::class::ClassSymbol;
use crate::compilation::symbols::function::FunctionSymbol;
use crate::compilation::symbols::variable::VariableSymbol;
use crate::compilation::global_scope::GlobalScope;
use crate::compilation::local_scope::LocalScope;
use crate::typings::Type;

pub struct Scopes {
    pub local_scopes: Vec<LocalScope>,
    pub global_scope: GlobalScope,
}

impl Scopes {
    pub fn new() -> Self {
        Scopes {
            local_scopes: Vec::new(),
            global_scope: GlobalScope::new(),
        }
    }

    pub fn from_global_scope(global_scope: GlobalScope) -> Self {
        Scopes {
            local_scopes: Vec::new(),
            global_scope,
        }
    }

    pub fn enter_scope(&mut self, function: Option<FunctionSymbol>, class: Option<ClassSymbol>) {
        self.local_scopes.push(LocalScope::new(function, class));
    }

    pub fn enter_function_scope(&mut self, function: FunctionSymbol) {
        self.local_scopes.push(LocalScope::new(Some(function), None));
    }

    pub fn enter_class_scope(&mut self, class: ClassSymbol) {
        self.local_scopes.push(LocalScope::new(None, Some(class)));
    }
    pub
    fn enter_nested_scope(&mut self) {
        self.local_scopes.push(LocalScope::new(None, None));
    }

    pub fn exit_scope(&mut self) {
        self.local_scopes.pop();
    }

    pub fn declare_variable(&mut self, identifier: &str, ty: Type) -> &VariableSymbol {
        if self.is_inside_local_scope() {
            self.local_scopes.last_mut().unwrap().declare_variable(identifier, ty)
        } else {
            self.global_scope.declare_variable(identifier, ty)
        }
    }

    pub fn declare_parameter(&mut self, param: &VariableSymbol) -> &VariableSymbol {
        self.local_scopes.last_mut().unwrap().add_variable(param)
    }

    pub fn lookup_variable(&self, identifier: &str) -> Option<&VariableSymbol> {
        for scope in self.local_scopes.iter().rev() {
            if let Some(variable) = scope.lookup_variable(identifier) {
                return Some(variable);
            }
        }
        self.global_scope.lookup_variable(identifier)
    }

    pub fn lookup_function(&self, identifier: &str) -> Option<&FunctionSymbol> {
        if let Some(class) = self.surrounding_class() {
            for method in &class.methods {
                if method.name == identifier {
                    return Some(method);
                }
            }
        }
        self.global_scope.lookup_function(identifier)
    }

    pub fn is_inside_local_scope(&self) -> bool {
        !self.local_scopes.is_empty()
    }

    pub fn surrounding_function(&self) -> Option<&FunctionSymbol> {
        for scope in self.local_scopes.iter().rev() {
            if let Some(function) = &scope.function {
                return Some(function);
            }
        }
        None
    }

    pub fn surrounding_class(&self) -> Option<&ClassSymbol> {
        for scope in self.local_scopes.iter().rev() {
            if let Some(class) = &scope.class {
                return Some(class);
            }
        }
        None
    }

    pub fn current_local_scope(&self) -> Option<&LocalScope> {
        self.local_scopes.last()
    }
}
