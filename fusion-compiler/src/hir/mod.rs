use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::rc::Rc;

use fusion_compiler::{id, id_generator, Result};

use crate::ast::{Ast, ASTBinaryOperator, ASTBinaryOperatorKind, ASTBooleanExpression, ASTCharExpression, ASTDerefExpression, ASTExpression, ASTExpressionKind, ASTFuncDeclStatement, ASTIdentifierExpression, ASTLetStatement, ASTNumberExpression, ASTRefExpression, ASTStatement, ASTStatementKind, ASTStringExpression, ASTUnaryExpression, ASTUnaryOperator, ASTUnaryOperatorKind, FuncDeclParameter, TypeSyntax};
use crate::ast::lexer::Token;
use crate::ast::visitor::ASTVisitor;
use crate::diagnostics::DiagnosticsBagCell;
use crate::text::span::TextSpan;
use crate::typings::Type;

mod visitor;
mod visualization;

pub struct HIR {
    pub scope: Scope,
    pub function_bodies: HashMap<FunctionId, Vec<HIRStatement>>,
    pub structs: Vec<HIRStruct>,
    pub globals: Vec<HIRGlobal>,
}

pub enum HIRGlobal {
    Variable {
        id: VariableId,
        initializer: HIRExpression,
    },
}

pub struct HIRFunction {
    pub name: String,
    pub parameters: Vec<HIRParameter>,
    pub return_type: Type,
    pub body: Vec<HIRStatement>,
}

pub struct HIRParameter {
    pub name: String,
    pub ty: Type,
}

pub struct HIRStruct {
    pub name: String,
    pub fields: Vec<HIRField>,
}

pub struct HIRField {
    pub name: String,
    pub ty: Type,
}

pub struct HIRStatement {
    pub kind: HIRStatementKind,
    pub span: TextSpan,
}

pub enum HIRStatementKind {
    Return(HIRReturnStatement),
    Expression(HIRExpressionStatement),
    VariableDeclaration(HIRVariableDeclarationStatement),
    If(HIRIfStatement),
    While(HIRWhileStatement),
    Block(HIRBlockStatement),
}

pub struct HIRBlockStatement {
    pub statements: Vec<HIRStatement>,
}

pub struct HIRReturnStatement {
    pub expression: HIRExpression,
}

pub struct HIRExpressionStatement {
    pub expression: HIRExpression,
}

pub struct HIRVariableDeclarationStatement {
    pub variable_id: VariableId,
    pub initializer: HIRExpression,
}

pub struct HIRIfStatement {
    pub condition: HIRExpression,
    pub then: Vec<HIRStatement>,
    pub else_: Option<Vec<HIRStatement>>,
}

pub struct HIRWhileStatement {
    pub condition: HIRExpression,
    pub body: Vec<HIRStatement>,
}


pub struct HIRExpression {
    pub kind: HIRExpressionKind,
    pub span: TextSpan,
    pub ty: Type,
}

pub enum HIRExpressionKind {
    Literal(HIRLiteralExpression),
    Variable(HIRVariableExpression),
    Assignment(HIRAssignmentExpression),
    Binary(HIRBinaryExpression),
    Unary(HIRUnaryExpression),
    Call(HIRCallExpression),
    MemberAccess(HIRMemberAccessExpression),
    Parenthesized(HIRParenthesizedExpression),
    Ref(HIRRefExpression),
    Deref(HIRDerefExpression),
    Void,
}

pub struct HIRRefExpression {
    pub expression: Box<HIRExpression>,
}

pub struct HIRDerefExpression {
    pub expression: Box<HIRExpression>,
}

pub struct HIRParenthesizedExpression {
    pub expression: Box<HIRExpression>,
}

pub struct HIRLiteralExpression {
    pub value: HIRLiteralValue,
}

pub enum HIRLiteralValue {
    Integer(i64),
    Boolean(bool),
    String(String),
    Char(char),
}

pub struct HIRVariableExpression {
    pub variable_id: VariableId,
}

pub struct HIRAssignmentExpression {
    pub target: HIRAssignmentTarget,
    pub value: Box<HIRExpression>,
}

pub struct HIRAssignmentTarget {
    pub span: TextSpan,
    pub kind: HIRAssignmentTargetKind,

}

pub enum HIRAssignmentTargetKind {
    Variable(VariableId),
    Deref(Box<HIRExpression>),
    Error
}

pub struct HIRBinaryExpression {
    pub left: Box<HIRExpression>,
    pub op: HIRBinaryOperator,
    pub right: Box<HIRExpression>,
}

