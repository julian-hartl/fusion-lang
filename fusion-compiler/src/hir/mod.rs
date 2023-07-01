use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use fusion_compiler::{idx, Result};
use fusion_compiler::Idx;

use crate::ast;
use crate::ast::{Ast, FunctionDeclaration, ItemKind, ModuleDeclaration, QualifiedIdentifier, StructDeclaration, TypeSyntax};
use crate::ast::expr::{BlockExpr, Expr, ExprKind};
use crate::ast::lexer::token::{Token, TokenKind};
use crate::ast::stmt::{LetStmt, Stmt, StmtKind};
use crate::ast::visitor::ASTVisitor;
use crate::compilation::SourceTree;
use crate::diagnostics::DiagnosticsBagCell;
use crate::modules::scopes::{GlobalScope, GlobalScopeCell, SymbolLookupResult};
use crate::modules::symbols::{Function, ModuleIdx};
use crate::text::span::TextSpan;
use crate::typings::{IntSize, Type};

mod visitor;
mod visualization;


pub struct HIR {
    pub function_bodies: HashMap<FunctionIdx, Vec<HIRStatement>>,
    pub structs: Vec<HIRStruct>,
    pub globals: Vec<HIRGlobal>,
}

pub enum HIRGlobal {
    Variable {
        id: VariableIdx,
        initializer: HIRExpr,
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

#[derive(Debug)]
pub struct HIRStatement {
    pub kind: HIRStatementKind,
    pub span: TextSpan,
}

#[derive(Debug)]
pub enum HIRStatementKind {
    Return(HIRReturnStatement),
    Expression(HIRExpressionStatement),
    VariableDeclaration(HIRVariableDeclarationStatement),
}

#[derive(Debug)]
pub struct HIRBlockExpr {
    pub statements: Vec<HIRStatement>,
}

#[derive(Debug)]
pub struct HIRReturnStatement {
    pub expression: HIRExpr,
}

#[derive(Debug)]
pub struct HIRExpressionStatement {
    pub expression: HIRExpr,
}

#[derive(Debug)]
pub struct HIRVariableDeclarationStatement {
    pub variable_id: VariableIdx,
    pub initializer: HIRExpr,
}

#[derive(Debug)]
pub struct HIRIfExpr {
    pub condition: Box<HIRExpr>,
    pub then: Box<HIRExpr>,
    pub else_: Option<Box<HIRExpr>>,
}

#[derive(Debug)]
pub struct HIRWhileExpr {
    pub condition: Box<HIRExpr>,
    pub body: Box<HIRExpr>,
}

#[derive(Debug)]
pub struct HIRExpr {
    pub kind: HIRExprKind,
    pub span: TextSpan,
    pub ty: Type,
}

#[derive(Debug)]
pub enum HIRExprKind {
    Literal(HIRLiteralExpression),
    Variable(HIRVariableExpression),
    Assignment(HIRAssignmentExpression),
    Binary(HIRBinaryExpression),
    Unary(HIRUnaryExpression),
    Call(HIRCallExpression),
    FieldAccess(HIRFieldAccessExpression),
    Ref(HIRRefExpression),
    Deref(HIRDerefExpression),
    Cast(HIRCastExpression),
    StructInit(HIRStructInitExpression),
    Index(HIRIndexExpression),
    If(HIRIfExpr),
    While(HIRWhileExpr),
    Block(HIRBlockExpr),
    Void,
}

#[derive(Debug)]
pub struct HIRIndexExpression {
    pub target: Box<HIRExpr>,
    pub index: Box<HIRExpr>,
}

#[derive(Debug)]
pub struct HIRStructInitExpression {
    pub struct_id: StructIdx,
    pub fields: Vec<HIRStructInitField>,
}

#[derive(Debug)]
pub struct HIRStructInitField {
    pub field_id: FieldIdx,
    pub value: HIRExpr,
}

#[derive(Debug)]
pub struct HIRCastExpression {
    pub expression: Box<HIRExpr>,
    pub ty: Type,
}

#[derive(Debug)]
pub struct HIRRefExpression {
    pub expression: Box<HIRExpr>,
}

#[derive(Debug)]
pub struct HIRDerefExpression {
    pub target: Box<HIRExpr>,
}

#[derive(Debug)]
pub struct HIRParenthesizedExpression {
    pub expression: Box<HIRExpr>,
}

#[derive(Debug)]
pub struct HIRLiteralExpression {
    pub value: HIRLiteralValue,
}

#[derive(Debug)]
pub enum HIRLiteralValue {
    Integer(IntegerLiteralValue),
    Boolean(bool),
    String(String),
    Char(char),
}

#[derive(Debug)]
pub enum IntegerLiteralValue {
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    ISize(isize),
}

impl Display for IntegerLiteralValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IntegerLiteralValue::I8(value) => write!(f, "{}", value),
            IntegerLiteralValue::I16(value) => write!(f, "{}", value),
            IntegerLiteralValue::I32(value) => write!(f, "{}", value),
            IntegerLiteralValue::I64(value) => write!(f, "{}", value),
            IntegerLiteralValue::ISize(value) => write!(f, "{}", value),
        }
    }
}

#[derive(Debug)]
pub struct HIRVariableExpression {
    pub variable_id: VariableIdx,
}

#[derive(Debug)]
pub struct HIRAssignmentExpression {
    pub target: Box<HIRExpr>,
    pub value: Box<HIRExpr>,
}

