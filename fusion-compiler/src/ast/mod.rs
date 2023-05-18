use std::fmt::{Display, Formatter};

use termion::color::{Fg, Reset};

use printer::ASTPrinter;
use visitor::ASTVisitor;

use crate::ast::lexer::Token;
use crate::text::span::TextSpan;

pub mod lexer;
pub mod parser;
pub mod visitor;
pub mod printer;


#[derive(Debug, Clone)]
pub struct Ast {
    pub statements: Vec<ASTStatement>,
    pub structs: Vec<Token>,
}

impl Ast {
    pub fn new() -> Self {
        Self { statements: Vec::new(), structs: Vec::new() }
    }


    pub fn expression_statement(&mut self, expr: ASTExpression) -> ASTStatement {
        ASTStatement::new(ASTStatementKind::Expression(
            expr
        ))
    }

    pub fn let_statement(&mut self, mut_token: Option<Token>, identifier: Token, initializer: ASTExpression, type_annotation: Option<StaticTypeAnnotation>) -> ASTStatement {
        ASTStatement::new(
            ASTStatementKind::Let(ASTLetStatement { mut_token, identifier, initializer, type_annotation }))
    }

    pub fn if_statement(&mut self, if_keyword: Token, condition: ASTExpression, then: ASTStatement, else_statement: Option<ASTElseStatement>) -> ASTStatement {
        ASTStatement::new(
            ASTStatementKind::If(ASTIfStatement
            {
                if_keyword,
                condition,
                then_branch: Box::new(then),
                else_branch: else_statement,
            }))
    }

    pub fn block_statement(&mut self, open_brace: Token, statements: Vec<ASTStatement>, close_brace: Token) -> ASTStatement {
        ASTStatement::new(ASTStatementKind::Block(ASTBlockStatement { statements, open_brace, close_brace }))
    }

    pub fn while_statement(&mut self, while_keyword: Token, condition: ASTExpression, body: ASTStatement) -> ASTStatement {
        ASTStatement::new(ASTStatementKind::While(ASTWhileStatement { while_keyword, condition, body: Box::new(body) }))
    }

    pub fn return_statement(&mut self, return_keyword: Token, return_value: Option<ASTExpression>, is_top_level: bool) -> ASTStatement {
        ASTStatement::new(ASTStatementKind::Return(ASTReturnStatement { return_keyword, return_value, is_top_level }))
    }

    pub fn func_decl_statement(&mut self, func_token: Token, modifier_tokens: Vec<Token>, identifier: Token, parameters: Vec<FuncDeclParameter>, body: Option<Vec<ASTStatement>>, return_type: Option<ASTFunctionReturnType>) -> ASTStatement {
        ASTStatement::new(ASTStatementKind::FuncDecl(ASTFuncDeclStatement { identifier, parameters, body, return_type, modifier_tokens, func_token }))
    }


    pub fn struct_decl_statement(&mut self, struct_token: Token, identifier: Token, fields: Vec<ASTStructField>, open_brace: Token, close_brace: Token) -> ASTStatement {
        ASTStatement::new(ASTStatementKind::StructDecl(ASTStructDeclStatement { struct_token, identifier, fields, open_brace, close_brace }))
    }

    pub fn module_decl_statement(&mut self, module_token: Token, identifier: Token) -> ASTStatement {
        ASTStatement::new(ASTStatementKind::ModDecl(ASTModDeclStatement { mod_token: module_token, identifier }))
    }

    pub fn number_expression(&mut self, token: Token, number: i64) -> ASTExpression {
        ASTExpression::new(ASTExpressionKind::Number(ASTNumberExpression { number, token }))
    }

    pub fn string_expression(&mut self, open_quote: Token, value: ASTString, close_quote: Token) -> ASTExpression {
        ASTExpression::new(ASTExpressionKind::String(ASTStringExpression { open_quote, close_quote, string: value }))
    }