impl Display for HIRBinaryOperator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let op = match self {
            HIRBinaryOperator::Add => "+",
            HIRBinaryOperator::Subtract => "-",
            HIRBinaryOperator::Multiply => "*",
            HIRBinaryOperator::Divide => "/",
            HIRBinaryOperator::Equals => "==",
            HIRBinaryOperator::NotEquals => "!=",
            HIRBinaryOperator::LessThan => "<",
            HIRBinaryOperator::LessThanOrEqual => "<=",
            HIRBinaryOperator::GreaterThan => ">",
            HIRBinaryOperator::GreaterThanOrEqual => ">=",
            HIRBinaryOperator::BitwiseAnd => "&",
            HIRBinaryOperator::BitwiseOr => "|",
            HIRBinaryOperator::BitwiseXor => "^",
            HIRBinaryOperator::Modulo => "%",
        };
        write!(f, "{}", op)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HIRBinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Equals,
    NotEquals,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
}


struct HIRBinaryOperatorDefinition {
    left: Type,
    right: Type,
    result: Type,
}

impl HIRBinaryOperator {
    fn definitions(&self) -> Vec<HIRBinaryOperatorDefinition> {
        match self {
            HIRBinaryOperator::Add => vec![
                HIRBinaryOperatorDefinition {
                    left: Type::I64,
                    right: Type::I64,
                    result: Type::I64,
                },
            ],
            HIRBinaryOperator::Subtract => vec![
                HIRBinaryOperatorDefinition {
                    left: Type::I64,
                    right: Type::I64,
                    result: Type::I64,
                },
            ],
            HIRBinaryOperator::Multiply => vec![
                HIRBinaryOperatorDefinition {
                    left: Type::I64,
                    right: Type::I64,
                    result: Type::I64,
                },
            ],
            HIRBinaryOperator::Divide => vec![
                HIRBinaryOperatorDefinition {
                    left: Type::I64,
                    right: Type::I64,
                    result: Type::I64,
                },
            ],
            HIRBinaryOperator::Equals => vec![
                HIRBinaryOperatorDefinition {
                    left: Type::I64,
                    right: Type::I64,
                    result: Type::Bool,
                },
                HIRBinaryOperatorDefinition {
                    left: Type::Bool,
                    right: Type::Bool,
                    result: Type::Bool,
                },
            ],
            HIRBinaryOperator::NotEquals => vec![
                HIRBinaryOperatorDefinition {
                    left: Type::I64,
                    right: Type::I64,
                    result: Type::Bool,
                },
                HIRBinaryOperatorDefinition {
                    left: Type::Bool,
                    right: Type::Bool,
                    result: Type::Bool,
                },
            ],
            HIRBinaryOperator::LessThan => vec![
                HIRBinaryOperatorDefinition {
                    left: Type::I64,
                    right: Type::I64,
                    result: Type::Bool,
                },
            ],
            HIRBinaryOperator::LessThanOrEqual => vec![
                HIRBinaryOperatorDefinition {
                    left: Type::I64,
                    right: Type::I64,
                    result: Type::Bool,
                },
            ],
            HIRBinaryOperator::GreaterThan => vec![
                HIRBinaryOperatorDefinition {
                    left: Type::I64,
                    right: Type::I64,
                    result: Type::Bool,
                },
            ],
            HIRBinaryOperator::GreaterThanOrEqual => vec![
                HIRBinaryOperatorDefinition {
                    left: Type::I64,
                    right: Type::I64,
                    result: Type::Bool,
                },
            ],
            HIRBinaryOperator::BitwiseAnd => vec![
                HIRBinaryOperatorDefinition {
                    left: Type::I64,
                    right: Type::I64,
                    result: Type::I64,
                },
            ],
            HIRBinaryOperator::BitwiseOr => vec![
                HIRBinaryOperatorDefinition {
                    left: Type::I64,
                    right: Type::I64,
                    result: Type::I64,
                },
            ],
            HIRBinaryOperator::BitwiseXor => vec![
                HIRBinaryOperatorDefinition {
                    left: Type::I64,
                    right: Type::I64,
                    result: Type::I64,
                },
            ],
            HIRBinaryOperator::Modulo => {
                vec![HIRBinaryOperatorDefinition {
                    left: Type::I64,
                    right: Type::I64,
                    result: Type::I64,
                }]
            }
        }
    }
}

