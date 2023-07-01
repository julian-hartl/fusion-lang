use std::fmt::{Display, Formatter};

use termion::color::{Fg, Reset};

use expr::{AssignExpr, BinOperator, BinOperatorKind, ASTStructInitField, UnOperator, UnOperatorKind, BinExpr, BoolExpr, CallExpr, CastExpr, CharExpr, DerefExpr, Expr, ExprKind, IdenExpr, IndexExpr, MemberAccessExpr, NumberExpr, ParenExpr, RefExpr, StringExpr, StructInitExpr, UnaryExpr};
use lexer::{StringToken, StringTokenPart};
use lexer::token::Token;
use printer::ASTPrinter;
use stmt::{LetStmt, ParameterSyntax, ReturnStmt, Stmt, StmtKind};
use visitor::ASTVisitor;

use crate::ast::expr::{BlockExpr, ElseBranch, IfExpr, WhileExpr};
use crate::text::span::TextSpan;

pub mod parser;
pub mod visitor;
pub mod printer;
pub mod expr;
pub mod stmt;
pub mod lexer;

pub trait ASTNode {
    fn span(&self) -> TextSpan;
}

#[derive(Debug)]
pub struct Module {
    pub ast: Ast,
}

#[derive(Debug)]
pub struct Item {
    pub kind: ItemKind,
}

impl Item {
    pub(crate) fn new(kind: ItemKind) -> Self {
        Self {
            kind,
        }
    }
}

impl ASTNode for Item {
    fn span(&self) -> TextSpan {
        match &self.kind {
            ItemKind::FunctionDeclaration(stmt) => {
                let mut spans = vec![
                    &stmt.identifier.span,
                ];
                for parameter in &stmt.parameters {
                    spans.push(&parameter.identifier.span);
                    spans.push(&parameter.type_annotation.colon.span);
                }
                if let Some(return_type) = &stmt.return_type {
                    spans.push(&return_type.arrow.span);
                    // spans.push(&return_type.ty.name.span());
                }
                let body_spans = stmt.body.as_ref().map(|body| {
                    body.span()
                });
                if let Some(body_span) = body_spans.as_ref() {
                    spans.push(body_span);
                }
                TextSpan::merge(
                    spans
                )
            }
            ItemKind::ModuleDeclaration(stmt) => {
                let spans = vec![
                    &stmt.mod_token.span,
                    &stmt.identifier.span,
                ];
                TextSpan::merge(
                    spans
                )
            }
            ItemKind::StructDeclaration(stmt) => {
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
        }
    }
}

#[derive(Debug)]
pub enum ItemKind {
    FunctionDeclaration(FunctionDeclaration),
    ModuleDeclaration(ModuleDeclaration),
    StructDeclaration(StructDeclaration),
    NotAllowed(Stmt),
}

#[derive(Debug)]
pub struct FunctionDeclaration {
    pub func_token: Token,
    pub modifier_tokens: Vec<Token>,
    pub identifier: Token,
    pub parameters: Vec<ParameterSyntax>,
    pub body: Option<BlockExpr>,
    pub return_type: Option<ReturnTypeSyntax>,
}

#[derive(Debug, Clone)]
pub struct ReturnTypeSyntax {
    pub arrow: Token,
    pub ty: TypeSyntax,
}

impl ReturnTypeSyntax {
    pub fn new(arrow: Token, ty: TypeSyntax) -> Self {
        Self { arrow, ty }
    }
}

#[derive(Debug)]
pub struct ModuleDeclaration {
    pub mod_token: Token,
    pub identifier: Token,
}

#[derive(Debug)]
pub struct StructDeclaration {
    pub struct_token: Token,
    pub identifier: Token,
    pub open_brace: Token,
    pub close_brace: Token,
    pub fields: Vec<StructField>,
}


#[derive(Debug)]
pub struct Ast {
    pub items: Vec<Item>,
}

impl Ast {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }


    pub fn expression_statement(&mut self, expr: Expr) -> Stmt {
        Stmt::new(StmtKind::Expr(
            expr
        ))
    }