    pub fn binary_expression(&mut self, operator: ASTBinaryOperator, left: ASTExpression, right: ASTExpression) -> ASTExpression {
        ASTExpression::new(ASTExpressionKind::Binary(ASTBinaryExpression { operator, left: Box::new(left), right: Box::new(right) }))
    }

    pub fn parenthesized_expression(&mut self, left_paren: Token, expression: ASTExpression, right_paren: Token) -> ASTExpression {
        ASTExpression::new(ASTExpressionKind::Parenthesized(ASTParenthesizedExpression { expression: Box::new(expression), left_paren, right_paren }))
    }

    pub fn identifier_expression(&mut self, identifier: QualifiedIdentifier) -> ASTExpression {
        ASTExpression::new(ASTExpressionKind::Identifier(ASTIdentifierExpression { identifier }))
    }

    pub fn unary_expression(&mut self, operator: ASTUnaryOperator, operand: ASTExpression) -> ASTExpression {
        ASTExpression::new(ASTExpressionKind::Unary(ASTUnaryExpression { operator, operand: Box::new(operand) }))
    }

    pub fn assignment_expression(&mut self, assignee: ASTExpression, equals: Token, expression: ASTExpression) -> ASTExpression {
        ASTExpression::new(ASTExpressionKind::Assignment(ASTAssignmentExpression { assignee: Box::new(assignee), expression: Box::new(expression), equals }))
    }

    pub fn boolean_expression(&mut self, token: Token, value: bool) -> ASTExpression {
        ASTExpression::new(ASTExpressionKind::Boolean(ASTBooleanExpression { token, value }))
    }

    pub fn call_expression(&mut self, callee: ASTExpression, left_paren: Token, arguments: Vec<ASTExpression>, right_paren: Token) -> ASTExpression {
        ASTExpression::new(ASTExpressionKind::Call(ASTCallExpression { callee: Box::new(callee), arguments, left_paren, right_paren }))
    }

    pub fn ref_expression(&mut self, ampersand: Token, mut_token: Option<Token>, expression: ASTExpression) -> ASTExpression {
        ASTExpression::new(ASTExpressionKind::Ref(ASTRefExpression { mut_token, ampersand, expr: Box::new(expression) }))
    }

    pub fn deref_expression(&mut self, star: Token, expression: ASTExpression) -> ASTExpression {
        ASTExpression::new(ASTExpressionKind::Deref(ASTDerefExpression { star, expr: Box::new(expression) }))
    }

    pub fn character_expression(&mut self, open_quote: Token, value: char, close_quote: Token) -> ASTExpression {
        ASTExpression::new(ASTExpressionKind::Char(ASTCharExpression { open_quote, value, close_quote }))
    }

    pub fn cast_expression(&mut self, expression: ASTExpression, as_keyword: Token, ty: TypeSyntax) -> ASTExpression {
        ASTExpression::new(ASTExpressionKind::Cast(ASTCastExpression { as_keyword, expr: Box::new(expression), ty }))
    }

    pub fn member_access_expression(&mut self, expression: ASTExpression, access_operator: Token, member: Token) -> ASTExpression {
        ASTExpression::new(ASTExpressionKind::MemberAccess(ASTMemberAccessExpression { expr: Box::new(expression), access_operator, member }))
    }

    pub fn struct_init_expression(&mut self, identifier: QualifiedIdentifier, open_brace: Token, fields: Vec<ASTStructInitField>, close_brace: Token) -> ASTExpression {
        ASTExpression::new(ASTExpressionKind::StructInit(ASTStructInitExpression { identifier, open_brace, fields, close_brace }))
    }

    pub fn error_expression(&mut self, span: TextSpan) -> ASTExpression {
        ASTExpression::new(ASTExpressionKind::Error(span))
    }