impl From<&ASTBinaryOperator> for HIRBinaryOperator {
    fn from(op: &ASTBinaryOperator) -> Self {
        match op.kind {
            ASTBinaryOperatorKind::Plus => HIRBinaryOperator::Add,
            ASTBinaryOperatorKind::Minus => HIRBinaryOperator::Subtract,
            ASTBinaryOperatorKind::Multiply => HIRBinaryOperator::Multiply,
            ASTBinaryOperatorKind::Divide => HIRBinaryOperator::Divide,
            ASTBinaryOperatorKind::Equals => HIRBinaryOperator::Equals,

            ASTBinaryOperatorKind::NotEquals => HIRBinaryOperator::NotEquals,
            ASTBinaryOperatorKind::LessThan => HIRBinaryOperator::LessThan,
            ASTBinaryOperatorKind::LessThanOrEqual => HIRBinaryOperator::LessThanOrEqual,
            ASTBinaryOperatorKind::GreaterThan => HIRBinaryOperator::GreaterThan,

            ASTBinaryOperatorKind::GreaterThanOrEqual => HIRBinaryOperator::GreaterThanOrEqual,
            ASTBinaryOperatorKind::BitwiseAnd => HIRBinaryOperator::BitwiseAnd,
            ASTBinaryOperatorKind::BitwiseOr => HIRBinaryOperator::BitwiseOr,
            ASTBinaryOperatorKind::BitwiseXor => HIRBinaryOperator::BitwiseXor,

            ASTBinaryOperatorKind::Power => unimplemented!(),
            ASTBinaryOperatorKind::Modulo => HIRBinaryOperator::Modulo,
        }
    }
}

pub struct HIRUnaryExpression {
    pub op: HIRUnaryOperator,
    pub operand: Box<HIRExpression>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HIRUnaryOperator {
    Negate,
    BitwiseNot,
}

impl Display for HIRUnaryOperator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            HIRUnaryOperator::Negate => "-",
            HIRUnaryOperator::BitwiseNot => "~",
        };
        write!(f, "{}", s)
    }
}

impl From<&ASTUnaryOperator> for HIRUnaryOperator {
    fn from(op: &ASTUnaryOperator) -> Self {
        match op.kind {
            ASTUnaryOperatorKind::Minus => HIRUnaryOperator::Negate,
            ASTUnaryOperatorKind::BitwiseNot => HIRUnaryOperator::BitwiseNot,
        }
    }
}

struct HIRUnaryOperatorDefinition {
    operand: Type,
    result: Type,
}

impl HIRUnaryOperator {
    fn definitions(&self) -> Vec<HIRUnaryOperatorDefinition> {
        match self {
            HIRUnaryOperator::Negate => vec![HIRUnaryOperatorDefinition {
                operand: Type::I64,
                result: Type::I64,
            }],
            HIRUnaryOperator::BitwiseNot => vec![HIRUnaryOperatorDefinition {
                operand: Type::I64,
                result: Type::I64,
            }],
        }
    }
}

pub struct HIRCallExpression {
    pub callee: HIRCallee,
    pub arguments: Vec<HIRExpression>,
}

pub enum HIRCallee {
    Function(FunctionId),
    Undeclared,
    Invalid,
}

pub struct HIRMemberAccessExpression {
    pub target: Box<HIRExpression>,
    pub member: String,
}

mod common {
    use fusion_compiler::Result;

    use crate::ast::{ASTFuncDeclStatement, ASTLetStatement, FuncDeclParameter};
    use crate::ast::lexer::Token;
    use crate::diagnostics::{DiagnosticsBag, DiagnosticsBagCell};
    use crate::hir::{FunctionModifier, HIRGen, HIRStatementKind, HIRVariableDeclarationStatement, Variable, VariableId};
    use crate::hir;
    use crate::typings::Type;

    fn resolve_func_modifier(modifier: &Token, diagnostics_bag: DiagnosticsBagCell) -> Result<FunctionModifier> {
        match modifier.span.literal.as_str() {
            "extern" => Ok(FunctionModifier::Extern),
            _ => {
                diagnostics_bag.borrow_mut().report_invalid_function_modifier(&modifier.span);
                Err(())
            }
        }
    }