    pub fn let_statement(&mut self, mut_token: Option<Token>, identifier: Token, initializer: Expr, type_annotation: Option<StaticTypeAnnotation>) -> Stmt {
        Stmt::new(
            StmtKind::Let(LetStmt { mut_token, identifier, initializer, type_annotation })
        )
    }

    pub fn if_expr(&mut self, if_keyword: Token, condition: Expr, then: BlockExpr, else_branch: Option<ElseBranch>) -> Expr {
        Expr::new(
            ExprKind::If(IfExpr
            {
                if_keyword,
                condition: Box::new(condition),
                then_branch: Box::new(then),
                else_branch,
            })
        )
    }

    pub fn block_expression(&mut self, open_brace: Token, statements: Vec<Stmt>, close_brace: Token) -> Expr {
        Expr::new(ExprKind::Block(BlockExpr { stmts: statements, open_brace, close_brace }))
    }

    pub fn while_expression(&mut self, while_keyword: Token, condition: Expr, body: BlockExpr) -> Expr {
        Expr::new(ExprKind::While(WhileExpr { while_keyword, condition: Box::new(condition), body: Box::new(body) }))
    }

    pub fn return_statement(&mut self, return_keyword: Token, return_value: Option<Expr>, is_top_level: bool) -> Stmt {
        Stmt::new(StmtKind::Return(ReturnStmt { return_keyword, expr: return_value, is_top_level }))
    }


    pub fn number_expression(&mut self, token: Token, number: i64, ty: Option<Token>) -> Expr {
        Expr::new(ExprKind::Number(NumberExpr { number, token, size_specifier: ty }))
    }

    pub fn string_expression(&mut self, open_quote: Token, value: StringToken, close_quote: Token) -> Expr {
        Expr::new(ExprKind::String(StringExpr { open_quote, close_quote, string: value }))
    }

    pub fn binary_expression(&mut self, operator: BinOperator, left: Expr, right: Expr) -> Expr {
        Expr::new(ExprKind::Binary(BinExpr { operator, left: Box::new(left), right: Box::new(right) }))
    }

    pub fn parenthesized_expression(&mut self, left_paren: Token, expression: Expr, right_paren: Token) -> Expr {
        Expr::new(ExprKind::Parenthesized(ParenExpr { expr: Box::new(expression), left_paren, right_paren }))
    }

    pub fn identifier_expression(&mut self, identifier: QualifiedIdentifier) -> Expr {
        Expr::new(ExprKind::Identifier(IdenExpr { identifier }))
    }

    pub fn unary_expression(&mut self, operator: UnOperator, operand: Expr) -> Expr {
        Expr::new(ExprKind::Unary(UnaryExpr { operator, operand: Box::new(operand) }))
    }

    pub fn assignment_expression(&mut self, assignee: Expr, equals: Token, expression: Expr) -> Expr {
        Expr::new(ExprKind::Assignment(AssignExpr { left: Box::new(assignee), right: Box::new(expression), equals }))
    }

    pub fn boolean_expression(&mut self, token: Token, value: bool) -> Expr {
        Expr::new(ExprKind::Boolean(BoolExpr { token, value }))
    }

    pub fn call_expression(&mut self, callee: Expr, left_paren: Token, arguments: Vec<Expr>, right_paren: Token) -> Expr {
        Expr::new(ExprKind::Call(CallExpr { callee: Box::new(callee), arguments, left_paren, right_paren }))
    }

    pub fn ref_expression(&mut self, ampersand: Token, mut_token: Option<Token>, expression: Expr) -> Expr {
        Expr::new(ExprKind::Ref(RefExpr { mut_token, ampersand, expr: Box::new(expression) }))
    }

    pub fn deref_expression(&mut self, star: Token, expression: Expr) -> Expr {
        Expr::new(ExprKind::Deref(DerefExpr { star, expr: Box::new(expression) }))
    }

    pub fn character_expression(&mut self, open_quote: Token, value: char, close_quote: Token) -> Expr {
        Expr::new(ExprKind::Char(CharExpr { open_quote, value, close_quote }))
    }