#[derive(Debug)]
pub struct HIRBinaryExpression {
    pub left: Box<HIRExpr>,
    pub op: BinOperator,
    pub right: Box<HIRExpr>,
}

impl Display for BinOperator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let op = match self {
            BinOperator::Add => "+",
            BinOperator::Subtract => "-",
            BinOperator::Multiply => "*",
            BinOperator::Divide => "/",
            BinOperator::Equals => "==",
            BinOperator::NotEquals => "!=",
            BinOperator::LessThan => "<",
            BinOperator::LessThanOrEqual => "<=",
            BinOperator::GreaterThan => ">",
            BinOperator::GreaterThanOrEqual => ">=",
            BinOperator::BitwiseAnd => "&",
            BinOperator::BitwiseOr => "|",
            BinOperator::BitwiseXor => "^",
            BinOperator::Modulo => "%",
            BinOperator::LogicalAnd => "&&",
        };
        write!(f, "{}", op)
    }
}


#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BinOperator {
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
    LogicalAnd,
}

impl BinOperator {
    pub fn get_type_table(&self) -> Vec<(Type, Type, Type)> {
        match self {
            BinOperator::Add => {
                let int_types = Self::get_arithmetic_types();
                Vec::from(int_types)
            }
            BinOperator::Subtract => {
                let int_types = Self::get_arithmetic_types();
                Vec::from(int_types)
            }
            BinOperator::Multiply => {
                let int_types = Self::get_arithmetic_types();
                Vec::from(int_types)
            }
            BinOperator::Divide => {
                let int_types = Self::get_arithmetic_types();
                Vec::from(int_types)
            }
            BinOperator::Modulo => {
                let int_types = Self::get_arithmetic_types();
                Vec::from(int_types)
            }
            BinOperator::Equals => {
                let built_in_types = Type::get_built_in_types();
                let mut types = Vec::with_capacity(built_in_types.len());
                for ty in built_in_types {
                    types.push((ty.clone(), ty.clone(), Type::Bool));
                }
                types
            }
            BinOperator::NotEquals => {
                let built_in_types = Type::get_built_in_types();
                let mut types = Vec::with_capacity(built_in_types.len());
                for ty in built_in_types {
                    types.push((ty.clone(), ty.clone(), Type::Bool));
                }
                types
            }
            BinOperator::LessThan => {
                Self::get_number_comparison_types()
            }
            BinOperator::LessThanOrEqual => {
                Self::get_number_comparison_types()
            }
            BinOperator::GreaterThan => {
                Self::get_number_comparison_types()
            }
            BinOperator::GreaterThanOrEqual => {
                Self::get_number_comparison_types()
            }
            BinOperator::BitwiseAnd => {
                let int_types = Self::get_arithmetic_types();
                Vec::from(int_types)
            }
            BinOperator::BitwiseOr => {
                let int_types = Self::get_arithmetic_types();
                Vec::from(int_types)
            }
            BinOperator::BitwiseXor => {
                let int_types = Self::get_arithmetic_types();
                Vec::from(int_types)
            }
            BinOperator::LogicalAnd => {
                vec![
                    (Type::Bool, Type::Bool, Type::Bool),
                ]
            }
        }
    }

    fn get_number_comparison_types() -> Vec<(Type, Type, Type)> {
        let int_types = Type::get_integer_types();
        let mut types = Vec::with_capacity(int_types.len());
        for ty in int_types {
            types.push((ty.clone(), ty.clone(), Type::Bool));
        }
        types
    }

    fn get_arithmetic_types() -> [(Type, Type, Type); 6] {
        [
            (Type::Integer(IntSize::I8), Type::Integer(IntSize::I8), Type::Integer(IntSize::I8)),
            (Type::Integer(IntSize::I16), Type::Integer(IntSize::I16), Type::Integer(IntSize::I16)),
            (Type::Integer(IntSize::I32), Type::Integer(IntSize::I32), Type::Integer(IntSize::I32)),
            (Type::Integer(IntSize::I64), Type::Integer(IntSize::I64), Type::Integer(IntSize::I64)),
            (Type::Integer(IntSize::ISize), Type::Integer(IntSize::ISize), Type::Integer(IntSize::ISize)),
            (Type::Char, Type::Char, Type::Char),
        ]
    }
}


impl From<&ast::expr::BinOperator> for BinOperator {
    fn from(op: &ast::expr::BinOperator) -> Self {
        match op.kind {
            ast::expr::BinOperatorKind::Plus => BinOperator::Add,
            ast::expr::BinOperatorKind::Minus => BinOperator::Subtract,
            ast::expr::BinOperatorKind::Multiply => BinOperator::Multiply,
            ast::expr::BinOperatorKind::Divide => BinOperator::Divide,
            ast::expr::BinOperatorKind::Equals => BinOperator::Equals,

            ast::expr::BinOperatorKind::NotEquals => BinOperator::NotEquals,
            ast::expr::BinOperatorKind::LessThan => BinOperator::LessThan,
            ast::expr::BinOperatorKind::LessThanOrEqual => BinOperator::LessThanOrEqual,
            ast::expr::BinOperatorKind::GreaterThan => BinOperator::GreaterThan,

            ast::expr::BinOperatorKind::GreaterThanOrEqual => BinOperator::GreaterThanOrEqual,
            ast::expr::BinOperatorKind::BitwiseAnd => BinOperator::BitwiseAnd,
            ast::expr::BinOperatorKind::BitwiseOr => BinOperator::BitwiseOr,
            ast::expr::BinOperatorKind::BitwiseXor => BinOperator::BitwiseXor,

            ast::expr::BinOperatorKind::Power => unimplemented!(),
            ast::expr::BinOperatorKind::Modulo => BinOperator::Modulo,
            ast::expr::BinOperatorKind::LogicalAnd => BinOperator::LogicalAnd,
        }
    }
}