    pub fn visit(&self, visitor: &mut dyn ASTVisitor) {
        for statement in &self.statements {
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
    Expression(ASTExpression),
    Let(ASTLetStatement),
    If(ASTIfStatement),
    Block(ASTBlockStatement),
    While(ASTWhileStatement),
    FuncDecl(ASTFuncDeclStatement),
    Return(ASTReturnStatement),
    StructDecl(ASTStructDeclStatement),
    ModDecl(ASTModDeclStatement),
}

#[derive(Debug, Clone)]
pub struct ASTModDeclStatement {
    pub mod_token: Token,
    pub identifier: Token,
}

#[derive(Debug, Clone)]
pub struct ASTStructDeclStatement {
    pub struct_token: Token,
    pub identifier: Token,
    pub open_brace: Token,
    pub close_brace: Token,
    pub fields: Vec<ASTStructField>,
}

#[derive(Debug, Clone)]
pub struct ASTStructField {
    pub identifier: Token,
    pub ty: StaticTypeAnnotation,
}


#[derive(Debug, Clone)]
pub struct ASTReturnStatement {
    pub return_keyword: Token,
    pub return_value: Option<ASTExpression>,
    pub is_top_level: bool,
}

#[derive(Debug, Clone)]
pub struct StaticTypeAnnotation {
    pub colon: Token,
    pub ty: TypeSyntax,
}

#[derive(Debug, Clone)]
pub struct TypeSyntax {
    pub name: QualifiedIdentifier,
    pub ptr: Option<PtrSyntax>,
}

#[derive(Debug, Clone)]
pub struct PtrSyntax {
    pub star: Token,
    pub mut_token: Option<Token>,
}

impl TypeSyntax {
    pub fn new(name: QualifiedIdentifier, ptr: Option<PtrSyntax>) -> Self {
        Self { name, ptr }
    }

    pub fn span(&self) -> TextSpan {
        let id_span = self.name.span();
        let mut spans = vec![&id_span];
        if let Some(ptr) = &self.ptr {
            spans.push(&ptr.star.span);
            if let Some(mut_token) = &ptr.mut_token {
                spans.push(&mut_token.span);
            }
        }
        TextSpan::merge(
            spans
        )
    }
}

impl Display for TypeSyntax {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(ptr) = &self.ptr {
            if ptr.mut_token.is_some() {
                write!(f, "*mut {}", self.name.get_qualified_name())
            } else {
                write!(f, "*{}", self.name.get_qualified_name())
            }
        } else {
            write!(f, "{}", self.name.get_qualified_name())
        }
    }
}

impl StaticTypeAnnotation {
    pub fn new(colon: Token, ty: TypeSyntax) -> Self {
        Self { colon, ty }
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
    pub mut_token: Option<Token>,
    pub identifier: Token,
    pub type_annotation: StaticTypeAnnotation,
}

#[derive(Debug, Clone)]
pub struct ASTFunctionReturnType {
    pub arrow: Token,
    pub ty: TypeSyntax,
}

impl ASTFunctionReturnType {
    pub fn new(arrow: Token, ty: TypeSyntax) -> Self {
        Self { arrow, ty: ty }
    }
}

#[derive(Debug, Clone)]
pub struct ASTFuncDeclStatement {
    pub func_token: Token,
    pub modifier_tokens: Vec<Token>,
    pub identifier: Token,
    pub parameters: Vec<FuncDeclParameter>,
    pub body: Option<Vec<ASTStatement>>,
    pub return_type: Option<ASTFunctionReturnType>,
}

#[derive(Debug, Clone)]
pub struct ASTWhileStatement {
    pub while_keyword: Token,
    pub condition: ASTExpression,
    pub body: Box<ASTStatement>,
}

#[derive(Debug, Clone)]
pub struct ASTBlockStatement {
    pub open_brace: Token,
    pub statements: Vec<ASTStatement>,
    pub close_brace: Token,
}

#[derive(Debug, Clone)]
pub struct ASTElseStatement {
    pub else_keyword: Token,
    pub else_statement: Box<ASTStatement>,
}

impl ASTElseStatement {
    pub fn new(else_keyword: Token, else_statement: ASTStatement) -> Self {
        ASTElseStatement { else_keyword, else_statement: Box::new(else_statement) }
    }
}

#[derive(Debug, Clone)]
pub struct ASTIfStatement {
    pub if_keyword: Token,
    pub condition: ASTExpression,
    pub then_branch: Box<ASTStatement>,
    pub else_branch: Option<ASTElseStatement>,
}

#[derive(Debug, Clone)]
pub struct ASTLetStatement {
    pub mut_token: Option<Token>,
    pub identifier: Token,
    pub initializer: ASTExpression,
    pub type_annotation: Option<StaticTypeAnnotation>,
}

#[derive(Debug, Clone)]
pub struct ASTStatement {
    pub kind: ASTStatementKind,
}

impl ASTStatement {
    pub fn new(kind: ASTStatementKind) -> Self {
        ASTStatement { kind }
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

    pub fn span(&self) -> TextSpan {
        match &self.kind
        {
            ASTStatementKind::Expression(expr) => expr.span(),
            ASTStatementKind::Let(stmt) => {
                let init_span = &stmt.initializer.span();
                let mut spans = vec![
                    &stmt.identifier.span,
                    &init_span,
                ];
                if let Some(type_annotation) = &stmt.type_annotation {
                    spans.push(&type_annotation.colon.span);
                    // let id_span = type_annotation.ty.name.span();
                    // spans.push(&id_span);
                }
                TextSpan::merge(
                    spans
                )
            }
            ASTStatementKind::If(stmt) => {
                let cond_span = stmt.condition.span();
                let then_branch_span = stmt.then_branch.span();
                let mut spans = vec![
                    &stmt.if_keyword.span,
                    &cond_span,
                    &then_branch_span,
                ];

                if let Some(else_branch) = &stmt.else_branch {
                    spans.push(&else_branch.else_keyword.span);
                    let span1 = else_branch.else_statement.span();
                    // todo: fix this
                    // spans.push(&span1);
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
                    stmt.statements.iter().map(|stmt| stmt.span()));
                TextSpan::merge(
                    spans.iter().map(|span| span).collect()
                )
            }
            ASTStatementKind::While(stmt) => {
                let cond_span = stmt.condition.span();
                let body_span = stmt.body.span();
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
                            // let id_span = parameter.type_annotation.ty.name.span();
                            // spans.push(&id_span);
                        }
                        FuncDeclParameter::Self_(token) => {
                            spans.push(&token.span);
                        }
                    }
                }
                if let Some(return_type) = &stmt.return_type {
                    spans.push(&return_type.arrow.span);
                    // spans.push(&return_type.ty.name.span());
                }
                let body_spans = stmt.body.as_ref().map(|body| {
                    body.iter().map(|stmt| stmt.span()).collect::<Vec<_>>()
                });
                if let Some(body_span) = body_spans.as_ref() {
                    spans.extend(body_span);
                }
                TextSpan::merge(
                    spans
                )
            }
            ASTStatementKind::Return(stmt) => {
                let mut spans = vec![
                    &stmt.return_keyword.span,
                ];
                let return_value_span = stmt.return_value.as_ref().map(|return_value| return_value.span());
                if let Some(return_value_span) = return_value_span.as_ref() {
                    spans.push(return_value_span);
                }
                TextSpan::merge(
                    spans
                )
            }
            ASTStatementKind::StructDecl(stmt) => {
                let spans = vec![
                    &stmt.struct_token.span,
                    &stmt.identifier.span,
                    &stmt.open_brace.span,
                    &stmt.close_brace.span,
                ];
                TextSpan::merge(
                    spans
                )
            }
            ASTStatementKind::ModDecl(stmt) => {
                let spans = vec![
                    &stmt.mod_token.span,
                    &stmt.identifier.span,
                ];
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
    Char(
        ASTCharExpression
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
    Ref(
        ASTRefExpression
    ),
    Deref(
        ASTDerefExpression
    ),
    Cast(
        ASTCastExpression
    ),
    MemberAccess(
        ASTMemberAccessExpression
    ),
    StructInit(
        ASTStructInitExpression
    ),
    Error(
        TextSpan
    ),
}

#[derive(Debug, Clone)]
pub struct ASTStructInitExpression {
    pub identifier: QualifiedIdentifier,
    pub open_brace: Token,
    pub close_brace: Token,
    pub fields: Vec<ASTStructInitField>,
}

#[derive(Debug, Clone)]
pub struct ASTStructInitField {
    pub identifier: Token,
    pub colon: Token,
    pub initializer: Box<ASTExpression>,
}

impl ASTStructInitField {
    pub fn span(&self) -> TextSpan {
        TextSpan::merge(
            vec![
                &self.identifier.span,
                &self.colon.span,
                &self.initializer.span(),
            ]
        )
    }
}

#[derive(Debug, Clone)]
pub struct ASTMemberAccessExpression {
    pub expr: Box<ASTExpression>,
    pub access_operator: Token,
    pub member: Token,
}

#[derive(Debug, Clone)]
pub struct ASTCastExpression {
    pub expr: Box<ASTExpression>,
    pub as_keyword: Token,
    pub ty: TypeSyntax,
}

#[derive(Debug, Clone)]
pub struct ASTCharExpression {
    pub open_quote: Token,
    pub value: char,
    pub close_quote: Token,
}

#[derive(Debug, Clone)]
pub struct ASTRefExpression {
    pub ampersand: Token,
    pub mut_token: Option<Token>,
    pub expr: Box<ASTExpression>,
}

#[derive(Debug, Clone)]
pub struct ASTDerefExpression {
    pub star: Token,
    pub expr: Box<ASTExpression>,
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
    // Expression(ASTExpression),
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
    pub callee: Box<ASTExpression>,
    pub left_paren: Token,
    pub arguments: Vec<ASTExpression>,
    pub right_paren: Token,
}

#[derive(Debug, Clone)]
pub struct ASTBooleanExpression {
    pub value: bool,
    pub token: Token,
}

#[derive(Debug, Clone)]
pub struct ASTAssignmentExpression {
    pub assignee: Box<ASTExpression>,
    pub equals: Token,
    pub expression: Box<ASTExpression>,
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
    pub operand: Box<ASTExpression>,
}

#[derive(Debug, Clone)]
pub struct QualifiedIdentifier {
    pub parts: Vec<Token>,
}

impl QualifiedIdentifier {
    pub fn new(parts: Vec<Token>) -> Self {
        QualifiedIdentifier { parts }
    }

    pub fn span(&self) -> TextSpan {
        TextSpan::merge(self.parts.iter().map(|p| &p.span).collect())
    }

    pub fn is_qualified(&self) -> bool {
        self.parts.len() > 1
    }

    pub fn get_qualified_name(&self) -> String {
        let mut result = String::new();
        for (i, part) in self.parts.iter().enumerate() {
            if i > 0 {
                result.push_str("::");
            }
            result.push_str(&part.span.literal);
        }
        result
    }

    pub fn get_unqualified_name(&self) -> &Token {
        &self.parts[self.parts.len() - 1]
    }
}

impl Display for QualifiedIdentifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for (i, part) in self.parts.iter().enumerate() {
            if i > 0 {
                write!(f, "::")?;
            }
            write!(f, "{}", part.span.literal)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ASTIdentifierExpression {
    pub identifier: QualifiedIdentifier,
}


#[derive(Debug, Clone)]
pub enum ASTBinaryOperatorKind {
    // Arithmetic
    Plus,
    Minus,
    Multiply,
    Divide,
    Power,
    Modulo,
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
            ASTBinaryOperatorKind::Modulo => 19,
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
    pub left: Box<ASTExpression>,
    pub operator: ASTBinaryOperator,
    pub right: Box<ASTExpression>,
}

#[derive(Debug, Clone)]
pub struct ASTNumberExpression {
    pub number: i64,
    pub token: Token,
}

#[derive(Debug, Clone)]
pub struct ASTParenthesizedExpression {
    pub left_paren: Token,
    pub expression: Box<ASTExpression>,
    pub right_paren: Token,
}

#[derive(Debug, Clone)]
pub struct ASTExpression {
    pub kind: ASTExpressionKind,
}

impl ASTExpression {
    pub fn new(kind: ASTExpressionKind) -> Self {
        ASTExpression { kind }
    }

    pub fn span(&self) -> TextSpan {
        match &self.kind {
            ASTExpressionKind::Number(expr) => expr.token.span.clone(),
            ASTExpressionKind::Binary(expr) => {
                let left = &expr.left.span();
                let operator = &expr.operator.token.span;
                let right = &expr.right.span();
                TextSpan::merge(vec![left, operator, right])
            }
            ASTExpressionKind::Unary(expr) => {
                let operator = &expr.operator.token.span;
                let operand = &expr.operand.span();
                TextSpan::merge(vec![operator, operand])
            }
            ASTExpressionKind::Parenthesized(expr) => {
                let open_paren = &expr.left_paren.span;
                let expression = &expr.expression.span();
                let close_paren = &expr.right_paren.span;
                TextSpan::merge(vec![open_paren, expression, close_paren])
            }
            ASTExpressionKind::Identifier(expr) => expr.identifier.span(),
            ASTExpressionKind::Assignment(expr) => {
                let identifier = &expr.assignee.span();
                let equals = &expr.equals.span;
                let expression = &expr.expression.span();
                TextSpan::merge(vec![identifier, equals, expression])
            }
            ASTExpressionKind::Boolean(expr) => expr.token.span.clone(),
            ASTExpressionKind::Call(expr) => {
                let expr_span = &expr.callee.span();
                let left_paren = &expr.left_paren.span;
                let right_paren = &expr.right_paren.span;
                let mut spans = vec![expr_span, left_paren, right_paren];
                let argument_spans: Vec<TextSpan> = expr.arguments.iter().map(|arg| arg.span()).collect();
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
            ASTExpressionKind::Ref(expr) => {
                let span1 = expr.expr.span();
                let spans = vec![&expr.ampersand.span, &span1];
                TextSpan::merge(spans)
            }
            ASTExpressionKind::Deref(expr) => {
                let span2 = expr.expr.span();
                let spans = vec![&expr.star.span, &span2];
                TextSpan::merge(spans)
            }
            ASTExpressionKind::Char(expr) => {
                let spans = vec![&expr.open_quote.span, &expr.close_quote.span];
                TextSpan::merge(spans)
            }
            ASTExpressionKind::Cast(expr) => {
                let span1 = &expr.as_keyword.span;
                let span2 = &expr.ty.span();
                let span3 = expr.expr.span();
                TextSpan::merge(vec![&span1, &span2, &span3])
            }
            ASTExpressionKind::MemberAccess(expr) => {
                let span1 = expr.expr.span();
                let span2 = &expr.access_operator.span;
                let span3 = &expr.member.span;
                TextSpan::merge(vec![&span1, &span2, &span3])
            }
            ASTExpressionKind::StructInit(expr) => {
                let span2 = &expr.open_brace.span;
                let span3 = &expr.close_brace.span;
                let mut spans = vec![span2, span3];
                let field_spans: Vec<TextSpan> = expr.fields.iter().map(|field| field.span()).collect();
                for span in &field_spans {
                    spans.push(span);
                }
                TextSpan::merge(spans)
            }
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
            let mut verifier = ASTVerifier { expected, actual: Vec::new(), ast: compilation_unit.source_tree };
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
