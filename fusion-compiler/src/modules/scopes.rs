use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;

use fusion_compiler::IdxVec;

use crate::ast::lexer::Token;
use crate::ast::QualifiedIdentifier;
use crate::hir::{FieldIdx, FunctionIdx, StructIdx, VariableIdx};
use crate::modules::symbols::{Function, FunctionModifier, ModuleIdx, QualifiedName, Struct, StructField, Variable};
use crate::typings::Type;

pub struct LocalScope {
    variables: Vec<VariableIdx>,
}

impl LocalScope {
    pub fn new() -> Self {
        Self {
            variables: Vec::new(),
        }
    }
}

pub type GlobalScopeCell = Rc<RefCell<GlobalScope>>;

pub struct Module {
    pub name: String,
    pub submodules: Vec<ModuleIdx>,
    pub parent: Option<ModuleIdx>,
    functions: Vec<FunctionIdx>,
    variables: Vec<VariableIdx>,
    structs: Vec<StructIdx>,
    local_scopes: Vec<LocalScope>,
    surrounding_function: Option<FunctionIdx>,
}

impl Module {
    pub fn has_direct_submodule(&self, name: &str, modules: &IdxVec<ModuleIdx, Module>) -> bool {
        self.submodules.iter().any(|id| {
            let module = modules.get(*id);
            module.name == *name
        })
    }

    pub fn find_submodule(&self, name: &str, modules: &IdxVec<ModuleIdx, Module>) -> Option<ModuleIdx> {
        self.submodules.iter().find_map(|id| {
            let module = modules.get(*id);
            if module.name == *name {
                Some(*id)
            } else {
                None
            }
        })
    }
}

impl Module {
    pub fn new(
        name: String,
        submodules: Vec<ModuleIdx>,
        parent: Option<ModuleIdx>,
    ) -> Self {
        Self {
            name,
            parent,
            functions: Vec::new(),
            variables: Vec::new(),
            structs: Vec::new(),
            local_scopes: Vec::new(),
            surrounding_function: None,
            submodules,
        }
    }
}

pub enum SymbolLookupResult<T> {
    ModuleNotFound {
        index: usize,
    },
    SymbolNotFound,
    Found(T),
}

impl<T> From<Option<T>> for SymbolLookupResult<T> {
    fn from(option: Option<T>) -> Self {
        match option {
            Some(value) => SymbolLookupResult::Found(value),
            None => SymbolLookupResult::SymbolNotFound,
        }
    }
}

pub struct GlobalScope {
    pub root_module: ModuleIdx,
    modules: IdxVec<ModuleIdx, Module>,
    pub external_modules: Vec<ModuleIdx>,
    current_module: ModuleIdx,
    functions: IdxVec<FunctionIdx, Function>,
    variables: IdxVec<VariableIdx, Variable>,
    pub(crate) structs: IdxVec<StructIdx, Struct>,
    fields: IdxVec<FieldIdx, StructField>,
}

impl GlobalScope {
    const ROOT_MODULE_NAME: &'static str = "root";
    pub fn new() -> Self {
        let mut modules = IdxVec::new();
        let root_module_idx = Self::create_root_module(
            &mut modules
        );
        let mut scope = Self {
            modules,
            external_modules: Vec::new(),
            root_module: root_module_idx,
            current_module: root_module_idx,
            functions: IdxVec::new(),
            variables: IdxVec::new(),
            structs: IdxVec::new(),
            fields: IdxVec::new(),
        };
        scope.create_external_modules();
        scope
    }

    pub fn get_surrounding_function(&self) -> Option<FunctionIdx> {
        self.current_module().surrounding_function
    }

    pub fn functions(&self) -> &IdxVec<FunctionIdx, Function> {
        &self.functions
    }

    pub fn set_current_module(&mut self, id: ModuleIdx) {
        self.current_module = id;
    }