#[derive(Debug)]
pub struct HIRUnaryExpression {
    pub op: UnOperator,
    pub operand: Box<HIRExpr>,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOperator {
    Negate,
    BitwiseNot,
}

impl UnOperator {
    pub fn get_type_table(&self) -> Vec<(Type, Type)> {
        match self {
            UnOperator::Negate => {
                let int_types = Type::get_integer_types();
                let mut types = Vec::with_capacity(int_types.len());
                for ty in int_types {
                    types.push((ty.clone(), ty.clone()));
                }
                types
            }
            UnOperator::BitwiseNot => {
                let int_types = Type::get_integer_types();
                let mut types = Vec::with_capacity(int_types.len());
                for ty in int_types {
                    types.push((ty.clone(), ty.clone()));
                }
                types
            }
        }
    }
}

impl Display for UnOperator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            UnOperator::Negate => "-",
            UnOperator::BitwiseNot => "~",
        };
        write!(f, "{}", s)
    }
}

impl From<&ast::expr::UnOperator> for UnOperator {
    fn from(op: &ast::expr::UnOperator) -> Self {
        match op.kind {
            ast::expr::UnOperatorKind::Minus => UnOperator::Negate,
            ast::expr::UnOperatorKind::BitwiseNot => UnOperator::BitwiseNot,
        }
    }
}


#[derive(Debug)]
pub struct HIRCallExpression {
    pub callee: HIRCallee,
    pub args: Vec<HIRExpr>,
}

#[derive(Debug)]
pub enum HIRCallee {
    Function(FunctionIdx),
    Undeclared(String),
    Invalid,
}

#[derive(Debug)]
pub struct HIRFieldAccessExpression {
    pub target: Box<HIRExpr>,
    pub field_id: FieldIdx,
}

mod common {
    use fusion_compiler::Result;

    use crate::ast::FunctionDeclaration;
    use crate::ast::lexer::token::Token;
    use crate::ast::stmt::{LetStmt, ParameterSyntax};
    use crate::diagnostics::{DiagnosticsBag, DiagnosticsBagCell};
    use crate::hir::{HIRGen, HIRStatementKind, HIRVariableDeclarationStatement, VariableIdx};
    use crate::hir;
    use crate::modules::symbols::{FunctionModifier, Variable};
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

    pub fn declare_function(hir_gen: &mut HIRGen, diagnostics_bag: DiagnosticsBagCell, func_decl_statement: &FunctionDeclaration) {
        let name = func_decl_statement.identifier.span.literal.clone();
        let parameters = func_decl_statement.parameters.iter().map(|param| {
            let name = param.identifier.span.literal.clone();
            let ty = hir_gen.resolve_type_syntax(&param.type_annotation.ty);
            hir_gen.scope.borrow_mut().declare_variable(name.clone(), ty, param.mut_token.is_some())
        }).collect();
        let return_type = match func_decl_statement.return_type.as_ref() {
            Some(return_type) => {
                hir_gen.resolve_type_syntax(&return_type.ty)
            }
            None => {
                Type::Void
            }
        };
        if let Err(_) = hir_gen.scope.borrow_mut().declare_function(
            name,
            parameters,
            return_type,
            func_decl_statement.modifier_tokens.iter().map(|token| resolve_func_modifier(token, diagnostics_bag.clone())).filter_map(|m| m.ok()).collect(),
        ) {
            diagnostics_bag.borrow_mut().report_function_already_declared(&func_decl_statement.identifier);
        }
    }

    pub fn declare_variable(hir_gen: &mut HIRGen, stmt: &LetStmt) -> HIRVariableDeclarationStatement {
        let static_type = stmt.type_annotation.as_ref().map(|ty| hir_gen.resolve_type_syntax(&ty.ty));
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
        let variable_id = hir_gen.scope.borrow_mut().declare_variable(
            stmt.identifier.span.literal.clone(),
            ty,
            stmt.mut_token.is_some(),
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
            structs: Vec::new(),
            globals: Vec::new(),
        }
    }

    fn push_stmt(&mut self, stmt: HIRStatement, function_id: FunctionIdx) {
        self.function_bodies
            .entry(function_id)
            .or_insert_with(Vec::new)
            .push(stmt);
    }

    pub fn functions<'a>(&self, scope: &'a GlobalScope) -> Vec<(&'a Function, Option<&Vec<HIRStatement>>)> {
        scope.functions().indexed_iter().map(|(function_idx, function)| {
            let body = self.function_bodies.get(&function_idx);
            (function, body)
        }).collect()
    }

    pub fn visualize(&self, scope: GlobalScopeCell) {
        let visualizer = visualization::HIRVisualizer::new(self, scope);
        let output = visualizer.visualize();
        println!("{}", output);
    }
}


idx!(VariableIdx);

idx!(FunctionIdx);

idx!(StructIdx);

idx!(FieldIdx);

pub struct HIRGen {
    hir: HIR,
    diagnostics_bag: DiagnosticsBagCell,
    scope: GlobalScopeCell,
}

impl HIRGen {
    pub fn new(
        diagnostics_bag: DiagnosticsBagCell,
        scope: GlobalScopeCell,
    ) -> Self {
        Self {
            diagnostics_bag,
            hir: HIR::new(),
            scope,
        }
    }

