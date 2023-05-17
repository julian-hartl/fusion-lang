use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::rc::Rc;

use crate::ast::lexer::Token;
use crate::hir::{FieldId, FieldIdGenerator, FunctionId, FunctionIdGenerator, StructId, StructIdGenerator, VariableId, VariableIdGenerator};
use crate::modules::symbols::{Function, FunctionModifier, ModuleId, ModuleIdGenerator, QualifiedName, Struct, StructField, Variable};
use crate::typings::Type;

pub struct LocalScope {
    variables: Vec<VariableId>,
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
    pub id: ModuleId,
    pub name: String,
    pub submodules: Vec<ModuleId>,
    pub parent: Option<ModuleId>,
    functions: Vec<FunctionId>,
    variables: Vec<VariableId>,
    structs: Vec<StructId>,
    local_scopes: Vec<LocalScope>,
    surrounding_function: Option<FunctionId>,
}

impl Module {
    pub fn has_direct_submodule(&self, name: &str, modules: &HashMap<ModuleId, Module>) -> bool {

        self.submodules.iter().any(|id| {
            let module = modules.get(id).unwrap();
            module.name == *name
        })

    }

    pub fn find_submodule(&self, name: &str, modules: &HashMap<ModuleId, Module>) -> Option<ModuleId> {
        self.submodules.iter().find_map(|id| {
            let module = modules.get(id).unwrap();
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
        id: ModuleId,
        name: String,
        submodules: Vec<ModuleId>,
        parent: Option<ModuleId>,
    ) -> Self {
        Self {
            id,
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

pub struct GlobalScope {
    pub root_module: ModuleId,
    modules: HashMap<ModuleId, Module>,
    module_id_gen: ModuleIdGenerator,
    current_module: ModuleId,
    functions: HashMap<FunctionId, Function>,
    function_id_gen: FunctionIdGenerator,
    variables: HashMap<VariableId, Variable>,
    variable_id_gen: VariableIdGenerator,
    structs: HashMap<StructId, Struct>,
    struct_id_gen: StructIdGenerator,
    fields: HashMap<FieldId, StructField>,
    field_id_gen: FieldIdGenerator,
}

impl GlobalScope {
    pub fn new(
    ) -> Self {
        let mut modules = HashMap::new();
        let mut generator = ModuleIdGenerator::new();
        let root_module = Self::create_root_module(
            &mut generator,
        );
        let root_module_id = root_module.id;
        modules.insert(root_module_id, root_module);
        Self {
            modules,
            module_id_gen: generator,
            root_module: root_module_id,
            current_module: root_module_id,
            functions: HashMap::new(),
            function_id_gen: FunctionIdGenerator::new(),
            variables: HashMap::new(),
            variable_id_gen: VariableIdGenerator::new(),
            structs: HashMap::new(),
            struct_id_gen: StructIdGenerator::new(),
            fields: HashMap::new(),
            field_id_gen: FieldIdGenerator::new(),
        }
    }

    pub fn get_surrounding_function(&self) -> Option<FunctionId> {
        self.current_module().surrounding_function
    }

    pub fn functions(&self) -> &HashMap<FunctionId, Function> {
        &self.functions
    }

    pub fn set_current_module(&mut self, id: ModuleId) {
        self.current_module = id;
    }

    pub fn get_module(&self, id: &ModuleId) -> &Module {
        self.modules.get(id).unwrap()
    }

    fn create_root_module(
        id_gen: &mut ModuleIdGenerator,
    ) -> Module {
        let root_module = Module::new(
            id_gen.next(),
            "root".to_string(),
            Vec::new(),
            None,
        );
        root_module
    }

    pub fn declare_module(&mut self, name: String) -> fusion_compiler::Result<ModuleId> {
        if self.current_module().has_direct_submodule(&name, &self.modules) {
            return Err(());
        }
        let id = self.module_id_gen.next();
        let module = Module::new(id, name, Vec::new(), Some(self.current_module));
        self.modules.insert(id, module);
        self.current_module_mut().submodules.push(id);
        Ok(id)
    }

    pub fn current_module(&self) -> &Module {
        self.modules.get(&self.current_module).unwrap()
    }

    fn current_module_mut(&mut self) -> &mut Module {
        self.modules.get_mut(&self.current_module).unwrap()
    }

    pub fn declare_struct(&mut self, name: String) -> fusion_compiler::Result<StructId> {
        if self.lookup_struct_unqualified(&name).is_some() {
            return Err(());
        }
        let id = self.struct_id_gen.next();
        let struct_ = Struct {
            name: self.qualify_name(&name),
            fields: Vec::new(),
            id,
        };
        self.structs.insert(id, struct_);
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
        let mut qualified_name = String::new();
        let mut module_id = self.current_module;
        loop {
            if module_id == self.root_module {
                qualified_name.insert_str(0, "root");
                break;
            }
            let module = self.modules.get(&module_id).unwrap();
            qualified_name.insert_str(0, &module.name);
            qualified_name.insert_str(0, "::");
            let parent = self.get_module(&module_id).parent.unwrap();
            module_id = parent;
        }
        qualified_name
    }

    pub fn set_struct_fields(&mut self, id: &StructId, fields: Vec<(String, Type)>) -> fusion_compiler::Result<()> {
        let struct_ = self.structs.get_mut(id).ok_or(())?;
        struct_.fields = fields.into_iter().map(|(name, ty)| {
            let struct_id = *id;
            let id = self.field_id_gen.next();
            let field = StructField {
                name,
                ty,
                id,
                struct_id,
            };
            self.fields.insert(id, field);
            id
        }).collect();
        Ok(())
    }

    // todo: take in qualified name and traverse modules if necessary
    pub fn lookup_struct_unqualified(&self, name: &str) -> Option<StructId> {
        self.lookup_struct_in_module(name, &self.current_module)
    }

    fn lookup_struct_in_module(&self, name: &str, module_id: &ModuleId) -> Option<StructId> {
        let module = self.get_module(module_id);
        module.structs
            .iter()
            .find(|struct_id| {
                let struct_ = self.get_struct(struct_id);
                struct_.name.unqualified_name() == name
            }).map(|struct_id| *struct_id)
    }

    pub fn lookup_struct_qualified(&self, name: &str) -> Option<StructId> {
        let mut name_parts: Vec<&str> = name.split("::").collect();
        if name_parts.len() == 1 {
            return self.lookup_struct_unqualified(name);
        }
        let mut effective_module_id = self.current_module;
        while name_parts.len() > 1 {
            let module_name = name_parts[0];
            let effective_module = self.get_module(&effective_module_id);
            let module = effective_module.find_submodule(module_name, &self.modules)?;
            effective_module_id = module;
            name_parts.remove(0);
        }

        self.lookup_struct_in_module(name_parts[0], &effective_module_id)
    }

    pub fn lookup_field(&self, struct_id: &StructId, name: &str) -> Option<FieldId> {
        let struct_ = self.structs.get(struct_id)?;
        struct_.fields.iter().find(|field_id| {
            let field = self.get_field(field_id);
            field.name == name
        }).map(|field_id| *field_id)
    }

    pub fn get_field(&self, id: &FieldId) -> &StructField {
        self.fields.get(id).unwrap()
    }

    pub fn get_field_offset(&self, id: &FieldId) -> u32 {
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

    pub fn get_struct(&self, id: &StructId) -> &Struct {
        self.structs.get(id).expect(format!("Struct with id {} not found. All structs: {:?}", id.index, self.structs.values()).as_str())
    }

    pub fn declare_function(
        &mut self,
        name: String,
        parameters: Vec<VariableId>,
        return_type: Type,
        modifiers: Vec<FunctionModifier>,
    ) -> fusion_compiler::Result<FunctionId> {
        let id = self.function_id_gen.next();
        if self.lookup_function_unqualified(&name).is_some() {
            return Err(());
        }
        let function = Function {
            name: self.qualify_name(&name),
            parameters,
            return_type,
            id,
            modifiers,
        };
        self.functions.insert(id, function);
        self.current_module_mut().functions.push(id);
        Ok(id)
    }

    pub fn get_function(&self, id: &FunctionId) -> &Function {
        self.functions.get(&id).unwrap()
    }

    pub fn lookup_function_unqualified(&self, name: &str) -> Option<FunctionId> {
        self.functions
            .iter()
            .find(|(_, f)| f.name.unqualified_name() == name)
            .map(|(id, _)| id.clone())
    }

    pub fn declare_variable(
        &mut self,
        name: String,
        ty: Type,
        is_mutable: bool,
    ) -> VariableId {
        let id = self.variable_id_gen.next();
        let variable = Variable { name, ty, id, is_mutable };
        self.variables.insert(id, variable);
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

    pub fn lookup_variable(&self, name: &str) -> Option<VariableId> {
        for local_scope in self.current_module().local_scopes.iter().rev() {
            for var in local_scope.variables.iter().rev() {
                let var = self.get_variable(var);
                if var.name == name {
                    return Some(var.id);
                }
            }
        }
        None
    }

    pub fn get_variable(&self, id: &VariableId) -> &Variable {
        self.variables.get(&id).unwrap()
    }

    pub fn resolve_type_from_identifier(
        &self,
        token: &Token,
    ) -> Option<Type> {
        let name = &token.span.literal;
        if let Some(ty) = Type::get_builtin_type(name) {
            return Some(ty);
        }
        if let Some(id) = self.lookup_struct_qualified(name) {
            return Some(Type::Struct(id));
        }
        None
    }

    pub fn enter_local_scope(&mut self) {
        self.current_module_mut().local_scopes.push(LocalScope::new());
    }

    pub fn exit_local_scope(&mut self) {
        self.current_module_mut().local_scopes.pop();
    }

    pub fn enter_function_scope(&mut self, function_id: FunctionId) {
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
}