    pub fn get_module(&self, id: &ModuleIdx) -> &Module {
        self.modules.get(*id)
    }

    fn create_root_module(
        modules: &mut IdxVec<ModuleIdx, Module>,
    ) -> ModuleIdx {
        let root_module = Module::new(
            Self::ROOT_MODULE_NAME.to_string(),
            Vec::new(),
            None,
        );
        modules.push(root_module)
    }

    fn create_external_modules(&mut self) {
        let external_modules_path = Self::get_external_modules_path();
        let external_modules = std::fs::read_dir(external_modules_path);
        match external_modules {
            Ok(external_modules) => {
                for module in external_modules {
                    let module = module.unwrap();
                    let module_path = module.path();
                    let module_name = module_path.file_name().unwrap().to_str().unwrap();
                    let module_name = module_name.to_string();
                    let module = Module::new(module_name, Vec::new(), None);
                    let module_idx = self.modules.push(module);
                    self.external_modules.push(module_idx);
                }
            }
            Err(_) => {
                println!("Warning: Could not find external modules directory.");
            }
        }
    }

    pub fn get_external_modules_path() -> PathBuf {
        dirs::home_dir().unwrap().join(".fusion").join("modules")
    }

    pub fn declare_module(&mut self, name: String) -> fusion_compiler::Result<ModuleIdx> {
        if self.current_module().has_direct_submodule(&name, &self.modules) {
            return Err(());
        }
        let module = Module::new(name, Vec::new(), Some(self.current_module));
        let id = self.modules.push(module);
        self.current_module_mut().submodules.push(id);
        Ok(id)
    }

    pub fn current_module(&self) -> &Module {
        self.modules.get(self.current_module)
    }

    fn current_module_mut(&mut self) -> &mut Module {
        self.modules.get_mut(self.current_module)
    }

    pub fn declare_struct(&mut self, name: Token) -> fusion_compiler::Result<StructIdx> {
        let literal = &name.span.literal;
        if self.lookup_struct_unqualified(literal).is_some() {
            return Err(());
        }
        let struct_ = Struct {
            name: self.qualify_name(literal),
            fields: Vec::new(),
            decl_token: name,
            decl_in_module: self.current_module,
        };
        let id = self.structs.push(struct_);
        self.current_module_mut().structs.push(id);
        Ok(id)
    }

    fn qualify_name(&self, name: &str) -> QualifiedName {
        let mut qualified_name = self.current_qualified_name();
        qualified_name.push_str("::");
        qualified_name.push_str(name);
        QualifiedName {
            name: qualified_name,
            module: self.current_module,
        }
    }

    fn current_qualified_name(&self) -> String {
        let module_id = self.current_module;
        self.get_qualified_name_for_module(module_id)
    }

    pub(crate) fn get_qualified_name_for_module(&self, mut module_id: ModuleIdx) -> String {
        let mut qualified_name = String::new();
        loop {
            if module_id == self.root_module {
                qualified_name.insert_str(0, Self::ROOT_MODULE_NAME);
                break;
            }
            let module = self.modules.get(module_id);
            qualified_name.insert_str(0, &module.name);
            let parent = self.get_module(&module_id).parent;
            match parent {
                Some(parent) => {
                    qualified_name.insert_str(0, "::");

                    module_id = parent;
                }
                None => break,
            }
        }
        qualified_name
    }

    pub fn set_struct_fields(&mut self, id: &StructIdx, fields: Vec<(String, Type)>) -> fusion_compiler::Result<()> {
        let struct_ = self.structs.get_mut(*id);
        struct_.fields = fields.into_iter().map(|(name, ty)| {
            let struct_id = *id;
            let field = StructField {
                name,
                ty,
                struct_id,
            };
            self.fields.push(field)
        }).collect();
        Ok(())
    }

    pub fn lookup_struct_unqualified(&self, name: &str) -> Option<StructIdx> {
        self.lookup_struct_in_module(name, &self.current_module)
    }