    pub fn cast_expression(&mut self, expression: Expr, as_keyword: Token, ty: TypeSyntax) -> Expr {
        Expr::new(ExprKind::Cast(CastExpr { as_keyword, expr: Box::new(expression), ty }))
    }

    pub fn member_access_expression(&mut self, expression: Expr, access_operator: Token, member: Token) -> Expr {
        Expr::new(ExprKind::MemberAccess(MemberAccessExpr { expr: Box::new(expression), access_operator, member }))
    }

    pub fn struct_init_expression(&mut self, identifier: QualifiedIdentifier, open_brace: Token, fields: Vec<ASTStructInitField>, close_brace: Token) -> Expr {
        Expr::new(ExprKind::StructInit(StructInitExpr { identifier, open_brace, fields, close_brace }))
    }

    pub fn index_expression(&mut self, expression: Expr, open_bracket: Token, index: Expr, close_bracket: Token) -> Expr {
        Expr::new(ExprKind::Index(IndexExpr { target: Box::new(expression), open_bracket, index: Box::new(index), close_bracket }))
    }

    pub fn error_expression(&mut self, span: TextSpan) -> Expr {
        Expr::new(ExprKind::Error(span))
    }

    pub fn visit(&self, visitor: &mut dyn ASTVisitor) {
        for item in &self.items {
            visitor.visit_item(item);
        }
    }

    pub fn visualize(&self) {
        let mut printer = ASTPrinter::new(
            self
        );
        self.visit(&mut printer);
        println!("{}", printer.result);
    }
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub identifier: Token,
    pub ty: StaticTypeAnnotation,
}

#[derive(Debug, Clone)]
pub struct StaticTypeAnnotation {
    pub colon: Token,
    pub ty: TypeSyntax,
}

#[derive(Debug, Clone)]
pub struct TypeSyntax {
    pub name: QualifiedIdentifier,
    pub ptr: Option<Vec<PtrSyntax>>,
}

#[derive(Debug, Clone)]
pub struct PtrSyntax {
    pub star: Token,
    pub mut_token: Option<Token>,
}

impl TypeSyntax {
    pub fn new(name: QualifiedIdentifier, ptr: Option<Vec<PtrSyntax>>) -> Self {
        Self { name, ptr }
    }

