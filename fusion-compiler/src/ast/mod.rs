use std::collections::HashMap;
use std::fmt::format;
use std::hash::Hash;

use termion::color::{Fg, Reset};

use printer::ASTPrinter;
use visitor::ASTVisitor;

use crate::ast::lexer::Token;
use crate::ast::parser::Counter;
use crate::compilation::symbols::function::FunctionSymbol;
use crate::compilation::symbols::variable::VariableSymbol;
use crate::compilation::symbols::class::ClassSymbol;
use crate::text::span::TextSpan;
use crate::typings::Type;

pub mod lexer;
pub mod parser;
pub mod evaluator;
pub mod visitor;
pub mod printer;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ASTNodeId {
    Stmt(ASTStmtId),
    Expr(ASTExprId),
    Unknown,
}

impl From<ASTStmtId> for ASTNodeId {
    fn from(id: ASTStmtId) -> Self {
        ASTNodeId::Stmt(id)
    }
}

impl From<&ASTStmtId> for ASTNodeId {
    fn from(id: &ASTStmtId) -> Self {
        Self::from(*id)
    }
}

impl From<ASTExprId> for ASTNodeId {
    fn from(id: ASTExprId) -> Self {
        ASTNodeId::Expr(id)
    }
}

impl From<&ASTExprId> for ASTNodeId {
    fn from(id: &ASTExprId) -> Self {
        Self::from(*id)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ASTStmtId {
    pub id: usize,
}

impl ASTStmtId {
    pub fn new(id: usize) -> Self {
        ASTStmtId { id }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct ASTExprId {
    pub id: usize,
}

impl ASTExprId {
    pub fn new(id: usize) -> Self {
        ASTExprId { id }
    }
}

pub struct ASTNode {
    pub id: ASTNodeId,
    pub span: TextSpan,
}

impl ASTNode {
    pub fn new(id: ASTNodeId, span: TextSpan) -> Self {
        Self { id, span }
    }
}

#[derive(Debug, Clone)]
pub struct ASTNodeIdGen {
    pub next_stmt_id: Counter,
    pub next_expr_id: Counter,
}

impl ASTNodeIdGen {
    pub fn new() -> Self {
        Self { next_stmt_id: Counter::new(), next_expr_id: Counter::new() }
    }

    pub fn next_stmt_id(&self) -> ASTStmtId {
        let id = self.next_stmt_id.get_value();
        self.next_stmt_id.increment();
        ASTStmtId::new(id)
    }

    pub fn next_expr_id(&self) -> ASTExprId {
        let id = self.next_expr_id.get_value();
        self.next_expr_id.increment();
        ASTExprId::new(id)
    }
}

#[derive(Debug, Clone)]
pub struct Ast {
    pub statements: HashMap<ASTStmtId, ASTStatement>,
    pub expressions: HashMap<ASTExprId, ASTExpression>,
    pub top_level_statements: Vec<ASTStmtId>,
    pub node_id_gen: ASTNodeIdGen,
}

impl Ast {
    pub fn new() -> Self {
        Self { statements: HashMap::new(), expressions: HashMap::new(), node_id_gen: ASTNodeIdGen::new(), top_level_statements: Vec::new() }
    }

    pub fn query_expr(&self, expr_id: &ASTExprId) -> &ASTExpression {
        &self.expressions[expr_id]
    }

    pub fn query_stmt(&self, stmt_id: &ASTStmtId) -> &ASTStatement {
        &self.statements[stmt_id]
    }

    pub fn span(&self, node_id: &ASTNodeId) -> TextSpan {
        match node_id {
            ASTNodeId::Stmt(stmt_id) => {
                let stmt = self.query_stmt(stmt_id);
                stmt.span(&self)
            }
            ASTNodeId::Expr(expr_id) => {
                let expr = self.query_expr(expr_id);
                expr.span(&self)
            }
            ASTNodeId::Unknown => panic!("Cannot get span for unknown node"),
        }
    }

    pub fn compute_node(&self, node_id: &ASTNodeId) -> Option<ASTNode> {
        match node_id {
            ASTNodeId::Stmt(stmt_id) => {
                let stmt = self.query_stmt(stmt_id);
                Some(ASTNode::new(*node_id, stmt.span(&self)))
            }
            ASTNodeId::Expr(expr_id) => {
                let expr = self.query_expr(expr_id);
                Some(ASTNode::new(*node_id, expr.span(&self)))
            }
            ASTNodeId::Unknown => None,
        }
    }

    pub fn set_type(&mut self, expr_id: &ASTExprId, ty: Type) {
        let expr = self.expressions.get_mut(expr_id).unwrap();
        expr.ty = ty;
    }

    pub fn set_symbol_for_stmt(&mut self, stmt_id: &ASTStmtId, symbol: VariableSymbol) {
        let stmt = self.statements.get_mut(stmt_id).unwrap();
        match &mut stmt.kind {
            ASTStatementKind::Let(let_stmt) => {
                let_stmt.variable = Some(symbol);
            }
            _ => panic!("Cannot set symbol for non-let statement"),
        }
    }

    pub fn set_symbol_for_expr(&mut self, expr_id: &ASTExprId, symbol: VariableSymbol) {
        let expr = self.expressions.get_mut(expr_id).unwrap();
        match &mut expr.kind {
            ASTExpressionKind::Identifier(expr) => {
                match &expr.kind {
                    ASTIdentifierKind::Variable(symbol) => {
                        expr.kind = ASTIdentifierKind::Variable(symbol.clone());
                    }
                    _ => panic!("Cannot set symbol for non-variable identifier"),
                }
            }
            ASTExpressionKind::Assignment(assignment) => {
                assignment.variable = Some(symbol);
            }
            _ => panic!("Cannot set symbol for non-variable expression: {:?}", expr.kind),
        }
    }

    pub fn set_identifier_kind(&mut self, expr_id: &ASTExprId, kind: ASTIdentifierKind) {
        let expr = self.expressions.get_mut(expr_id).unwrap();
        match &mut expr.kind {
            ASTExpressionKind::Identifier(expr) => {
                expr.kind = kind;
            }
            _ => panic!("Cannot set identifier kind for non-identifier expression"),
        }
    }

    pub fn set_top_level_on_return(&mut self, stmt_id: &ASTStmtId, is_top_level: bool) {
        let stmt = self.statements.get_mut(stmt_id).unwrap();
        match &mut stmt.kind {
            ASTStatementKind::Return(ret) => {
                ret.is_top_level = is_top_level;
            }
            _ => panic!("Cannot set top level on non-return expression"),
        }
    }

    pub fn mark_top_level_statement(&mut self, stmt_id: ASTStmtId) {
        self.top_level_statements.push(stmt_id);
    }

    fn stmt_from_kind(&mut self, kind: ASTStatementKind) -> &ASTStatement {
        let stmt = ASTStatement::new(kind, self.node_id_gen.next_stmt_id());
        let id = stmt.id;
        self.statements.insert(id, stmt);
        &self.statements[&id]
    }

    pub fn expression_statement(&mut self, expr_id: ASTExprId) -> &ASTStatement {
        self.stmt_from_kind(ASTStatementKind::Expression(expr_id))
    }

    pub fn let_statement(&mut self, identifier: Token, initializer: ASTExprId, type_annotation: Option<StaticTypeAnnotation>, variable: Option<VariableSymbol>) -> &ASTStatement {
        self.stmt_from_kind(ASTStatementKind::Let(ASTLetStatement { identifier, initializer, type_annotation, variable }))
    }

    pub fn if_statement(&mut self, if_keyword: Token, condition: ASTExprId, then: ASTStmtId, else_statement: Option<ASTElseStatement>) -> &ASTStatement {
        self.stmt_from_kind(ASTStatementKind::If(ASTIfStatement { if_keyword, condition, then_branch: then, else_branch: else_statement }))
    }

    pub fn block_statement(&mut self, open_brace: Token, statements: Vec<ASTStmtId>, close_brace: Token) -> &ASTStatement {
        self.stmt_from_kind(ASTStatementKind::Block(ASTBlockStatement { statements, open_brace, close_brace }))
    }

    pub fn while_statement(&mut self, while_keyword: Token, condition: ASTExprId, body: ASTStmtId) -> &ASTStatement {
        self.stmt_from_kind(ASTStatementKind::While(ASTWhileStatement { while_keyword, condition, body }))
    }

    pub fn return_statement(&mut self, return_keyword: Token, return_value: Option<ASTExprId>, is_top_level: bool) -> &ASTStatement {
        self.stmt_from_kind(ASTStatementKind::Return(ASTReturnStatement { return_keyword, return_value, is_top_level }))
    }

    pub fn func_decl_statement(&mut self, func_token: Token, modifier_tokens: Vec<Token>, identifier: Token, parameters: Vec<FuncDeclParameter>, body: Option<ASTStmtId>, return_type: Option<ASTFunctionReturnType>) -> &ASTStatement {
        self.stmt_from_kind(ASTStatementKind::FuncDecl(ASTFuncDeclStatement { identifier, parameters, body, return_type, modifier_tokens, func_token }))
    }

    pub fn class_statement(&mut self, class_token: Token, identifier: Token, constructor: Option<ASTClassConstructor>, body: ASTClassBody) -> &ASTStatement {
        self.stmt_from_kind(ASTStatementKind::Class(ASTClassStatement { class_keyword: class_token, identifier, constructor, body }))
    }

    fn expr_from_kind(&mut self, kind: ASTExpressionKind) -> &ASTExpression {
        let expr = ASTExpression::new(kind, self.node_id_gen.next_expr_id(), Type::Unresolved);
        let expr_id = expr.id;
        self.expressions.insert(expr_id, expr);
        &self.expressions[&expr_id]
    }

    pub fn number_expression(&mut self, token: Token, number: i64) -> &ASTExpression {
        self.expr_from_kind(ASTExpressionKind::Number(ASTNumberExpression { number, token }))
    }

    pub fn string_expression(&mut self, open_quote: Token, value: ASTString, close_quote: Token) -> &ASTExpression {
        self.expr_from_kind(ASTExpressionKind::String(ASTStringExpression { open_quote, close_quote, string: value }))
    }

    pub fn binary_expression(&mut self, operator: ASTBinaryOperator, left: ASTExprId, right: ASTExprId) -> &ASTExpression {
        self.expr_from_kind(ASTExpressionKind::Binary(ASTBinaryExpression { operator, left, right }))
    }

    pub fn parenthesized_expression(&mut self, left_paren: Token, expression: ASTExprId, right_paren: Token) -> &ASTExpression {
        self.expr_from_kind(ASTExpressionKind::Parenthesized(ASTParenthesizedExpression { expression, left_paren, right_paren }))
    }

    pub fn identifier_expression(&mut self, identifier: Token) -> &ASTExpression {
        self.expr_from_kind(ASTExpressionKind::Identifier(ASTIdentifierExpression { identifier,kind: ASTIdentifierKind::Unknown }))
    }

    pub fn unary_expression(&mut self, operator: ASTUnaryOperator, operand: ASTExprId) -> &ASTExpression {
        self.expr_from_kind(ASTExpressionKind::Unary(ASTUnaryExpression { operator, operand }))
    }

    pub fn assignment_expression(&mut self, identifier: Token, equals: Token, expression: ASTExprId, variable: Option<VariableSymbol>) -> &ASTExpression {
        self.expr_from_kind(ASTExpressionKind::Assignment(ASTAssignmentExpression { identifier, expression, equals, variable }))
    }

    pub fn boolean_expression(&mut self, token: Token, value: bool) -> &ASTExpression {
        self.expr_from_kind(ASTExpressionKind::Boolean(ASTBooleanExpression { token, value }))
    }

    pub fn call_expression(&mut self, callee: ASTExprId, left_paren: Token, arguments: Vec<ASTExprId>, right_paren: Token) -> &ASTExpression {
        self.expr_from_kind(ASTExpressionKind::Call(ASTCallExpression { callee, arguments, left_paren, right_paren }))
    }

    pub fn member_expression(&mut self, object: ASTExprId, dot: Token, property: Token) -> &ASTExpression {
        self.expr_from_kind(ASTExpressionKind::MemberAccess(ASTMemberAccessExpression { object, dot, target: property }))
    }

    pub fn self_expression(&mut self, token: Token) -> &ASTExpression {
        self.expr_from_kind(ASTExpressionKind::Self_(ASTSelfExpression { self_keyword: token }))
    }

    pub fn error_expression(&mut self, span: TextSpan) -> &ASTExpression {
        self.expr_from_kind(ASTExpressionKind::Error(span))
    }

    pub fn visit(&self, visitor: &mut dyn ASTVisitor) {
        for statement in &self.top_level_statements {
            visitor.visit_statement(statement);
        }
    }

    pub fn visualize(&self) -> () {
        let mut printer = ASTPrinter::new(
            self
        );
        self.visit(&mut printer);
        println!("{}", printer.result);
    }
}

#[derive(Debug, Clone)]
pub enum ASTStatementKind {
    Expression(ASTExprId),
    Let(ASTLetStatement),
    If(ASTIfStatement),
    Block(ASTBlockStatement),
    While(ASTWhileStatement),
    FuncDecl(ASTFuncDeclStatement),
    Return(ASTReturnStatement),
    Class(ASTClassStatement),
}


#[derive(Debug, Clone)]
pub struct ASTClassStatement {
    pub class_keyword: Token,
    pub identifier: Token,
    pub constructor: Option<ASTClassConstructor>,
    pub body: ASTClassBody,
}

#[derive(Debug, Clone)]
pub struct ASTClassConstructor {
    pub fields: Vec<ASTClassField>,
}

#[derive(Debug, Clone)]
pub struct ASTClassBody {
    pub open_brace: Token,
    pub members: Vec<ASTClassMember>,
    pub close_brace: Token,
}

#[derive(Debug, Clone)]
pub enum ASTClassMember {
    Field(ASTClassField),
    Method(ASTClassMethod),
    Invalid(ASTStmtId),
}

#[derive(Debug, Clone)]
pub struct ASTClassField {
    pub identifier: Token,
    pub type_annotation: StaticTypeAnnotation,
}

#[derive(Debug, Clone)]
pub struct ASTClassMethod {
    pub func_decl: ASTStmtId,
}

#[derive(Debug, Clone)]
pub struct ASTReturnStatement {
    pub return_keyword: Token,
    pub return_value: Option<ASTExprId>,
    pub is_top_level: bool,
}

#[derive(Debug, Clone)]
pub struct StaticTypeAnnotation {
    pub colon: Token,
    pub type_name: Token,
}

impl StaticTypeAnnotation {
    pub fn new(colon: Token, type_name: Token) -> Self {
        Self { colon, type_name }
    }
}

#[derive(Debug, Clone)]
pub enum FuncDeclParameter {
    Normal(
        NormalFuncDeclParameter
    ),
    Self_(Token),
}

#[derive(Debug, Clone)]
pub struct NormalFuncDeclParameter {
    pub identifier: Token,
    pub type_annotation: StaticTypeAnnotation,
}

#[derive(Debug, Clone)]
pub struct ASTFunctionReturnType {
    pub arrow: Token,
    pub type_name: Token,
}

impl ASTFunctionReturnType {
    pub fn new(arrow: Token, type_name: Token) -> Self {
        Self { arrow, type_name }
    }
}

#[derive(Debug, Clone)]
pub struct ASTFuncDeclStatement {
    pub func_token: Token,
    pub modifier_tokens: Vec<Token>,
    pub identifier: Token,
    pub parameters: Vec<FuncDeclParameter>,
    pub body: Option<ASTStmtId>,
    pub return_type: Option<ASTFunctionReturnType>,
}

#[derive(Debug, Clone)]
pub struct ASTWhileStatement {
    pub while_keyword: Token,
    pub condition: ASTExprId,
    pub body: ASTStmtId,
}

#[derive(Debug, Clone)]
pub struct ASTBlockStatement {
    pub open_brace: Token,
    pub statements: Vec<ASTStmtId>,
    pub close_brace: Token,
}

#[derive(Debug, Clone)]
pub struct ASTElseStatement {
    pub else_keyword: Token,
    pub else_statement: ASTStmtId,
}

impl ASTElseStatement {
    pub fn new(else_keyword: Token, else_statement: ASTStmtId) -> Self {
        ASTElseStatement { else_keyword, else_statement }
    }
}

#[derive(Debug, Clone)]
pub struct ASTIfStatement {
    pub if_keyword: Token,
    pub condition: ASTExprId,
    pub then_branch: ASTStmtId,
    pub else_branch: Option<ASTElseStatement>,
}

#[derive(Debug, Clone)]
pub struct ASTLetStatement {
    pub identifier: Token,
    pub initializer: ASTExprId,
    pub type_annotation: Option<StaticTypeAnnotation>,
    pub variable: Option<VariableSymbol>,
}

#[derive(Debug, Clone)]
pub struct ASTStatement {
    pub kind: ASTStatementKind,
    pub id: ASTStmtId,
}

impl ASTStatement {
    pub fn new(kind: ASTStatementKind, id: ASTStmtId) -> Self {
        ASTStatement { kind, id }
    }

    pub fn into_func_decl(&self) -> &ASTFuncDeclStatement {
        match &self.kind {
            ASTStatementKind::FuncDecl(func_decl) => func_decl,
            _ => panic!("Expected func decl statement")
        }
    }

    pub fn into_block_stmt(&self) -> &ASTBlockStatement {
        match &self.kind {
            ASTStatementKind::Block(block_stmt) => block_stmt,
            _ => panic!("Expected block statement")
        }
    }

    pub fn span(&self, ast: &Ast) -> TextSpan {
        match &self.kind
        {
            ASTStatementKind::Expression(expr) => ast.query_expr(expr).span(ast),
            ASTStatementKind::Let(stmt) => {
                let init_span = ast.query_expr(&stmt.initializer).span(ast);
                let mut spans = vec![
                    &stmt.identifier.span,
                    &init_span,
                ];
                if let Some(type_annotation) = &stmt.type_annotation {
                    spans.push(&type_annotation.colon.span);
                    spans.push(&type_annotation.type_name.span);
                }
                TextSpan::merge(
                    spans
                )
            }
            ASTStatementKind::If(stmt) => {
                let cond_span = ast.query_expr(&stmt.condition).span(ast);
                let then_branch_span = ast.query_stmt(&stmt.then_branch).span(ast);
                let mut spans = vec![
                    &stmt.if_keyword.span,
                    &cond_span,
                    &then_branch_span,
                ];

                let else_spans = stmt.else_branch.as_ref().map(|else_branch| {
                    (&else_branch.else_keyword.span, ast.query_stmt(&else_branch.else_statement).span(ast).clone())
                });

                if let Some((else_keyword_span, else_branch_span)) = else_spans.as_ref() {
                    spans.push(else_keyword_span);
                    spans.push(&else_branch_span);
                }
                TextSpan::merge(
                    spans
                )
            }
            ASTStatementKind::Block(stmt) => {
                let mut spans = vec![
                    stmt.open_brace.span.clone(),
                    stmt.close_brace.span.clone(),
                ];
                spans.extend(
                    stmt.statements.iter().map(|stmt| ast.query_stmt(stmt).span(ast)));
                TextSpan::merge(
                    spans.iter().map(|span| span).collect()
                )
            }
            ASTStatementKind::While(stmt) => {
                let cond_span = ast.query_expr(&stmt.condition).span(ast);
                let body_span = ast.query_stmt(&stmt.body).span(ast);
                let spans = vec![
                    &stmt.while_keyword.span,
                    &cond_span,
                    &body_span,
                ];
                TextSpan::merge(
                    spans
                )
            }
            ASTStatementKind::FuncDecl(stmt) => {
                let mut spans = vec![
                    &stmt.identifier.span,
                ];
                for parameter in &stmt.parameters {
                    match parameter {
                        FuncDeclParameter::Normal(parameter) => {
                            spans.push(&parameter.identifier.span);
                            spans.push(&parameter.type_annotation.colon.span);
                            spans.push(&parameter.type_annotation.type_name.span);
                        }
                        FuncDeclParameter::Self_(token) => {
                            spans.push(&token.span);
                        }
                    }
                }
                if let Some(return_type) = &stmt.return_type {
                    spans.push(&return_type.arrow.span);
                    spans.push(&return_type.type_name.span);
                }
                let body_span = stmt.body.as_ref().map(|body| ast.query_stmt(body).span(ast));
                if let Some(body_span) = body_span.as_ref() {
                    spans.push(body_span);
                }
                TextSpan::merge(
                    spans
                )
            }
            ASTStatementKind::Return(stmt) => {
                let mut spans = vec![
                    &stmt.return_keyword.span,
                ];
                let return_value_span = stmt.return_value.as_ref().map(|return_value| ast.query_expr(&return_value).span(ast));
                if let Some(return_value_span) = return_value_span.as_ref() {
                    spans.push(return_value_span);
                }
                TextSpan::merge(
                    spans
                )
            }
            ASTStatementKind::Class(stmt) => {
                let spans = vec![
                    &stmt.class_keyword.span,
                    &stmt.identifier.span,
                ];
                // todo:
                // for member in &stmt.body.members {
                //     spans.push(ast.query_stmt(mem).span(ast));
                // }
                TextSpan::merge(
                    spans
                )
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum ASTExpressionKind {
    Number(
        ASTNumberExpression
    ),
    String(
        ASTStringExpression
    ),
    Binary(
        ASTBinaryExpression
    ),
    Unary(
        ASTUnaryExpression
    ),
    Parenthesized(
        ASTParenthesizedExpression
    ),

    Identifier(
        ASTIdentifierExpression
    ),
    Assignment(
        ASTAssignmentExpression
    ),
    Boolean(
        ASTBooleanExpression
    ),
    Call(
        ASTCallExpression
    ),
    MemberAccess(
        ASTMemberAccessExpression
    ),
    Self_(ASTSelfExpression),
    Error(
        TextSpan
    ),
}

#[derive(Debug, Clone)]
pub struct ASTSelfExpression {
    pub self_keyword: Token,
}

#[derive(Debug, Clone)]
pub struct ASTMemberAccessExpression {
    pub object: ASTExprId,
    pub dot: Token,
    pub target: Token,
}

#[derive(Debug, Clone)]
pub struct ASTStringExpression {
    pub open_quote: Token,
    pub string: ASTString,
    pub close_quote: Token,
}

#[derive(Debug, Clone)]
pub struct ASTString {
    pub parts: Vec<StringPart>,
}

impl ASTString {
    pub fn new(parts: Vec<StringPart>) -> Self {
        ASTString { parts }
    }

    pub fn to_raw_string(&self) -> String {
        let mut result = String::new();
        for part in &self.parts {
            match part {
                StringPart::Literal(literal) => result.push_str(literal),
                StringPart::EscapeSequence(escape_sequence) => result.push_str(&format!("{}", escape_sequence.as_raw_string()))
            }
        }
        result
    }

    pub fn to_string(&self) -> String {
        let mut result = String::new();
        for part in &self.parts {
            match part {
                StringPart::Literal(literal) => result.push_str(literal),
                StringPart::EscapeSequence(escape_sequence) => {
                    result.push_str(&escape_sequence.as_string())
                }
            }
        }
        result
    }
}

#[derive(Debug, Clone)]
pub enum StringPart {
    Literal(String),
    // Expression(ASTExprId),
    EscapeSequence(EscapedCharacter),
}

#[derive(Debug, Clone)]
pub enum EscapedCharacter {
    Newline,
    CarriageReturn,
    Tab,
    Quote,
}

impl EscapedCharacter {
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            'n' => Some(EscapedCharacter::Newline),
            'r' => Some(EscapedCharacter::CarriageReturn),
            't' => Some(EscapedCharacter::Tab),
            '"' => Some(EscapedCharacter::Quote),
            _ => None,
        }
    }

    pub fn as_raw_string(&self) -> String {
        let mut result = String::new();
        result.push('\\');
        match self {
            EscapedCharacter::Newline => result.push('n'),
            EscapedCharacter::CarriageReturn => result.push('r'),
            EscapedCharacter::Tab => result.push('t'),
            EscapedCharacter::Quote => result.push('"'),
        }
        result
    }

    pub fn as_string(&self) -> String {
        let mut result = String::new();
        match self {
            EscapedCharacter::Newline => result.push_str(&format!("\n")),
            EscapedCharacter::CarriageReturn => result.push_str(&format!("\r")),
            EscapedCharacter::Tab => result.push_str(&format!("\t")),
            EscapedCharacter::Quote => result.push_str(&format!("\"")),
        }
        result
    }
}

#[derive(Debug, Clone)]
pub struct ASTCallExpression {
    pub callee: ASTExprId,
    pub left_paren: Token,
    pub arguments: Vec<ASTExprId>,
    pub right_paren: Token,
}

#[derive(Debug, Clone)]
pub struct ASTBooleanExpression {
    pub value: bool,
    pub token: Token,
}

#[derive(Debug, Clone)]
pub struct ASTAssignmentExpression {
    pub identifier: Token,
    pub equals: Token,
    pub expression: ASTExprId,
    pub variable: Option<VariableSymbol>,
}

#[derive(Debug, Clone)]
pub enum ASTUnaryOperatorKind {
    Minus,
    BitwiseNot,
}

#[derive(Debug, Clone)]
pub struct ASTUnaryOperator {
    pub(crate) kind: ASTUnaryOperatorKind,
    pub token: Token,
}

impl ASTUnaryOperator {
    pub fn new(kind: ASTUnaryOperatorKind, token: Token) -> Self {
        ASTUnaryOperator { kind, token }
    }
}

#[derive(Debug, Clone)]
pub struct ASTUnaryExpression {
    pub operator: ASTUnaryOperator,
    pub operand: ASTExprId,
}

#[derive(Debug, Clone)]
pub struct ASTIdentifierExpression {
    pub identifier: Token,
    pub kind: ASTIdentifierKind,
}

#[derive(Debug, Clone)]
pub enum ASTIdentifierKind {
    Variable(VariableSymbol),
    Function(FunctionSymbol),
    Class(ClassSymbol),
    Unknown,
}

impl ASTIdentifierExpression {
    pub fn identifier(&self) -> &str {
        &self.identifier.span.literal
    }
}


#[derive(Debug, Clone)]
pub enum ASTBinaryOperatorKind {
    // Arithmetic
    Plus,
    Minus,
    Multiply,
    Divide,
    Power,
    // Bitwise
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    // Relational
    Equals,
    NotEquals,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

#[derive(Debug, Clone)]
pub struct ASTBinaryOperator {
    pub kind: ASTBinaryOperatorKind,
    pub token: Token,
}

impl ASTBinaryOperator {
    pub fn new(kind: ASTBinaryOperatorKind, token: Token) -> Self {
        ASTBinaryOperator { kind, token }
    }

    pub fn precedence(&self) -> u8 {
        match self.kind {
            ASTBinaryOperatorKind::Power => 20,
            ASTBinaryOperatorKind::Multiply => 19,
            ASTBinaryOperatorKind::Divide => 19,
            ASTBinaryOperatorKind::Plus => 18,
            ASTBinaryOperatorKind::Minus => 18,
            ASTBinaryOperatorKind::BitwiseAnd => 17,
            ASTBinaryOperatorKind::BitwiseXor => 16,
            ASTBinaryOperatorKind::BitwiseOr => 15,
            ASTBinaryOperatorKind::Equals => 30,
            ASTBinaryOperatorKind::NotEquals => 30,
            ASTBinaryOperatorKind::LessThan => 29,
            ASTBinaryOperatorKind::LessThanOrEqual => 29,
            ASTBinaryOperatorKind::GreaterThan => 29,
            ASTBinaryOperatorKind::GreaterThanOrEqual => 29,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ASTBinaryExpression {
    pub left: ASTExprId,
    pub operator: ASTBinaryOperator,
    pub right: ASTExprId,
}

#[derive(Debug, Clone)]
pub struct ASTNumberExpression {
    pub number: i64,
    pub token: Token,
}

#[derive(Debug, Clone)]
pub struct ASTParenthesizedExpression {
    pub left_paren: Token,
    pub expression: ASTExprId,
    pub right_paren: Token,
}

#[derive(Debug, Clone)]
pub struct ASTExpression {
    pub kind: ASTExpressionKind,
    pub id: ASTExprId,
    pub ty: Type,
}

impl ASTExpression {
    pub fn new(kind: ASTExpressionKind, id: ASTExprId, ty: Type) -> Self {
        ASTExpression { kind, id, ty }
    }

    pub fn span(&self, ast: &Ast) -> TextSpan {
        match &self.kind {
            ASTExpressionKind::Number(expr) => expr.token.span.clone(),
            ASTExpressionKind::Binary(expr) => {
                let left = &ast.query_expr(&expr.left).span(ast);
                let operator = &expr.operator.token.span;
                let right = &ast.query_expr(&expr.right).span(ast);
                TextSpan::merge(vec![left, operator, right])
            }
            ASTExpressionKind::Unary(expr) => {
                let operator = &expr.operator.token.span;
                let operand = &ast.query_expr(&expr.operand).span(ast);
                TextSpan::merge(vec![operator, operand])
            }
            ASTExpressionKind::Parenthesized(expr) => {
                let open_paren = &expr.left_paren.span;
                let expression = &ast.query_expr(&expr.expression).span(ast);
                let close_paren = &expr.right_paren.span;
                TextSpan::merge(vec![open_paren, expression, close_paren])
            }
            ASTExpressionKind::Identifier(expr) => expr.identifier.span.clone(),
            ASTExpressionKind::Assignment(expr) => {
                let identifier = &expr.identifier.span;
                let equals = &expr.equals.span;
                let expression = &ast.query_expr(&expr.expression).span(ast);
                TextSpan::merge(vec![identifier, equals, expression])
            }
            ASTExpressionKind::Boolean(expr) => expr.token.span.clone(),
            ASTExpressionKind::Call(expr) => {
                let expr_span = &ast.query_expr(&expr.callee).span(ast);
                let left_paren = &expr.left_paren.span;
                let right_paren = &expr.right_paren.span;
                let mut spans = vec![expr_span, left_paren, right_paren];
                let argument_spans: Vec<TextSpan> = expr.arguments.iter().map(|arg| ast.query_expr(arg).span(ast)).collect();
                for span in &argument_spans {
                    spans.push(span);
                }
                TextSpan::merge(spans)
            }
            ASTExpressionKind::Error(span) => span.clone(),
            ASTExpressionKind::String(expr) => {
                let spans = vec![&expr.open_quote.span, &expr.close_quote.span];
                TextSpan::merge(spans)
            }
            ASTExpressionKind::MemberAccess(expr) => {
                let expr_span = &ast.query_expr(&expr.object).span(ast);
                let dot = &expr.dot.span;
                let target = &expr.target.span;
                TextSpan::merge(vec![expr_span, dot, target])
            }
            ASTExpressionKind::Self_(expr) => expr.self_keyword.span.clone(),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::ast::{Ast, ASTAssignmentExpression, ASTBinaryExpression, ASTBlockStatement, ASTBooleanExpression, ASTCallExpression, ASTExpression, ASTFuncDeclStatement, ASTIdentifierExpression, ASTIfStatement, ASTLetStatement, ASTNumberExpression, ASTParenthesizedExpression, ASTReturnStatement, ASTStatement, ASTUnaryExpression, ASTWhileStatement};
    use crate::compilation::CompilationUnit;
    use crate::text::SourceText;
    use crate::text::span::TextSpan;

    use super::visitor::ASTVisitor;

    #[derive(Debug, PartialEq, Eq)]
    enum TestASTNode {
        Number(i64),
        Boolean(bool),
        Binary,
        Unary,
        Parenthesized,
        Let,
        Assignment,
        Block,
        Variable(String),
        If,
        Else,
        Func,
        While,
        Return,
        Call,
    }

    struct ASTVerifier {
        expected: Vec<TestASTNode>,
        actual: Vec<TestASTNode>,
        ast: Ast,
    }

    impl ASTVerifier {
        pub fn new(input: &str, expected: Vec<TestASTNode>) -> Self {
            let source_text = SourceText::new(input, None);
            let compilation_unit = CompilationUnit::compile(&source_text).expect("Failed to compile");
            let mut verifier = ASTVerifier { expected, actual: Vec::new(), ast: compilation_unit.ast };
            verifier.flatten_ast();
            verifier
        }

        fn flatten_ast(&mut self) {
            self.actual.clear();
            let ast = &self.ast.clone();
            ast.visit(&mut *self);
        }

        pub fn verify(&self) {
            assert_eq!(self.expected.len(), self.actual.len(), "Expected {} nodes, but got {}. Actual nodes: {:?}", self.expected.len(), self.actual.len(), self.actual);

            for (index, (expected, actual)) in self.expected.iter().zip(
                self.actual.iter()
            ).enumerate() {
                assert_eq!(expected, actual, "Expected {:?} at index {}, but got {:?}", expected, index, actual);
            }
        }
    }

    impl ASTVisitor for ASTVerifier {
        fn get_ast(&self) -> &Ast {
            &self.ast
        }

        fn visit_func_decl_statement(&mut self, func_decl_statement: &ASTFuncDeclStatement) {
            self.actual.push(TestASTNode::Func);
            self.visit_statement(&func_decl_statement.body);
        }

        fn visit_return_statement(&mut self, return_statement: &ASTReturnStatement) {
            self.actual.push(TestASTNode::Return);
            if let Some(expression) = &return_statement.return_value {
                self.visit_expression(expression);
            }
        }

        fn visit_while_statement(&mut self, while_statement: &ASTWhileStatement) {
            self.actual.push(TestASTNode::While);
            self.visit_expression(&while_statement.condition);
            self.visit_statement(&while_statement.body);
        }

        fn visit_block_statement(&mut self, block_statement: &ASTBlockStatement) {
            self.actual.push(TestASTNode::Block);
            for statement in &block_statement.statements {
                self.visit_statement(statement);
            }
        }

        fn visit_if_statement(&mut self, if_statement: &ASTIfStatement) {
            self.actual.push(TestASTNode::If);
            self.visit_expression(&if_statement.condition);
            self.visit_statement(&if_statement.then_branch);
            if let Some(else_branch) = &if_statement.else_branch {
                self.actual.push(TestASTNode::Else);

                self.visit_statement(&else_branch.else_statement);
            }
        }

        fn visit_let_statement(&mut self, let_statement: &ASTLetStatement, stmt: &ASTStatement) {
            self.actual.push(TestASTNode::Let);
            self.visit_expression(&let_statement.initializer);
        }

        fn visit_call_expression(&mut self, call_expression: &ASTCallExpression, expr: &ASTExpression) {
            self.actual.push(TestASTNode::Call);
            for argument in &call_expression.arguments {
                self.visit_expression(argument);
            }
        }

        fn visit_assignment_expression(&mut self, assignment_expression: &ASTAssignmentExpression, expr: &ASTExpression) {
            self.actual.push(TestASTNode::Assignment);
            self.visit_expression(&assignment_expression.expression);
        }

        fn visit_identifier_expression(&mut self, variable_expression: &ASTIdentifierExpression, expr: &ASTExpression) {
            self.actual.push(TestASTNode::Variable(
                variable_expression.identifier().to_string()
            ));
        }

        fn visit_number_expression(&mut self, number: &ASTNumberExpression, expr: &ASTExpression) {
            self.actual.push(TestASTNode::Number(number.number));
        }

        fn visit_boolean_expression(&mut self, boolean: &ASTBooleanExpression, expr: &ASTExpression) {
            self.actual.push(TestASTNode::Boolean(boolean.value));
        }

        fn visit_error(&mut self, span: &TextSpan) {
            // do nothing
        }

        fn visit_unary_expression(&mut self, unary_expression: &ASTUnaryExpression, expr: &ASTExpression) {
            self.actual.push(TestASTNode::Unary);
            self.visit_expression(&unary_expression.operand);
        }

        fn visit_binary_expression(&mut self, binary_expression: &ASTBinaryExpression, expr: &ASTExpression) {
            self.actual.push(TestASTNode::Binary);
            self.visit_expression(&binary_expression.left);
            self.visit_expression(&binary_expression.right);
        }

        fn visit_parenthesized_expression(&mut self, parenthesized_expression: &ASTParenthesizedExpression, expr: &ASTExpression) {
            self.actual.push(TestASTNode::Parenthesized);
            self.visit_expression(&parenthesized_expression.expression);
        }
    }


    fn assert_tree(input: &str, expected: Vec<TestASTNode>) {
        let verifier = ASTVerifier::new(input, expected);
        verifier.verify();
    }

    #[test]
    pub fn should_parse_basic_binary_expression() {
        let input = "let a = 1 + 2";
        let expected = vec![
            TestASTNode::Let,
            TestASTNode::Binary,
            TestASTNode::Number(1),
            TestASTNode::Number(2),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_parenthesized_binary_expression() {
        let input = "let a = (1 + 2) * 3";
        let expected = vec![
            TestASTNode::Let,
            TestASTNode::Binary,
            TestASTNode::Parenthesized,
            TestASTNode::Binary,
            TestASTNode::Number(1),
            TestASTNode::Number(2),
            TestASTNode::Number(3),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_parenthesized_binary_expression_with_variable() {
        let input = "\
        let b = 1
        let a = (1 + 2) * b";
        let expected = vec![
            TestASTNode::Let,
            TestASTNode::Number(1),
            TestASTNode::Let,
            TestASTNode::Binary,
            TestASTNode::Parenthesized,
            TestASTNode::Binary,
            TestASTNode::Number(1),
            TestASTNode::Number(2),
            TestASTNode::Variable("b".to_string()),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_parenthesized_binary_expression_with_variable_and_number() {
        let input = "\
        let b = 1
        let a = (1 + 2) * b + 3";
        let expected = vec![
            TestASTNode::Let,
            TestASTNode::Number(1),
            TestASTNode::Let,
            TestASTNode::Binary,
            TestASTNode::Binary,
            TestASTNode::Parenthesized,
            TestASTNode::Binary,
            TestASTNode::Number(1),
            TestASTNode::Number(2),
            TestASTNode::Variable("b".to_string()),
            TestASTNode::Number(3),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_bitwise_and() {
        let input = "let a = 1 & 2";
        let expected = vec![
            TestASTNode::Let,
            TestASTNode::Binary,
            TestASTNode::Number(1),
            TestASTNode::Number(2),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_bitwise_or() {
        let input = "let a = 1 | 2";
        let expected = vec![
            TestASTNode::Let,
            TestASTNode::Binary,
            TestASTNode::Number(1),
            TestASTNode::Number(2),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_bitwise_xor() {
        let input = "let a = 1 ^ 2";
        let expected = vec![
            TestASTNode::Let,
            TestASTNode::Binary,
            TestASTNode::Number(1),
            TestASTNode::Number(2),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_bitwise_not() {
        let input = "let a = ~1";
        let expected = vec![
            TestASTNode::Let,
            TestASTNode::Unary,
            TestASTNode::Number(1),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_negation() {
        let input = "let a = -1";
        let expected = vec![
            TestASTNode::Let,
            TestASTNode::Unary,
            TestASTNode::Number(1),
        ];

        assert_tree(input, expected);
    }


    #[test]
    pub fn should_parse_hilarious_amount_of_unary_operators() {
        let input = "let a = -1 + -2 * -3 * ------4";
        let expected = vec![
            TestASTNode::Let,
            TestASTNode::Binary,
            TestASTNode::Unary,
            TestASTNode::Number(1),
            TestASTNode::Binary,
            TestASTNode::Unary,
            TestASTNode::Number(2),
            TestASTNode::Binary,
            TestASTNode::Unary,
            TestASTNode::Number(3),
            TestASTNode::Unary,
            TestASTNode::Unary,
            TestASTNode::Unary,
            TestASTNode::Unary,
            TestASTNode::Unary,
            TestASTNode::Unary,
            TestASTNode::Number(4),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_if_statement() {
        let input = "\
        let a = 1
        if a > 0 {
            a = 20
        }
        ";
        let expected = vec![
            TestASTNode::Let,
            TestASTNode::Number(1),
            TestASTNode::If,
            TestASTNode::Binary,
            TestASTNode::Variable("a".to_string()),
            TestASTNode::Number(0),
            TestASTNode::Block,
            TestASTNode::Assignment,
            TestASTNode::Number(20),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_if_statement_with_else() {
        let input = "\
        let a = 1
        if a > 0 {
            a = 20
        } else {
            a = 30
        }
        ";
        let expected = vec![
            TestASTNode::Let,
            TestASTNode::Number(1),
            TestASTNode::If,
            TestASTNode::Binary,
            TestASTNode::Variable("a".to_string()),
            TestASTNode::Number(0),
            TestASTNode::Block,
            TestASTNode::Assignment,
            TestASTNode::Number(20),
            TestASTNode::Else,
            TestASTNode::Block,
            TestASTNode::Assignment,
            TestASTNode::Number(30),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_while_statement() {
        let input = "\
        let a = 1
        while a < 10 {
            a = a + 1
        }
        ";
        let expected = vec![
            TestASTNode::Let,
            TestASTNode::Number(1),
            TestASTNode::While,
            TestASTNode::Binary,
            TestASTNode::Variable("a".to_string()),
            TestASTNode::Number(10),
            TestASTNode::Block,
            TestASTNode::Assignment,
            TestASTNode::Binary,
            TestASTNode::Variable("a".to_string()),
            TestASTNode::Number(1),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_function_declaration() {
        let input = "\
        func add(a: int, b: int) -> int {
            return a + b
        }
        ";
        let expected = vec![
            TestASTNode::Func,
            TestASTNode::Block,
            TestASTNode::Return,
            TestASTNode::Binary,
            TestASTNode::Variable("a".to_string()),
            TestASTNode::Variable("b".to_string()),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_call_expression() {
        let input = "\
        func add(a: int, b: int) -> int {
            return a + b
        }
        add(2 * 3, 4 + 5)";
        let expected = vec![
            TestASTNode::Func,
            TestASTNode::Block,
            TestASTNode::Return,
            TestASTNode::Binary,
            TestASTNode::Variable("a".to_string()),
            TestASTNode::Variable("b".to_string()),
            TestASTNode::Call,
            TestASTNode::Binary,
            TestASTNode::Number(2),
            TestASTNode::Number(3),
            TestASTNode::Binary,
            TestASTNode::Number(4),
            TestASTNode::Number(5),
        ];

        assert_tree(input, expected);
    }
}