    fn lookup_struct_in_module(&self, name: &str, module_id: &ModuleIdx) -> Option<StructIdx> {
        let module = self.get_module(module_id);
        module.structs
            .iter()
            .find(|struct_id| {
                let struct_ = self.get_struct(struct_id);
                struct_.name.unqualified_name() == name
            }).map(|struct_id| *struct_id)
    }

    pub fn lookup_struct_qualified(&self, name: &QualifiedIdentifier) -> SymbolLookupResult<StructIdx> {
        let (unqualified_name, effective_module_id) = match self.do_qualified_lookup(name) {
            Ok(value) => value,
            Err(value) => return value,
        };

        self.lookup_struct_in_module(unqualified_name, &effective_module_id).into()
    }

    fn do_qualified_lookup<'a, SymbolId>(&self, name: &'a QualifiedIdentifier) -> Result<(&'a str, ModuleIdx), SymbolLookupResult<SymbolId>> {
        let mut name_parts: Vec<&str> = name.parts.iter().map(|part| part.span.literal.as_str()).collect();
        let mut effective_module_id = if name_parts[0] == Self::ROOT_MODULE_NAME {
            name_parts.remove(0);
            self.root_module
        } else { self.current_module };
        let mut is_root = true;
        while name_parts.len() > 1 {
            let module_name = name_parts[0];
            let effective_module = self.get_module(&effective_module_id);
            let module = effective_module.find_submodule(module_name, &self.modules).or_else(
                || {
                    if is_root {
                        self.find_external_module(module_name)
                    } else {
                        None
                    }
                }
            );
            match module {
                None => {
                    return Err(SymbolLookupResult::ModuleNotFound {
                        index: name.parts.len() - name_parts.len(),
                    });
                }
                Some(module) => {
                    effective_module_id = module;
                    name_parts.remove(0);
                }
            }
            is_root = false;
        }
        Ok((name_parts[0], effective_module_id))
    }

    fn find_external_module(&self, name: &str) -> Option<ModuleIdx> {
        self.external_modules.iter().find(|module_id| {
            let module = self.get_module(module_id);
            module.name == name
        }).map(|module_id| *module_id)
    }

    pub fn lookup_field(&self, struct_id: &StructIdx, name: &str) -> Option<FieldIdx> {
        let struct_ = self.structs.get(*struct_id);
        struct_.fields.iter().find(|field_id| {
            let field = self.get_field(field_id);
            field.name == name
        }).map(|field_id| *field_id)
    }

    pub fn get_field(&self, id: &FieldIdx) -> &StructField {
        self.fields.get(*id)
    }

    pub fn get_field_offset(&self, id: &FieldIdx) -> u32 {
        let field = self.get_field(id);
        let struct_ = self.get_struct(&field.struct_id);
        let mut offset = 0;
        for field_id in &struct_.fields {
            if field_id == id {
                return offset;
            }
            let field = self.get_field(field_id);
            offset += field.ty.layout(self).size;
        }
        unreachable!()
    }

    pub fn get_struct(&self, id: &StructIdx) -> &Struct {
        self.structs.get(*id)
    }

    pub fn declare_function(
        &mut self,
        name: String,
        parameters: Vec<VariableIdx>,
        return_type: Type,
        modifiers: Vec<FunctionModifier>,
    ) -> fusion_compiler::Result<FunctionIdx> {
        if self.lookup_function_unqualified(&name).is_some() {
            return Err(());
        }
        let name = if modifiers.contains(&FunctionModifier::Extern) {
            // todo: this is a quick fix for now. Later we should not do that but rather adapt the names of the asm functions
            QualifiedName {
                name,
                module: self.current_module,
            }
        } else {
            self.qualify_name(&name)
        };
        let function = Function {
            name,
            parameters,
            return_type,
            modifiers,
        };
        let id = self.functions.push(function);
        self.current_module_mut().functions.push(id);
        Ok(id)
    }

    pub fn get_function(&self, id: &FunctionIdx) -> &Function {
        self.functions.get(*id)
    }

    pub fn lookup_function_unqualified(&self, name: &str) -> Option<FunctionIdx> {
        let qualified_name = self.qualify_name(name);
        self.functions
            .indexed_iter()
            .find(|(_, f)| f.name == qualified_name)
            .map(|(id, _)| id)
    }

    pub fn lookup_function_qualified(&self, name: &QualifiedIdentifier) -> SymbolLookupResult<FunctionIdx> {
        let (unqualified_name, effective_module_id) = match self.do_qualified_lookup(name) {
            Ok(value) => value,
            Err(value) => return value,
        };

        self.lookup_function_in_module(unqualified_name, &effective_module_id).into()
    }

    fn lookup_function_in_module(&self, name: &str, module_id: &ModuleIdx) -> Option<FunctionIdx> {
        let module = self.get_module(module_id);
        module.functions
            .iter()
            .find(|function_id| {
                let function = self.get_function(function_id);
                function.name.unqualified_name() == name
            }).map(|function_id| *function_id)
    }

    pub fn declare_variable(
        &mut self,
        name: String,
        ty: Type,
        is_mutable: bool,
    ) -> VariableIdx {
        let variable = Variable { name, ty, is_mutable };
        let id = self.variables.push(variable);
        match self.current_local_scope() {
            Some(local_scope) => {
                local_scope.variables.push(id);
            }
            None => {
                // todo: declare global variable
            }
        }
        id
    }

    fn current_local_scope(&mut self) -> Option<&mut LocalScope> {
        self.current_module_mut().local_scopes.last_mut()
    }

    pub fn lookup_variable(&self, name: &str) -> Option<VariableIdx> {
        for local_scope in self.current_module().local_scopes.iter().rev() {
            for var_idx in local_scope.variables.iter().rev() {
                let var = self.get_variable(var_idx);
                if var.name == name {
                    return Some(*var_idx);
                }
            }
        }
        None
    }

    pub fn get_variable(&self, id: &VariableIdx) -> &Variable {
        self.variables.get(*id)
    }


    pub fn enter_local_scope(&mut self) {
        self.current_module_mut().local_scopes.push(LocalScope::new());
    }

    pub fn exit_local_scope(&mut self) {
        self.current_module_mut().local_scopes.pop();
    }

    pub fn enter_function_scope(&mut self, function_id: FunctionIdx) {
        self.current_module_mut().surrounding_function = Some(function_id);
        self.enter_local_scope();
        let function = self.get_function(&function_id);
        for parameter_id in function.parameters.clone() {
            self.current_local_scope()
                .unwrap()
                .variables
                .push(parameter_id);
        }
    }

    pub fn exit_function_scope(&mut self) {
        self.current_module_mut().surrounding_function = None;
        self.exit_local_scope();
    }

    pub fn check_structs_for_infinite_size(&self) -> std::result::Result<(), StructIdx> {
        for struct_ in self.structs.indexed_iter() {
            let mut visited_structs = HashSet::new();
            self.check_struct_for_infinite_size(struct_, &mut visited_structs)?;
        }
        Ok(())
    }

    fn check_struct_for_infinite_size(&self, struct_: (StructIdx, &Struct), visited_structs: &mut HashSet<StructIdx>) -> Result<(), StructIdx> {
        if visited_structs.contains(&struct_.0) {
            return Err(struct_.0);
        }
        visited_structs.insert(struct_.0);
        for field_id in &struct_.1.fields {
            let field = self.get_field(field_id);
            if let Type::Struct(struct_id) = field.ty {
                let struct_ = self.get_struct(&struct_id);
                self.check_struct_for_infinite_size((struct_id, struct_), visited_structs)?;
            }
        }
        Ok(())
    }
}