    pub fn gen(mut self, tree: &SourceTree) -> HIR {
        // todo: handle top level statements
        for (module_id, (ast, _)) in tree.asts.iter() {
            self.scope.borrow_mut().set_current_module(*module_id);
            self.diagnostics_bag.borrow_mut().set_current_module(*module_id);
            self.gather_global_symbols(*module_id, ast);
        }
        for (module_id, (ast, _)) in tree.asts.iter() {
            self.gen_function_bodies(*module_id, ast);
        }
        self.hir
    }


    fn gather_global_symbols(&mut self, module_id: ModuleIdx, ast: &Ast) {
        self.scope.borrow_mut().set_current_module(module_id);
        self.diagnostics_bag.borrow_mut().set_current_module(module_id);
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

    fn gen_function_bodies(&mut self, module_id: ModuleIdx, ast: &Ast) {
        self.scope.borrow_mut().set_current_module(module_id);
        self.diagnostics_bag.borrow_mut().set_current_module(module_id);
        for statement in &ast.items {
            match &statement.kind {
                ItemKind::FunctionDeclaration(decl) => {
                    let body = &decl.body;
                    if let Some(body) = body {
                        let function_id = self.scope.borrow_mut().lookup_function_unqualified(&decl.identifier.span.literal).expect(format!("ICE: function {} not found", decl.identifier.span.literal).as_str());
                        self.scope.borrow_mut().enter_function_scope(function_id);
                        for stmt in body.stmts.iter() {
                            let stmt = self.gen_statement(stmt);
                            self.hir.push_stmt(stmt, function_id);
                        }
                        self.scope.borrow_mut().exit_function_scope();
                    }
                }

                _ => {}
            }
        }
    }

    fn gen_block(&mut self, block: &BlockExpr) -> HIRExpr {
        self.scope.borrow_mut().enter_local_scope();
        let stmts: Vec<HIRStatement> = block.stmts.iter().map(|stmt| self.gen_statement(stmt)).collect();
        self.scope.borrow_mut().exit_local_scope();
        let ty = stmts
            .last()
            .map(|stmt| match &stmt.kind {
                HIRStatementKind::Expression(expr) => expr.expression.ty.clone(),
                _ => Type::Void,
            })
            .unwrap_or(Type::Void);
        HIRExpr {
            kind: HIRExprKind::Block(HIRBlockExpr { statements: stmts }),
            ty,
            span: block.span().clone(),
        }
    }

    fn gen_statement(&mut self, stmt: &Stmt) -> HIRStatement {
        let kind = match &stmt.kind {
            StmtKind::Expr(expr) => {
                let expr = self.gen_expression(expr);
                HIRStatementKind::Expression(HIRExpressionStatement {
                    expression: expr,
                })
            }
            StmtKind::Let(stmt) => {
                HIRStatementKind::VariableDeclaration(common::declare_variable(self, &stmt))
            }
            StmtKind::Return(return_stmt) => {
                let expression = return_stmt.expr.as_ref().map(|expr| self.gen_expression(expr));
                let expression = expression.unwrap_or(HIRExpr {
                    kind: HIRExprKind::Void,
                    ty: Type::Void,
                    span: stmt.span(),
                });
                let scope = self.scope.borrow();
                match &scope.get_surrounding_function() {
                    None => {
                        self.diagnostics_bag.borrow_mut().report_cannot_return_outside_function(&return_stmt.return_keyword);
                    }
                    Some(function) => {
                        let function = scope.get_function(function);
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

    fn gen_expression(&mut self, expr: &Expr) -> HIRExpr {
        let (kind, ty) = match &expr.kind {
            ExprKind::Number(expr) => {
                let size = match &expr.size_specifier {
                    None => {
                        IntSize::I32
                    }
                    Some(size_specifier) => {
                        match size_specifier.span.literal.as_str() {
                            "i8" => IntSize::I8,
                            "i16" => IntSize::I16,
                            "i32" => IntSize::I32,
                            "i64" => IntSize::I64,
                            "isize" => IntSize::ISize,
                            _ => {
                                self.diagnostics_bag.borrow_mut().report_invalid_integer_size(&size_specifier.span);
                                IntSize::I32
                            }
                        }
                    }
                };
                let is_in_range = match size {
                    IntSize::I8 => expr.number <= i8::MAX as i64,
                    IntSize::I16 => expr.number <= i16::MAX as i64,
                    IntSize::I32 => expr.number <= i32::MAX as i64,
                    IntSize::I64 => expr.number <= i64::MAX as i64,
                    IntSize::ISize => expr.number <= isize::MAX as i64,
                };
                if !is_in_range {
                    self.diagnostics_bag.borrow_mut().report_integer_literal_out_of_range(&expr.token.span);
                }
                let ty = Type::Integer(size);
                let literal = match size {
                    IntSize::I8 => {
                        IntegerLiteralValue::I8(expr.number as i8)
                    }
                    IntSize::I16 => {
                        IntegerLiteralValue::I16(expr.number as i16)
                    }
                    IntSize::I32 => {
                        IntegerLiteralValue::I32(expr.number as i32)
                    }
                    IntSize::I64 => {
                        IntegerLiteralValue::I64(expr.number as i64)
                    }
                    IntSize::ISize => {
                        IntegerLiteralValue::ISize(expr.number as isize)
                    }
                };
                let expr = HIRExprKind::Literal(HIRLiteralExpression {
                    value: HIRLiteralValue::Integer(literal),
                });
                (expr, ty)
            }
            ExprKind::String(expr) => {
                let ty = Type::StringSlice(false);
                let expr = HIRExprKind::Literal(HIRLiteralExpression {
                    value: HIRLiteralValue::String(expr.string.to_raw_string()),
                });
                (expr, ty)
            }
            ExprKind::Binary(expr) => {
                let left = self.gen_expression(&expr.left);
                let right = self.gen_expression(&expr.right);
                let op = BinOperator::from(&expr.operator);
                let ty = self.resolve_bin_op_ty(&expr.operator.token.span, &left.ty, &right.ty, &op);
                let expr = HIRExprKind::Binary(HIRBinaryExpression {
                    left: Box::new(left),
                    right: Box::new(right),
                    op,
                });
                (expr, ty)
            }
            ExprKind::Unary(expr) => {
                let operand = self.gen_expression(&expr.operand);
                let op = UnOperator::from(&expr.operator);
                let ty = self.resolve_un_op_ty(&operand.span, &operand.ty, &op);
                let expr = HIRExprKind::Unary(HIRUnaryExpression {
                    operand: Box::new(operand),
                    op,
                });
                (expr, ty)
            }
            ExprKind::Parenthesized(expr) => {
                let inner = self.gen_expression(&expr.expr);
                let ty = inner.ty.clone();
                (inner.kind, ty)
            }
            ExprKind::Identifier(expr) => {
                // todo: for now we assume that the identifier references a variable
                if expr.identifier.is_qualified() {
                    self.diagnostics_bag.borrow_mut().report_unexpected_qualified_identifier(&expr.identifier);
                }
                let identifier = expr.identifier.get_unqualified_name();
                let variable_id = self.scope.borrow_mut().lookup_variable(&identifier.span.literal);
                match variable_id {
                    Some(variable_id) => {
                        let scope = self.scope.borrow();
                        let variable = scope.get_variable(&variable_id);
                        let ty = variable.ty.clone();
                        let expr = HIRExprKind::Variable(HIRVariableExpression {
                            variable_id,
                        });
                        (expr, ty)
                    }
                    None => {
                        self.diagnostics_bag.borrow_mut().report_undeclared_variable(&identifier);
                        (HIRExprKind::Void, Type::Error)
                    }
                }
            }
            ExprKind::Assignment(expr) => {
                let target = self.gen_expression(&expr.left);
                let ty = match &target.kind {
                    HIRExprKind::Variable(variable_expr) => {
                        let scope = self.scope.borrow();
                        let variable = scope.get_variable(&variable_expr.variable_id);
                        if !variable.is_mutable {
                            self.diagnostics_bag.borrow_mut().report_cannot_assign_twice_to_immutable_variable(&expr.left.span());
                        }
                        variable.ty.clone()
                    }
                    HIRExprKind::Deref(deref_expr) => {
                        dbg!(&target);
                        let is_mutable = match &deref_expr.target.ty {
                            Type::Ptr(_, is_mutable) => *is_mutable,
                            _ => unreachable!(),
                        };
                        if !is_mutable {
                            self.diagnostics_bag.borrow_mut().report_cannot_assign_to_immutable_pointer(&expr.left.span());
                        }
                        target.ty.clone()
                    }
                    HIRExprKind::FieldAccess(field_access_expr) => {
                        let is_mutable = self.is_expr_mutable(&field_access_expr.target);
                        if !is_mutable {
                            self.diagnostics_bag.borrow_mut().report_cannot_assign_to_immutable_field(&expr.left.span());
                        }
                        // todo: if we get some weird error messages here, it could be because we use FieldId = 0 as a placeholder for the error case.
                        let scope = self.scope.borrow();
                        let field = scope.get_field(&field_access_expr.field_id);
                        field.ty.clone()
                    }
                    HIRExprKind::Index(index_expr) => {
                        let is_mutable = match &index_expr.target.ty {
                            Type::Ptr(_, is_mutable) => *is_mutable,
                            _ => false
                        };
                        if !is_mutable {
                            self.diagnostics_bag.borrow_mut().report_cannot_assign_to_immutable_index(&expr.left.span());
                        }
                        target.ty.clone()
                    }
                    _ => {
                        self.diagnostics_bag.borrow_mut().report_cannot_assign_to(&expr.left.span());
                        Type::Error
                    }
                };
                let value = self.gen_expression(&expr.right);
                let value_ty = value.ty.clone();
                self.ensure_type_match(&value.span, &value.ty, &ty);
                let expr = HIRExprKind::Assignment(HIRAssignmentExpression {
                    target: Box::new(target),
                    value: Box::new(value),
                });
                (expr, value_ty)
            }
            ExprKind::Boolean(expr) => {
                let ty = Type::Bool;
                let expr = HIRExprKind::Literal(HIRLiteralExpression {
                    value: HIRLiteralValue::Boolean(expr.value),
                });
                (expr, ty)
            }
            ExprKind::Call(expr) => {
                let callee = self.resolve_callee(&expr.callee);
                let arguments: Vec<HIRExpr> = expr.arguments.iter().map(|arg| self.gen_expression(arg)).collect();
                let ty = match callee {
                    HIRCallee::Function(id) => {
                        let scope = self.scope.borrow();
                        let function = scope.get_function(&id);
                        if function.parameters.len() != arguments.len() {
                            self.diagnostics_bag.borrow_mut().report_invalid_argument_count(&expr.callee.span(), function.parameters.len(), arguments.len());
                        }
                        for (i, arg) in arguments.iter().enumerate() {
                            let param = &function.parameters.get(i);
                            if let Some(param) = param {
                                let param = scope.get_variable(&param);
                                self.ensure_type_match(&arg.span, &arg.ty, &param.ty);
                            }
                        }
                        function.return_type.clone()
                    }
                    HIRCallee::Undeclared(_) => {
                        Type::Error
                    }
                    HIRCallee::Invalid => {
                        Type::Error
                    }
                };
                let expr = HIRExprKind::Call(HIRCallExpression {
                    callee,
                    args: arguments,
                });
                (expr, ty)
            }
            ExprKind::Error(_) => {
                unimplemented!()
            }
            ExprKind::Ref(expr) => {
                let inner = self.gen_expression(&expr.expr);
                let (ty, expr) = self.ref_expression(&expr.expr.span(), expr.mut_token.is_some(), inner);
                (expr, ty)
            }
            ExprKind::Deref(deref_expr) => {
                let inner = self.gen_expression(&deref_expr.expr);
                let (ty, expr) = self.deref_expression(&expr.span(), inner);
                (expr, ty)
            }
            ExprKind::Char(expr) => {
                let ty = Type::Char;
                let expr = HIRExprKind::Literal(HIRLiteralExpression {
                    value: HIRLiteralValue::Char(expr.value),
                });
                (expr, ty)
            }
            ExprKind::Cast(expr) => {
                let inner = self.gen_expression(&expr.expr);
                let ty = self.resolve_type_syntax(&expr.ty);
                // todo: introduce cast matrix
                let expr = HIRExprKind::Cast(HIRCastExpression {
                    expression: Box::new(inner),
                    ty: ty.clone(),
                });
                (expr, ty)
            }
            ExprKind::MemberAccess(expr) => {
                let mut target = self.gen_expression(&expr.expr);
                let span = target.span.clone();
                if expr.access_operator.kind == TokenKind::Arrow {
                    let (ty, expr) = self.ref_expression(&span, true, target);
                    target = HIRExpr {
                        kind: expr,
                        ty,
                        span: span.clone(),
                    };
                }
                let (ty, member_id) = match &target.ty {
                    Type::Struct(id) => {
                        let scope = self.scope.borrow();
                        let struct_ = scope.get_struct(id);
                        let member = scope.lookup_field(&id, &expr.member.span.literal);
                        let ty = if let Some(member) = member {
                            scope.get_field(&member).ty.clone()
                        } else {
                            self.diagnostics_bag.borrow_mut().report_struct_has_no_member(&expr.member.span, &struct_.name.name);
                            Type::Error
                        };
                        (ty, member.unwrap_or(FieldIdx::new(0)))
                    }
                    _ => {
                        self.diagnostics_bag.borrow_mut().report_cannot_access_member_of_non_struct(&expr.expr.span(), &target.ty);
                        (Type::Error, FieldIdx::new(0))
                    }
                };
                let expr = HIRExprKind::FieldAccess(HIRFieldAccessExpression {
                    target: Box::new(target),
                    field_id: member_id,
                });
                (expr, ty)
            }
            ExprKind::StructInit(expr) => {
                let lookup_result = self.map_lookup_result(&expr.identifier, self.scope.borrow_mut().lookup_struct_qualified(&expr.identifier));
                let expr = match lookup_result {
                    Ok(struct_id) => {
                        match struct_id {
                            None => {
                                let span = &expr.identifier.get_unqualified_name().span;
                                self.diagnostics_bag.borrow_mut().report_undeclared_struct(&span, &span.literal);
                                None
                            }
                            Some(struct_id) => {
                                let mut fields = Vec::new();
                                for field in &expr.fields {
                                    let scope = self.scope.borrow();
                                    let field_id = scope.lookup_field(&struct_id, &field.identifier.span.literal);
                                    if let Some(field_id) = field_id {
                                        drop(scope);
                                        let value = self.gen_expression(&field.initializer);
                                        let scope = self.scope.borrow();
                                        let struct_field = scope.get_field(&field_id);
                                        self.ensure_type_match(&value.span, &value.ty, &struct_field.ty);
                                        fields.push(HIRStructInitField {
                                            field_id,
                                            value,
                                        });
                                    } else {
                                        let struct_ = scope.get_struct(&struct_id);

                                        self.diagnostics_bag.borrow_mut().report_struct_has_no_member(&field.identifier.span, struct_.name.unqualified_name());
                                    }
                                }
                                let scope_ref = self.scope.borrow();
                                let struct_ = scope_ref.get_struct(&struct_id);
                                for field in struct_.fields.iter() {
                                    if !fields.iter().any(|f| f.field_id == *field) {
                                        self.diagnostics_bag.borrow_mut().report_missing_field_in_struct(&expr.identifier.span(), struct_.name.unqualified_name(), &self.scope.borrow().get_field(&field).name);
                                    }
                                }
                                let expr = HIRExprKind::StructInit(HIRStructInitExpression {
                                    struct_id,
                                    fields,
                                });
                                Some((expr, Type::Struct(struct_id)))
                            }
                        }
                    }
                    Err(_) => { None }
                };
                match expr {
                    None => {
                        (HIRExprKind::Void, Type::Error)
                    }
                    Some(expr) => {
                        expr
                    }
                }
            }
            ExprKind::Index(expr) => {
                let target = self.gen_expression(&expr.target);
                let index = self.gen_expression(&expr.index);
                self.ensure_type_match(&index.span, &index.ty, &Type::Integer(IntSize::ISize));
                let ty = match &target.ty {
                    Type::Ptr(inner, _) => {
                        *inner.clone()
                    }
                    Type::Error => {
                        Type::Error
                    }
                    _ => {
                        self.diagnostics_bag.borrow_mut().report_cannot_index_type(&expr.target.span(), &target.ty);
                        Type::Error
                    }
                };
                let expr = HIRExprKind::Index(HIRIndexExpression {
                    target: Box::new(target),
                    index: Box::new(index),
                });
                (expr, ty)
            }
            ExprKind::Block(expr) => {
                return self.gen_block(expr);
            }
            ExprKind::While(expr) => {
                let condition = self.gen_expression(&expr.condition);
                let body = self.gen_block(&expr.body);
                // todo: check if we break out of the loop
                (HIRExprKind::While(HIRWhileExpr {
                    body: Box::new(body),
                    condition: Box::new(condition),
                }), Type::Void)
            }
            ExprKind::If(expr) => {
                let condition = self.gen_expression(&expr.condition);
                let then_branch = self.gen_block(&expr.then_branch);
                let else_branch = expr.else_branch.as_ref().map(|else_branch| self.gen_block(&else_branch.expr));
                let ty = match else_branch {
                    None => Type::Void,
                    Some(else_branch) => {
                        self.ensure_type_match(&else_branch.span, &else_branch.ty, &then_branch.ty);
                        then_branch.ty.clone()
                    }
                };
                (HIRExprKind::If(HIRIfExpr {
                    condition: Box::new(condition),
                    then: Box::new(then_branch),
                    else_: else_branch.map(|else_branch| Box::new(else_branch)),
                }), ty)
            }
        };
        HIRExpr {
            kind,
            ty,
            span: expr.span(),
        }
    }

    fn map_lookup_result<T>(&self, qualified_identifier: &QualifiedIdentifier, result: SymbolLookupResult<T>) -> std::result::Result<Option<T>, ()> {
        match result {
            SymbolLookupResult::ModuleNotFound { index } => {
                let not_found_module = &qualified_identifier.parts[index];
                self.diagnostics_bag.borrow_mut().report_module_not_found(&not_found_module.span);
                Err(())
            }

            SymbolLookupResult::SymbolNotFound => {
                Ok(None)
            }
            SymbolLookupResult::Found(symbol) => {
                Ok(Some(symbol))
            }
        }
    }

    fn ref_expression(&self, span: &TextSpan, is_mut: bool, inner: HIRExpr) -> (Type, HIRExprKind) {
        let ty = Type::Ptr(Box::new(inner.ty.clone()), is_mut);
        let expr = HIRExprKind::Ref(HIRRefExpression {
            expression: Box::new(inner),
        });
        (ty, expr)
    }

    fn deref_expression(&mut self, inner_span: &TextSpan, inner: HIRExpr) -> (Type, HIRExprKind) {
        let ty = match &inner.ty {
            Type::Ptr(ty, _) => {
                if **ty == Type::Void {
                    self.diagnostics_bag.borrow_mut().report_cannot_deref_void(inner_span);
                }
                *ty.clone()
            }
            _ => {
                self.diagnostics_bag.borrow_mut().report_cannot_deref(inner_span);
                Type::Error
            }
        };
        let expr = HIRExprKind::Deref(HIRDerefExpression {
            target: Box::new(inner),
        });
        (ty, expr)
    }

    fn is_expr_mutable(&self, expr: &HIRExpr) -> bool {
        match &expr.kind {
            HIRExprKind::Variable(expr) => {
                let scope = self.scope.borrow();
                let variable = scope.get_variable(&expr.variable_id);
                variable.is_mutable
            }
            HIRExprKind::Deref(_) => {
                match expr.ty {
                    Type::Ptr(_, is_mutable) => {
                        is_mutable
                    }
                    _ => {
                        false
                    }
                }
            }
            HIRExprKind::Index(expr) => {
                match expr.target.ty {
                    Type::Ptr(_, is_mutable) => {
                        is_mutable
                    }
                    _ => {
                        false
                    }
                }
            }
            _ => {
                false
            }
        }
    }

    fn resolve_callee(&mut self, callee: &Expr) -> HIRCallee {
        match &callee.kind {
            ExprKind::Identifier(expr) => {
                let lookup_result = self.scope.borrow_mut().lookup_function_qualified(&expr.identifier);
                let function_id = self.map_lookup_result(&expr.identifier, lookup_result);
                match function_id {
                    Ok(function_id) => {
                        match function_id {
                            Some(function_id) => {
                                HIRCallee::Function(function_id)
                            }
                            None => {
                                let identifier = expr.identifier.get_unqualified_name();
                                self.diagnostics_bag.borrow_mut().report_undeclared_function(&identifier);
                                HIRCallee::Undeclared(identifier.span.literal.clone())
                            }
                        }
                    }
                    Err(_) => {
                        // todo: we should not report errors anymore when doing this
                        HIRCallee::Invalid
                    }
                }
            }
            _ => {
                self.diagnostics_bag.borrow_mut().report_invalid_callee(&callee.span());
                HIRCallee::Invalid
            }
        }
    }

    fn resolve_type_syntax(&mut self, ty_syntax: &TypeSyntax) -> Type {
        if let Some(ty) = self.resolve_type_from_identifier(&ty_syntax.name) {
            return if let Some(ptr) = &ty_syntax.ptr {
                let mut ty = ty;
                for ptr in ptr.iter().rev() {
                    ty = Type::Ptr(Box::new(ty), ptr.mut_token.is_some());
                }
                ty
            } else {
                ty
            };
        }
        self.diagnostics_bag.borrow_mut().report_undeclared_type(ty_syntax.name.get_unqualified_name());
        Type::Error
    }

    pub fn resolve_type_from_identifier(
        &self,
        identifier: &QualifiedIdentifier,
    ) -> Option<Type> {
        if identifier.parts.len() == 1 {
            if let Some(ty) = Type::get_builtin_type(identifier.parts[0].span.literal.as_str()) {
                return Some(ty);
            }
        }
        let result: Option<StructIdx> = self.map_lookup_result(identifier, self.scope.borrow().lookup_struct_qualified(identifier)).ok().flatten();
        if let Some(id) = result {
            return Some(Type::Struct(id));
        }
        None
    }

    fn ensure_type_match<'a>(&self, actual_span: &TextSpan, actual: &'a Type, expected: &'a Type) -> &'a Type {
        if actual.is_assignable_to(expected) {
            return expected;
        }
        self.diagnostics_bag.borrow_mut().report_type_mismatch(actual_span, expected, actual);
        expected
    }

    fn resolve_bin_op_ty(&self, span: &TextSpan, left: &Type, right: &Type, op: &BinOperator) -> Type {
        let table = op.get_type_table();
        match table.into_iter().find(|(l, r, _)| left.is_assignable_to(l) && right.is_assignable_to(r)) {
            Some((_, _, ty)) => ty,
            None => {
                self.diagnostics_bag.borrow_mut().report_binary_operator_mismatch(span, left, right);
                Type::Error
            }
        }
    }

    fn resolve_un_op_ty(&self, span: &TextSpan, operand: &Type, op: &UnOperator) -> Type {
        let table = op.get_type_table();
        match table.into_iter().find(|(t, _)| operand.is_assignable_to(t)) {
            Some((_, ty)) => ty,
            None => {
                self.diagnostics_bag.borrow_mut().report_unary_operator_mismatch(span, operand);
                Type::Error
            }
        }
    }
}


// todo: remove this and gather symbols during parsing
struct HIRGlobalSymbolGatherer<'a> {
    hir_gen: &'a mut HIRGen,
    ast: &'a Ast,
    diagnostics_bag: DiagnosticsBagCell,
    global_initializers: Vec<(VariableIdx, HIRExpr)>,
}

impl ASTVisitor for HIRGlobalSymbolGatherer<'_> {
    fn get_ast(&self) -> &Ast {
        self.ast
    }

    fn visit_module_declaration(&mut self, mod_decl_stmt: &ModuleDeclaration) {}

    fn visit_struct_declaration(&mut self, struct_decl_stmt: &StructDeclaration) {
        let id = self.hir_gen.scope.borrow().lookup_struct_unqualified(&struct_decl_stmt.identifier.span.literal).unwrap();
        let fields = struct_decl_stmt.fields.iter().map(|f| {
            let ty = self.hir_gen.resolve_type_syntax(&f.ty.ty);
            (f.identifier.span.literal.clone(), ty)
        }).collect();
        self.hir_gen.scope.borrow_mut().set_struct_fields(&id, fields).unwrap();
    }


    fn visit_function_declaration(&mut self, func_decl_statement: &FunctionDeclaration) {
        common::declare_function(self.hir_gen, self.diagnostics_bag.clone(), func_decl_statement);
    }

    fn visit_let_statement(&mut self, let_statement: &LetStmt, statement: &Stmt) {
        let variable_declaration_stmt = common::declare_variable(self.hir_gen, let_statement);
        self.global_initializers.push((variable_declaration_stmt.variable_id, variable_declaration_stmt.initializer));
    }

    fn visit_index_expression(&mut self, index_expression: &ast::expr::IndexExpr, expr: &Expr) {}

    fn visit_struct_init_expression(&mut self, struct_init_expression: &ast::expr::StructInitExpr, expr: &Expr) {}

    fn visit_member_access_expression(&mut self, member_access_expression: &ast::expr::MemberAccessExpr, expr: &Expr) {}

    fn visit_cast_expression(&mut self, cast_expression: &ast::expr::CastExpr, expr: &Expr) {}

    fn visit_char_expression(&mut self, char_expression: &ast::expr::CharExpr, expr: &Expr) {}

    fn visit_deref_expression(&mut self, deref_expression: &ast::expr::DerefExpr) {}

    fn visit_ref_expression(&mut self, ref_expression: &ast::expr::RefExpr) {}

    fn visit_string_expression(&mut self, string_expression: &ast::expr::StringExpr, expr: &Expr) {}

    fn visit_assignment_expression(&mut self, assignment_expression: &ast::expr::AssignExpr, expr: &Expr) {}

    fn visit_identifier_expression(&mut self, variable_expression: &ast::expr::IdenExpr, expr: &Expr) {}

    fn visit_number_expression(&mut self, number: &ast::expr::NumberExpr, expr: &Expr) {}

    fn visit_boolean_expression(&mut self, boolean: &ast::expr::BoolExpr, expr: &Expr) {}

    fn visit_error(&mut self, span: &TextSpan) {}

    fn visit_unary_expression(&mut self, unary_expression: &ast::expr::UnaryExpr, expr: &Expr) {}
}