    pub fn declare_function(hir_gen: &mut HIRGen, diagnostics_bag: DiagnosticsBagCell, func_decl_statement: &ASTFuncDeclStatement) {
        let name = func_decl_statement.identifier.span.literal.clone();
        let parameters = func_decl_statement.parameters.iter().map(|param| {
            match param {
                FuncDeclParameter::Normal(param) => {
                    let name = param.identifier.span.literal.clone();
                    let ty = hir_gen.resolve_type(&param.type_annotation.ty);
                    let id = hir_gen.hir.scope.variable_id_gen.borrow_mut().next();
                    let variable = Variable {
                        name,
                        ty,
                        id,
                    };
                    variable
                }
                FuncDeclParameter::Self_(self_param) => {
                    diagnostics_bag.borrow_mut().report_self_outside_class(&self_param.span);
                    Variable {
                        name: String::from("self"),
                        ty: Type::Error,
                        id: hir_gen.hir.scope.variable_id_gen.borrow_mut().next(),
                    }
                }
            }
        }).collect();
        let return_type = match func_decl_statement.return_type {
            Some(ref return_type) => {
                hir_gen.resolve_type(&return_type.ty)
            }
            None => {
                Type::Void
            }
        };
        if let Err(_) = hir_gen.hir.scope.declare_function(
            name,
            parameters,
            return_type,
            func_decl_statement.modifier_tokens.iter().map(|token| resolve_func_modifier(token, diagnostics_bag.clone())).filter_map(|m| m.ok()).collect(),
        ) {
            diagnostics_bag.borrow_mut().report_function_already_declared(&func_decl_statement.identifier);
        }
    }

    pub fn declare_variable(hir_gen: &mut HIRGen, stmt: &ASTLetStatement) -> HIRVariableDeclarationStatement {
        let static_type = stmt.type_annotation.as_ref().map(|ty| hir_gen.resolve_type(&ty.ty));
        let initializer = hir_gen.gen_expression(&stmt.initializer);
        let ty = match static_type {
            None => {
                initializer.ty.clone()
            }
            Some(ty) => {
                hir_gen.ensure_type_match(&initializer.span, &initializer.ty, &ty);
                ty
            }
        };
        let variable_id = hir_gen.hir.scope.declare_variable(
            stmt.identifier.span.literal.clone(),
            ty,
        );
        HIRVariableDeclarationStatement {
            initializer,
            variable_id,
        }
    }
}

impl HIR {
    pub fn new() -> Self {
        Self {
            function_bodies: HashMap::new(),
            scope: Scope::new(),
            structs: Vec::new(),
            globals: Vec::new(),
        }
    }

    fn push_stmt(&mut self, stmt: HIRStatement, function_id: FunctionId) {
        self.function_bodies
            .entry(function_id)
            .or_insert_with(Vec::new)
            .push(stmt);
    }

    pub fn functions(&self) -> HashMap<&Function, Option<&Vec<HIRStatement>>> {
        self.scope.functions.iter().map(|(function_id, function)| {
            let body = self.function_bodies.get(function_id);
            (function, body)
        }).collect()
    }

    pub fn visualize(&self) {
        let visualizer = visualization::HIRVisualizer::new(self);
        let output = visualizer.visualize();
        println!("{}", output);
    }
}




id!(VariableId);
id_generator!(VariableIdGenerator, VariableId);
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Variable {
    pub name: String,
    pub ty: Type,
    pub id: VariableId,
}