    pub fn span(&self) -> TextSpan {
        let id_span = self.name.span();
        let mut spans = vec![&id_span];
        if let Some(ptr) = &self.ptr {
            for ptr in ptr {
                spans.push(&ptr.star.span);
                if let Some(mut_token) = &ptr.mut_token {
                    spans.push(&mut_token.span);
                }
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
            for ptr in ptr {
                if ptr.mut_token.is_some() {
                    write!(f, "*mut")?;
                } else {
                    write!(f, "*")?;
                }
            }
            write!(f, " {}", self.name.get_qualified_name())
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

#[cfg(test)]
mod test {
    use crate::ast::{Ast, FunctionDeclaration, ItemKind};
    use crate::ast::expr::{AssignExpr, BinExpr, BlockExpr, BoolExpr, CallExpr, CastExpr, CharExpr, DerefExpr, Expr, ExprKind, IdenExpr, IndexExpr, MemberAccessExpr, NumberExpr, ParenExpr, RefExpr, StringExpr, StructInitExpr, UnaryExpr};
    use crate::ast::stmt::{LetStmt, ReturnStmt, Stmt, StmtKind};
    use crate::compilation::{CompilationUnit, Parseable};
    use crate::text::SourceText;
    use crate::text::span::TextSpan;

    use super::visitor::ASTVisitor;

    #[derive(Debug, PartialEq, Eq)]
    struct ASTNode {
        kind: ASTNodeKind,
        children: Vec<ASTNode>,
    }

    impl ASTNode {
        pub fn new(kind: ASTNodeKind, children: Vec<ASTNode>) -> Self {
            ASTNode { kind, children }
        }

        pub fn empty(kind: ASTNodeKind) -> Self {
            ASTNode { kind, children: vec![] }
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    enum ASTNodeKind {
        Number(i64),
        String(String),
        Char(char),
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
        While,
        Return,
        Call,
        Ref,
        Deref,
        Cast,
        Type(String),
        MemberAccess(String),
        StructInit,
        Index,
        // Items
        Func,
        Struct(String, Vec<(String, String)>),
        Mod,
    }

    impl ASTNodeKind {
        pub fn into_node(self, children: Vec<ASTNode>) -> ASTNode {
            ASTNode::new(self, children)
        }

        pub fn no_children(self) -> ASTNode {
            ASTNode::empty(self)
        }
    }


    #[derive(Debug, PartialEq, Eq)]
    struct CompressedAST {
        nodes: Vec<ASTNode>,
    }

    impl CompressedAST {
        pub fn len(&self) -> usize {
            self.nodes.len()
        }

        pub fn clear(&mut self) {
            self.nodes.clear();
        }

        pub fn push_no_children(&mut self, kind: ASTNodeKind) {
            self.nodes.push(ASTNode::new(kind, vec![]));
        }

        pub fn push(&mut self, kind: ASTNodeKind, children: Vec<ASTNode>) {
            self.nodes.push(ASTNode::new(kind, children));
        }
    }

    struct ASTVerifier {
        expected: CompressedAST,
        actual: CompressedAST,
        ast: Ast,
    }

    impl ASTVerifier {
        pub fn new(input: &str, expected: Vec<ASTNode>) -> Self {
            struct TestParseable {
                input: String,
            }
            impl Parseable for TestParseable {
                type Error = std::io::Error;

                fn get_content(&self) -> anyhow::Result<String, Self::Error> {
                    Ok(self.input.clone())
                }

                fn join(&self, path: &str) -> Self {
                    todo!()
                }

                fn describes_module(&self) -> bool {
                    todo!()
                }

                fn with_extension(&self, ext: &str) -> Self {
                    todo!()
                }
            }
            let parseable = TestParseable {
                input: input.to_string(),
            };
            let compilation_unit = CompilationUnit::compile(&parseable).expect("Failed to compile");
            let mut verifier = ASTVerifier {
                expected: CompressedAST {
                    nodes: expected
                },
                actual: CompressedAST {
                    nodes: vec![]
                },
                ast: compilation_unit.source_tree.asts.into_iter().map(|(_, (ast, _))| ast).next().unwrap(),
            };
            verifier.flatten_ast();
            verifier
        }

        fn flatten_ast(&mut self) {
            self.actual.clear();
            self.gather_nodes();
        }

        pub fn verify(&self) {
            assert_eq!(self.expected.len(), self.actual.len(), "Expected {} nodes, but got {}. Actual nodes: {:?}", self.expected.len(), self.actual.len(), self.actual);

            for (index, (expected, actual)) in self.expected.nodes.iter().zip(
                self.actual.nodes.iter()
            ).enumerate() {
                assert_eq!(expected, actual, "Expected {:?} at index {}, but got {:?}", expected, index, actual);
            }
        }
    }

    impl ASTVerifier {
        pub fn gather_nodes(&mut self) {
            for item in self.ast.items.iter() {
                match &item.kind {
                    ItemKind::FunctionDeclaration(func_decl) => {
                        let children = match func_decl.body.as_ref() {
                            Some(body) => self.g_block_expr(body),
                            None => vec![]
                        };
                        self.actual.push(ASTNodeKind::Func, children);
                    }
                    ItemKind::ModuleDeclaration(_) => {
                        self.actual.push_no_children(ASTNodeKind::Mod);
                    }
                    ItemKind::StructDeclaration(decl) => {
                        let fields = decl.fields.iter().map(|field| {
                            (field.identifier.span.literal.clone(), field.ty.ty.to_string())
                        }).collect();
                        self.actual.push_no_children(ASTNodeKind::Struct(decl.identifier.span.literal.clone(), fields));
                    }
                }
            }
        }

        fn g_expr(&mut self, expr: &Expr) -> Vec<ASTNode> {
            match &expr.kind {
                ExprKind::Number(NumberExpr {
                                     number,
                                     ..
                                 }) => {
                    vec![ASTNode::new(ASTNodeKind::Number(*number), vec![])]
                }
                ExprKind::String(StringExpr {
                                     string,
                                     ..
                                 }) => {
                    vec![ASTNode::new(ASTNodeKind::String(string.to_raw_string()), vec![])]
                }
                ExprKind::Char(CharExpr {
                                   value,
                                   ..
                               }) => {
                    vec![ASTNode::new(ASTNodeKind::Char(*value), vec![])]
                }
                ExprKind::Binary(BinExpr {
                                     left,
                                     right,
                                     ..
                                 }) => {
                    let mut children = self.g_expr(left);
                    children.extend(self.g_expr(right));
                    vec![ASTNode::new(ASTNodeKind::Binary, children)]
                }
                ExprKind::Unary(UnaryExpr {
                                    operand,
                                    ..
                                }) => {
                    let mut children = self.g_expr(operand);
                    vec![ASTNode::new(ASTNodeKind::Unary, children)]
                }
                ExprKind::Parenthesized(ParenExpr {
                                            expr,
                                            ..
                                        }) => {
                    let mut children = self.g_expr(expr);
                    vec![ASTNode::new(ASTNodeKind::Parenthesized, children)]
                }
                ExprKind::Identifier(IdenExpr {
                                         identifier,
                                         ..
                                     }) => {
                    vec![ASTNode::new(ASTNodeKind::Variable(identifier.get_qualified_name()), vec![])]
                }
                ExprKind::Assignment(AssignExpr {
                                         left,
                                         right,
                                         ..
                                     }) => {
                    let mut children = self.g_expr(left);
                    children.extend(self.g_expr(right));
                    vec![ASTNode::new(ASTNodeKind::Assignment, children)]
                }
                ExprKind::Boolean(BoolExpr {
                                      value,
                                      ..
                                  }) => {
                    vec![ASTNode::new(ASTNodeKind::Boolean(*value), vec![])]
                }
                ExprKind::Call(CallExpr {
                                   callee,
                                   arguments,
                                   ..
                               }) => {
                    let mut children = self.g_expr(callee);
                    for arg in arguments {
                        children.extend(self.g_expr(arg));
                    }
                    vec![ASTNode::new(ASTNodeKind::Call, children)]
                }
                ExprKind::Ref(RefExpr {
                                  expr,
                                  ..
                              }) => {
                    let mut children = self.g_expr(expr);
                    vec![ASTNode::new(ASTNodeKind::Ref, children)]
                }
                ExprKind::Deref(DerefExpr {
                                    expr,
                                    ..
                                }) => {
                    let mut children = self.g_expr(expr);
                    vec![ASTNode::new(ASTNodeKind::Deref, children)]
                }
                ExprKind::Cast(CastExpr {
                                   expr,
                                   ty,
                                   ..
                               }) => {
                    let mut children = self.g_expr(expr);
                    children.push(ASTNode::new(ASTNodeKind::Type(ty.to_string()), vec![]));
                    vec![ASTNode::new(ASTNodeKind::Cast, children)]
                }
                ExprKind::MemberAccess(MemberAccessExpr {
                                           expr,
                                           member,
                                           ..
                                       }) => {
                    vec![ASTNode::empty(ASTNodeKind::MemberAccess(
                        member.span.literal.clone()
                    ))]
                }
                ExprKind::StructInit(StructInitExpr {
                                         fields,
                                         ..
                                     }) => {
                    let mut children = vec![];
                    for field in fields {
                        children.extend(self.g_expr(&field.initializer));
                    }
                    vec![ASTNode::new(ASTNodeKind::StructInit, children)]
                }
                ExprKind::Index(IndexExpr {
                                    target,
                                    index,
                                    ..
                                }) => {
                    let mut children = self.g_expr(target);
                    children.extend(self.g_expr(index));
                    vec![ASTNode::new(ASTNodeKind::Index, children)]
                }
                ExprKind::Error(_) => {
                    unreachable!()
                }
                ExprKind::Block(block) => {
                    self.g_block_expr(block)
                }
                ExprKind::While(while_expr) => {
                    let mut children = self.g_expr(&while_expr.condition);
                    children.extend(self.g_block_expr(&while_expr.body));
                    vec![ASTNode::new(ASTNodeKind::While, children)]
                }
                ExprKind::If(if_expr) => {
                    let mut children = self.g_expr(&if_expr.condition);
                    children.extend(self.g_block_expr(&if_expr.then_branch));
                    if let Some(else_branch) = &if_expr.else_branch {
                        children.extend(self.g_block_expr(&else_branch.expr));
                    }
                    vec![ASTNode::new(ASTNodeKind::If, children)]
                }
            }
        }
        fn g_block_expr(&mut self, block: &BlockExpr) -> Vec<ASTNode> {
            let mut children = vec![];
            for stmt in &block.stmts {
                children.extend(self.g_stmt(stmt));
            }
            vec![ASTNode::new(ASTNodeKind::Block, children)]
        }

        fn g_stmt(&mut self, stmt: &Stmt) -> Vec<ASTNode> {
            match &stmt.kind {
                StmtKind::Expr(expr) => {
                    self.g_expr(expr)
                }
                StmtKind::Let(let_stmt) => {
                    let mut children = self.g_expr(&let_stmt.initializer);
                    vec![ASTNode::new(ASTNodeKind::Let, children)]
                }
                StmtKind::Return(return_stmt) => {
                    let mut children = return_stmt.expr.as_ref().map(|expr| self.g_expr(expr)).unwrap_or_else(|| vec![]);
                    vec![ASTNode::new(ASTNodeKind::Return, children)]
                }
            }
        }
    }


    fn assert_tree(input: &str, expected: Vec<ASTNode>) {
        let verifier = ASTVerifier::new(input, expected);
        verifier.verify();
    }

    #[test]
    pub fn should_parse_basic_binary_expression() {
        let input = "let a = 1 + 2";
        let expected = vec![
            ASTNodeKind::Let.into_node(
                vec![
                    ASTNodeKind::Binary.into_node(
                        vec![
                            ASTNodeKind::Number(1).no_children(),
                            ASTNodeKind::Number(2).no_children(),
                        ]
                    )
                ]
            )
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_parenthesized_binary_expression() {
        let input = "let a = (1 + 2) * 3";
        let expected = vec![
            ASTNodeKind::Let
                .into_node(
                    vec![
                        ASTNodeKind::Binary
                            .into_node(
                                vec![
                                    ASTNodeKind::Parenthesized
                                        .into_node(
                                            vec![
                                                ASTNodeKind::Binary
                                                    .into_node(
                                                        vec![
                                                            ASTNodeKind::Number(1).no_children(),
                                                            ASTNodeKind::Number(2).no_children(),
                                                        ]
                                                    )
                                            ]
                                        ),
                                    ASTNodeKind::Number(3).no_children(),
                                ]
                            )
                    ]
                )
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_parenthesized_binary_expression_with_variable() {
        let input = "\
        let b = 1
        let a = (1 + 2) * b";
        let expected = vec![
            ASTNodeKind::Let.into_node(
                vec![
                    ASTNodeKind::Number(1).no_children(),
                ]
            ),
            ASTNodeKind::Let.into_node(
                vec![
                    ASTNodeKind::Binary.into_node(
                        vec![
                            ASTNodeKind::Parenthesized.into_node(
                                vec![
                                    ASTNodeKind::Binary.into_node(
                                        vec![
                                            ASTNodeKind::Number(1).no_children(),
                                            ASTNodeKind::Number(2).no_children(),
                                        ]
                                    )
                                ]
                            ),
                            ASTNodeKind::Variable("b".to_string()).no_children(),
                        ]
                    )
                ]
            ),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_parenthesized_binary_expression_with_variable_and_number() {
        let input = "\
        let b = 1
        let a = (1 + 2) * b + 3";
        let expected = vec![
            ASTNodeKind::Let
                .into_node(
                    vec![
                        ASTNodeKind::Number(1).no_children(),
                    ]
                ),
            ASTNodeKind::Let
                .into_node(
                    vec![
                        ASTNodeKind::Binary
                            .into_node(
                                vec![
                                    ASTNodeKind::Binary
                                        .into_node(
                                            vec![
                                                ASTNodeKind::Parenthesized
                                                    .into_node(
                                                        vec![
                                                            ASTNodeKind::Binary
                                                                .into_node(
                                                                    vec![
                                                                        ASTNodeKind::Number(1).no_children(),
                                                                        ASTNodeKind::Number(2).no_children(),
                                                                    ]
                                                                )
                                                        ]
                                                    ),
                                                ASTNodeKind::Variable("b".to_string()).no_children(),
                                            ]
                                        ),
                                    ASTNodeKind::Number(3).no_children(),
                                ]
                            )
                    ]
                ),
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_bitwise_and() {
        let input = "let a = 1 & 2";
        let expected = vec![
            ASTNodeKind::Let
                .into_node(
                    vec![
                        ASTNodeKind::Binary
                            .into_node(
                                vec![
                                    ASTNodeKind::Number(1).no_children(),
                                    ASTNodeKind::Number(2).no_children(),
                                ]
                            )
                    ]
                )
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_bitwise_or() {
        let input = "let a = 1 | 2";
        let expected = vec![
            ASTNodeKind::Let.into_node(
                vec![
                    ASTNodeKind::Binary.into_node(
                        vec![
                            ASTNodeKind::Number(1).no_children(),
                            ASTNodeKind::Number(2).no_children(),
                        ]
                    )
                ]
            )
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_bitwise_xor() {
        let input = "let a = 1 ^ 2";
        let expected = vec![
            ASTNodeKind::Let.into_node(
                vec![
                    ASTNodeKind::Binary.into_node(
                        vec![
                            ASTNodeKind::Number(1).no_children(),
                            ASTNodeKind::Number(2).no_children(),
                        ]
                    )
                ]
            )
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_bitwise_not() {
        let input = "let a = ~1";
        let expected = vec![
            ASTNodeKind::Let.into_node(
                vec![
                    ASTNodeKind::Unary.into_node(
                        vec![
                            ASTNodeKind::Number(1).no_children(),
                        ]
                    )
                ]
            )
        ];

        assert_tree(input, expected);
    }

    #[test]
    pub fn should_parse_negation() {
        let input = "let a = -1";
        let expected = vec![
            ASTNodeKind::Let.into_node(
                vec![
                    ASTNodeKind::Unary.into_node(
                        vec![
                            ASTNodeKind::Number(1).no_children(),
                        ]
                    )
                ]
            )
        ];

        assert_tree(input, expected);
    }


    #[test]
    pub fn should_parse_hilarious_amount_of_unary_operators() {
        let input = "let a = -1 + -2 * -3 * ------4";
        let expected = vec![
            ASTNodeKind::Let.into_node(
                vec![
                    ASTNodeKind::Binary.into_node(
                        vec![
                            ASTNodeKind::Unary.into_node(
                                vec![
                                    ASTNodeKind::Number(1).no_children(),
                                ]
                            ),
                            ASTNodeKind::Binary.into_node(
                                vec![
                                    ASTNodeKind::Binary.into_node(
                                        vec![
                                            ASTNodeKind::Unary.into_node(
                                                vec![
                                                    ASTNodeKind::Number(2).no_children(),
                                                ]
                                            ),
                                            ASTNodeKind::Unary.into_node(
                                                vec![
                                                    ASTNodeKind::Binary.into_node(
                                                        vec![
                                                            ASTNodeKind::Unary.into_node(
                                                                vec![
                                                                    ASTNodeKind::Number(3).no_children(),
                                                                ]
                                                            ),
                                                        ]
                                                    ),
                                                ]
                                            ),
                                        ]
                                    ),
                                    ASTNodeKind::Unary.into_node(
                                        vec![
                                            ASTNodeKind::Unary.into_node(
                                                vec![
                                                    ASTNodeKind::Unary.into_node(
                                                        vec![
                                                            ASTNodeKind::Unary.into_node(
                                                                vec![
                                                                    ASTNodeKind::Unary.into_node(
                                                                        vec![
                                                                            ASTNodeKind::Unary.into_node(
                                                                                vec![
                                                                                    ASTNodeKind::Number(3).no_children()
                                                                                ]
                                                                            ),
                                                                        ]
                                                                    ),
                                                                ]
                                                            ),
                                                        ]
                                                    ),
                                                ]
                                            ),
                                        ]
                                    ),
                                ]
                            ),
                        ]
                    )
                ]
            )
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
            ASTNodeKind::Let.into_node(
                vec![
                    ASTNodeKind::Number(1).no_children(),
                ]
            ),
            ASTNodeKind::If.into_node(
                vec![
                    ASTNodeKind::Binary.into_node(
                        vec![
                            ASTNodeKind::Variable("a".to_string()).no_children(),
                            ASTNodeKind::Number(0).no_children(),
                        ]
                    ),
                    ASTNodeKind::Block.into_node(
                        vec![
                            ASTNodeKind::Assignment.into_node(
                                vec![
                                    ASTNodeKind::Number(20).no_children(),
                                ]
                            ),
                        ]
                    ),
                ]
            ),
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
            ASTNodeKind::Let.into_node(
                vec![
                    ASTNodeKind::Number(1).no_children(),
                ]
            ),
            ASTNodeKind::If.into_node(
                vec![
                    ASTNodeKind::Binary.into_node(
                        vec![
                            ASTNodeKind::Variable("a".to_string()).no_children(),
                            ASTNodeKind::Number(0).no_children(),
                        ]
                    ),
                    ASTNodeKind::Block.into_node(
                        vec![
                            ASTNodeKind::Assignment.into_node(
                                vec![
                                    ASTNodeKind::Number(20).no_children(),
                                ]
                            ),
                        ]
                    ),
                    ASTNodeKind::Block.into_node(
                        vec![
                            ASTNodeKind::Assignment.into_node(
                                vec![
                                    ASTNodeKind::Number(30).no_children(),
                                ]
                            ),
                        ]
                    ),
                ]
            ),
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
            ASTNodeKind::Let.into_node(
                vec![
                    ASTNodeKind::Number(1).no_children(),
                ]
            ),
            ASTNodeKind::While.into_node(
                vec![
                    ASTNodeKind::Binary.into_node(
                        vec![
                            ASTNodeKind::Variable("a".to_string()).no_children(),
                            ASTNodeKind::Number(10).no_children(),
                        ]
                    ),
                    ASTNodeKind::Block.into_node(
                        vec![
                            ASTNodeKind::Assignment.into_node(
                                vec![
                                    ASTNodeKind::Binary.into_node(
                                        vec![
                                            ASTNodeKind::Variable("a".to_string()).no_children(),
                                            ASTNodeKind::Number(1).no_children(),
                                        ]
                                    ),
                                ]
                            ),
                        ]
                    ),
                ]
            ),
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
            ASTNodeKind::Func.into_node(
                vec![
                    ASTNodeKind::Block.into_node(
                        vec![
                            ASTNodeKind::Return.into_node(
                                vec![
                                    ASTNodeKind::Binary.into_node(
                                        vec![
                                            ASTNodeKind::Variable("a".to_string()).no_children(),
                                            ASTNodeKind::Variable("b".to_string()).no_children(),
                                        ]
                                    ),
                                ]
                            ),
                        ]
                    ),
                ]
            ),
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
            ASTNodeKind::Func.into_node(
                vec![
                    ASTNodeKind::Block.into_node(
                        vec![
                            ASTNodeKind::Return.into_node(
                                vec![
                                    ASTNodeKind::Binary.into_node(
                                        vec![
                                            ASTNodeKind::Variable("a".to_string()).no_children(),
                                            ASTNodeKind::Variable("b".to_string()).no_children(),
                                        ]
                                    ),
                                ]
                            ),
                        ]
                    ),
                ]
            ),
            ASTNodeKind::Call.into_node(
                vec![
                    ASTNodeKind::Binary.into_node(
                        vec![
                            ASTNodeKind::Number(2).no_children(),
                            ASTNodeKind::Number(3).no_children(),
                        ]
                    ),
                    ASTNodeKind::Binary.into_node(
                        vec![
                            ASTNodeKind::Number(4).no_children(),
                            ASTNodeKind::Number(5).no_children(),
                        ]
                    ),
                ]
            ),
        ];

        assert_tree(input, expected);
    }
}