id!(FunctionId);
id_generator!(FunctionIdGenerator, FunctionId);
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Function {
    pub name: String,
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

struct LocalScope {
    variables: HashSet<VariableId>,
    variable_id_gen: Rc<RefCell<VariableIdGenerator>>,
}

impl LocalScope {
    pub fn new(variable_id_gen: Rc<RefCell<VariableIdGenerator>>) -> Self {
        Self {
            variables: HashSet::new(),
            variable_id_gen,
        }
    }
}

pub struct Scope {
    pub functions: HashMap<FunctionId, Function>,
    function_id_gen: FunctionIdGenerator,
    variables: HashMap<VariableId, Variable>,
    global_variables: HashSet<VariableId>,
    variable_id_gen: Rc<RefCell<VariableIdGenerator>>,
    local_scopes: Vec<LocalScope>,
    surrounding_function: Option<FunctionId>,
}

impl Scope {
    fn new() -> Self {
        Self {
            functions: HashMap::new(),
            local_scopes: Vec::new(),
            function_id_gen: FunctionIdGenerator::new(),
            variables: HashMap::new(),
            variable_id_gen: Rc::new(RefCell::new(VariableIdGenerator::new())),
            surrounding_function: None,
            global_variables: HashSet::new(),
        }
    }

    fn declare_function(
        &mut self,
        name: String,
        parameters: Vec<Variable>,
        return_type: Type,
        modifiers: Vec<FunctionModifier>,
    ) -> Result<FunctionId> {
        let id = self.function_id_gen.next();
        for parameter in &parameters {
            self.variables.insert(parameter.id, parameter.clone());
        }
        let function = Function {
            name,
            parameters: parameters.iter().map(|p| p.id).collect(),
            return_type,
            id,
            modifiers,
        };
        if self.lookup_function(&function.name).is_some() {
            return Err(());
        }
        self.functions.insert(id, function);
        Ok(id)
    }

    pub fn get_function(&self, id: &FunctionId) -> &Function {
        self.functions.get(&id).unwrap()
    }

    fn lookup_function(&self, name: &str) -> Option<FunctionId> {
        self.functions
            .iter()
            .find(|(_, f)| f.name == name)
            .map(|(id, _)| id.clone())
    }

    fn declare_variable(
        &mut self,
        name: String,
        ty: Type,
    ) -> VariableId {
        let id = self.variable_id_gen.borrow_mut().next();
        let variable = Variable { name, ty, id };
        let is_shadowing = self.lookup_variable(&variable.name).is_some();
        self.variables.insert(id, variable);
        match self.current_local_scope() {
            Some(local_scope) => {
                local_scope.variables.insert(id);
            }
            None => {
                if !is_shadowing {
                    self.global_variables.insert(id);
                }
            }
        }
        id
    }

    fn current_local_scope(&mut self) -> Option<&mut LocalScope> {
        self.local_scopes.last_mut()
    }

    fn lookup_variable(&self, name: &str) -> Option<VariableId> {
        for local_scope in self.local_scopes.iter().rev() {
            // todo: handle shadowing
            for var in local_scope.variables.iter() {
                let var = self.get_variable(var);
                if var.name == name {
                    return Some(var.id);
                }
            }
            for var in self.global_variables.iter() {
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

    fn resolve_type_from_identifier(
        &self,
        token: &Token,
    ) -> Option<Type> {
        if let Some(ty) = Type::get_builtin_type(&token.span.literal) {
            return Some(ty);
        }
        // todo: handle structs
        None
    }

    fn enter_local_scope(&mut self) {
        self.local_scopes.push(LocalScope::new(self.variable_id_gen.clone()));
    }

    fn exit_local_scope(&mut self) {
        self.local_scopes.pop();
    }

    fn enter_function_scope(&mut self, function_id: FunctionId) {
        self.surrounding_function = Some(function_id);
        self.enter_local_scope();
        let function = self.get_function(&function_id);
        for parameter_id in function.parameters.clone() {
            self.current_local_scope()
                .unwrap()
                .variables
                .insert(parameter_id);
        }
    }

    fn exit_function_scope(&mut self) {
        self.surrounding_function = None;
        self.exit_local_scope();
    }
}

pub struct HIRGen {
    hir: HIR,
    diagnostics_bag: DiagnosticsBagCell,
}

impl HIRGen {
    pub fn new(
        diagnostics_bag: DiagnosticsBagCell,
    ) -> Self {
        Self {
            diagnostics_bag,
            hir: HIR::new(),
        }
    }

    pub fn gen(mut self, ast: &Ast) -> HIR {
        // todo: handle top level statements
        self.gather_global_symbols(ast);
        self.gen_function_bodies(ast);
        self.hir
    }

    fn gather_global_symbols(&mut self, ast: &Ast) {
        let mut visitor = HIRGlobalSymbolGatherer {
            diagnostics_bag: self.diagnostics_bag.clone(),
            hir_gen: self,
            ast,
            global_initializers: Vec::new(),
        };
        ast.visit(&mut visitor);
        for (id, initializer) in visitor.global_initializers {
            self.hir.globals.push(HIRGlobal::Variable {
                initializer,
                id,
            });
        }
    }

    fn gen_function_bodies(&mut self, ast: &Ast) {
        for statement in &ast.statements {
            match &statement.kind {
                ASTStatementKind::FuncDecl(stmt) => {
                    let body = &stmt.body;
                    if let Some(body) = body {
                        let function_id = self.hir.scope.lookup_function(&stmt.identifier.span.literal).expect(format!("ICE: function {} not found", stmt.identifier.span.literal).as_str());
                        self.hir.scope.enter_function_scope(function_id);
                        for stmt in body {
                            let stmt = self.gen_statement(stmt);
                            self.hir.push_stmt(stmt, function_id);
                        }
                        self.hir.scope.exit_function_scope();
                    }
                }

                _ => {}
            }
        }
    }

    fn gen_statements(&mut self, stmt: &ASTStatement) -> Vec<HIRStatement> {
        self.hir.scope.enter_local_scope();
        let stmts = match &stmt.kind {
            ASTStatementKind::Block(block) => {
                block.statements.iter().map(|stmt| self.gen_statement(stmt)).collect()
            }
            _ => {
                let stmt = self.gen_statement(stmt);
                vec![stmt]
            }
        };
        self.hir.scope.exit_local_scope();
        stmts
    }

    fn gen_statement(&mut self, stmt: &ASTStatement) -> HIRStatement {
        let kind = match &stmt.kind {
            ASTStatementKind::Expression(expr) => {
                let expr = self.gen_expression(expr);
                HIRStatementKind::Expression(HIRExpressionStatement {
                    expression: expr,
                })
            }
            ASTStatementKind::Let(stmt) => {
                HIRStatementKind::VariableDeclaration(common::declare_variable(self, &stmt))
            }
            ASTStatementKind::If(stmt) => {
                let condition = self.gen_expression(&stmt.condition);
                let then_branch = self.gen_statements(&stmt.then_branch);
                let else_branch = stmt.else_branch.as_ref().map(|branch| self.gen_statements(&branch.else_statement));
                HIRStatementKind::If(HIRIfStatement {
                    condition,
                    then: then_branch,
                    else_: else_branch,
                })
            }
            ASTStatementKind::Block(_) => {
                let statements = self.gen_statements(stmt);
                HIRStatementKind::Block(HIRBlockStatement { statements })
            }
            ASTStatementKind::While(stmt) => {
                let condition = self.gen_expression(&stmt.condition);
                let body = self.gen_statements(&stmt.body);
                HIRStatementKind::While(HIRWhileStatement { condition, body })
            }
            ASTStatementKind::FuncDecl(_) => {
                unreachable!("ICE: function declarations should be handled in gen_function_bodies")
            }
            ASTStatementKind::Return(return_stmt) => {
                let expression = return_stmt.return_value.as_ref().map(|expr| self.gen_expression(expr));
                let expression = expression.unwrap_or(HIRExpression {
                    kind: HIRExpressionKind::Void,
                    ty: Type::Void,
                    span: stmt.span(),
                });
                match &self.hir.scope.surrounding_function {
                    None => {
                        self.diagnostics_bag.borrow_mut().report_cannot_return_outside_function(&return_stmt.return_keyword);
                    }
                    Some(function) => {
                        let function = self.hir.scope.get_function(function);
                        self.ensure_type_match(&expression.span, &expression.ty, &function.return_type);
                    }
                }
                HIRStatementKind::Return(HIRReturnStatement {
                    expression
                })
            }
        };
        HIRStatement { kind, span: stmt.span() }
    }

    fn gen_expression(&mut self, expr: &ASTExpression) -> HIRExpression {
        let (kind, ty) = match &expr.kind {
            ASTExpressionKind::Number(expr) => {
                let ty = Type::I64;
                let expr = HIRExpressionKind::Literal(HIRLiteralExpression {
                    value: HIRLiteralValue::Integer(expr.number),
                });
                (expr, ty)
            }
            ASTExpressionKind::String(expr) => {
                let ty = Type::StringSlice();
                let expr = HIRExpressionKind::Literal(HIRLiteralExpression {
                    value: HIRLiteralValue::String(expr.string.to_string()),
                });
                (expr, ty)
            }
            ASTExpressionKind::Binary(expr) => {
                let left = self.gen_expression(&expr.left);
                let right = self.gen_expression(&expr.right);
                let op = HIRBinaryOperator::from(&expr.operator);
                let ty = self.resolve_bin_op_ty(&left.span, &left.ty, &right.ty, &op);
                let expr = HIRExpressionKind::Binary(HIRBinaryExpression {
                    left: Box::new(left),
                    right: Box::new(right),
                    op,
                });
                (expr, ty)
            }
            ASTExpressionKind::Unary(expr) => {
                let operand = self.gen_expression(&expr.operand);
                let op = HIRUnaryOperator::from(&expr.operator);
                let ty = self.resolve_un_op_ty(&operand.span, &operand.ty, &op);
                let expr = HIRExpressionKind::Unary(HIRUnaryExpression {
                    operand: Box::new(operand),
                    op,
                });
                (expr, ty)
            }
            ASTExpressionKind::Parenthesized(expr) => {
                let inner = self.gen_expression(&expr.expression);
                let ty = inner.ty.clone();
                let expr = HIRExpressionKind::Parenthesized(HIRParenthesizedExpression {
                    expression: Box::new(inner),
                });
                (expr, ty)
            }
            ASTExpressionKind::Identifier(expr) => {
                // todo: for now we assume that the identifier references a variable
                let variable_id = self.hir.scope.lookup_variable(&expr.identifier.span.literal);
                match variable_id {
                    Some(variable_id) => {
                        let variable = self.hir.scope.get_variable(&variable_id);
                        let ty = variable.ty.clone();
                        let expr = HIRExpressionKind::Variable(HIRVariableExpression {
                            variable_id,
                        });
                        (expr, ty)
                    }
                    None => {
                        self.diagnostics_bag.borrow_mut().report_undeclared_variable(&expr.identifier);
                        (HIRExpressionKind::Void, Type::Error)
                    }
                }
            }
            ASTExpressionKind::Assignment(expr) => {
                let hir_expr = self.gen_expression(&expr.assignee);
                let (target, ty) = match hir_expr.kind {
                    HIRExpressionKind::Variable(variable_expr) => {
                        let variable = self.hir.scope.get_variable(&variable_expr.variable_id);
                        (HIRAssignmentTargetKind::Variable(variable_expr.variable_id), variable.ty.clone())
                    }
                    HIRExpressionKind::Deref(deref_expr) => {
                        (HIRAssignmentTargetKind::Deref(Box::new(*deref_expr.expression)), hir_expr.ty.deref().unwrap_or(Type::Error))
                    }
                    _ => {
                        self.diagnostics_bag.borrow_mut().report_cannot_assign_to(&expr.assignee.span());
                        (HIRAssignmentTargetKind::Error, Type::Error)
                    }
                };
                let value = self.gen_expression(&expr.expression);
                let value_ty = value.ty.clone();
                self.ensure_type_match(&value.span, &value.ty, &ty);
                let expr = HIRExpressionKind::Assignment(HIRAssignmentExpression {
                    target:HIRAssignmentTarget {
                        kind: target,
                        span: expr.assignee.span(),
                    },
                    value: Box::new(value),
                });
                (expr, value_ty)
            }
            ASTExpressionKind::Boolean(expr) => {
                let ty = Type::Bool;
                let expr = HIRExpressionKind::Literal(HIRLiteralExpression {
                    value: HIRLiteralValue::Boolean(expr.value),
                });
                (expr, ty)
            }
            ASTExpressionKind::Call(expr) => {
                let callee = self.resolve_callee(&expr.callee);
                let arguments: Vec<HIRExpression> = expr.arguments.iter().map(|arg| self.gen_expression(arg)).collect();
                let ty = match callee {
                    HIRCallee::Function(id) => {
                        let function = self.hir.scope.get_function(&id);
                        if function.parameters.len() != arguments.len() {
                            self.diagnostics_bag.borrow_mut().report_invalid_argument_count(&expr.callee.span(), function.parameters.len(), arguments.len());
                        }
                        for (i, arg) in arguments.iter().enumerate() {
                            let param = &function.parameters.get(i);
                            if let Some(param) = param {
                                let param = self.hir.scope.get_variable(&param);
                                self.ensure_type_match(&arg.span, &param.ty, &arg.ty);
                            }
                        }
                        function.return_type.clone()
                    }
                    HIRCallee::Undeclared => {
                        Type::Error
                    }
                    HIRCallee::Invalid => {
                        Type::Error
                    }
                };
                let expr = HIRExpressionKind::Call(HIRCallExpression {
                    callee,
                    arguments,
                });
                (expr, ty)
            }
            ASTExpressionKind::Error(_) => {
                unimplemented!()
            }
            ASTExpressionKind::Ref(expr) => {
                let inner = self.gen_expression(&expr.expr);
                let ty = Type::Ptr(Box::new(inner.ty.clone()));
                let expr = HIRExpressionKind::Ref(HIRRefExpression {
                    expression: Box::new(inner),
                });
                (expr, ty)
            }
            ASTExpressionKind::Deref(expr) => {
                let inner = self.gen_expression(&expr.expr);
                let ty = match &inner.ty {
                    Type::Ptr(ty) => {
                        *ty.clone()
                    }
                    _ => {
                        self.diagnostics_bag.borrow_mut().report_cannot_deref(&expr.expr.span());
                        Type::Error
                    }
                };
                let expr = HIRExpressionKind::Deref(HIRDerefExpression {
                    expression: Box::new(inner),
                });
                (expr, ty)
            }
            ASTExpressionKind::Char(expr) => {
                let ty = Type::Char;
                let expr = HIRExpressionKind::Literal(HIRLiteralExpression {
                    value: HIRLiteralValue::Char(expr.value),
                });
                (expr, ty)
            }
        };
        HIRExpression {
            kind,
            ty,
            span: expr.span(),
        }
    }

    fn resolve_callee(&self, callee: &ASTExpression) -> HIRCallee {
        match &callee.kind {
            ASTExpressionKind::Identifier(expr) => {
                let function_id = self.hir.scope.lookup_function(&expr.identifier.span.literal);
                match function_id {
                    Some(function_id) => {
                        HIRCallee::Function(function_id)
                    }
                    None => {
                        self.diagnostics_bag.borrow_mut().report_undeclared_function(&expr.identifier);
                        HIRCallee::Undeclared
                    }
                }
            }
            _ => {
                self.diagnostics_bag.borrow_mut().report_invalid_callee(&callee.span());
                HIRCallee::Invalid
            }
        }
    }

    fn resolve_type(&mut self, ty_syntax: &TypeSyntax) -> Type {
        if let Some(ty) = self.hir.scope.resolve_type_from_identifier(&ty_syntax.name) {
            return if ty_syntax.star.is_some() {
                Type::Ptr(Box::new(ty))
            } else {
                ty
            }
        }
        self.diagnostics_bag.borrow_mut().report_undeclared_type(&ty_syntax.name);
        Type::Error
    }

    fn ensure_type_match(&self, actual_span: &TextSpan, actual: &Type, expected: &Type) -> Type {
        if actual.is_assignable_to(expected) {
            return expected.clone();
        }
        self.diagnostics_bag.borrow_mut().report_type_mismatch(actual_span, actual, expected);
        expected.clone()
    }

    fn resolve_bin_op_ty(&self, span: &TextSpan, left: &Type, right: &Type, op: &HIRBinaryOperator) -> Type {
        let definitions = op.definitions();
        for definition in definitions {
            if left.is_assignable_to(&definition.left) && right.is_assignable_to(&definition.right) {
                return definition.result;
            }
        }
        self.diagnostics_bag.borrow_mut().report_binary_operator_mismatch(span, left, right);
        Type::Error
    }

    fn resolve_un_op_ty(&self, span: &TextSpan, operand: &Type, op: &HIRUnaryOperator) -> Type {
        let definitions = op.definitions();
        for definition in definitions {
            if operand.is_assignable_to(&definition.operand) {
                return definition.result;
            }
        }
        self.diagnostics_bag.borrow_mut().report_unary_operator_mismatch(span, operand);
        Type::Error
    }
}

// todo: remove this and gather symbols during parsing
struct HIRGlobalSymbolGatherer<'a> {
    hir_gen: &'a mut HIRGen,
    ast: &'a Ast,
    diagnostics_bag: DiagnosticsBagCell,
    global_initializers: Vec<(VariableId, HIRExpression)>,
}

impl ASTVisitor for HIRGlobalSymbolGatherer<'_> {
    fn get_ast(&self) -> &Ast {
        self.ast
    }


    fn visit_func_decl_statement(&mut self, func_decl_statement: &ASTFuncDeclStatement) {
        common::declare_function(self.hir_gen, self.diagnostics_bag.clone(), func_decl_statement);
    }

    fn visit_let_statement(&mut self, let_statement: &ASTLetStatement, statement: &ASTStatement) {
        let variable_declaration_stmt = common::declare_variable(self.hir_gen, let_statement);
        self.global_initializers.push((variable_declaration_stmt.variable_id, variable_declaration_stmt.initializer));
    }

    fn visit_char_expression(&mut self, char_expression: &ASTCharExpression, expr: &ASTExpression) {

    }

    fn visit_deref_expression(&mut self, deref_expression: &ASTDerefExpression) {
    }

    fn visit_ref_expression(&mut self, ref_expression: &ASTRefExpression) {
    }

    fn visit_string_expression(&mut self, string_expression: &ASTStringExpression, expr: &ASTExpression) {}

    fn visit_identifier_expression(&mut self, variable_expression: &ASTIdentifierExpression, expr: &ASTExpression) {}

    fn visit_number_expression(&mut self, number: &ASTNumberExpression, expr: &ASTExpression) {}

    fn visit_boolean_expression(&mut self, boolean: &ASTBooleanExpression, expr: &ASTExpression) {}

    fn visit_error(&mut self, span: &TextSpan) {}

    fn visit_unary_expression(&mut self, unary_expression: &ASTUnaryExpression, expr: &ASTExpression) {}
}

