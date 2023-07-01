use crate::ast::{StaticTypeAnnotation, TypeSyntax};
use crate::ast::expr::Expr;
use crate::ast::lexer::token::Token;
use crate::text::span::TextSpan;

#[derive(Debug, Clone)]
pub enum StmtKind {
    Expr(Expr),
    Let(LetStmt),
    Return(ReturnStmt),
}

#[derive(Debug, Clone)]
pub struct ReturnStmt {
    pub return_keyword: Token,
    pub expr: Option<Expr>,
    pub is_top_level: bool,
}


#[derive(Debug, Clone)]
pub struct ParameterSyntax {
    pub mut_token: Option<Token>,
    pub identifier: Token,
    pub type_annotation: StaticTypeAnnotation,
}

#[derive(Debug, Clone)]
pub struct LetStmt {
    pub mut_token: Option<Token>,
    pub identifier: Token,
    pub initializer: Expr,
    pub type_annotation: Option<StaticTypeAnnotation>,
}

#[derive(Debug, Clone)]
pub struct Stmt {
    pub kind: StmtKind,
}

impl Stmt {
    pub fn new(kind: StmtKind) -> Self {
        Stmt { kind }
    }

    pub fn span(&self) -> TextSpan {
        match &self.kind
        {
            StmtKind::Expr(expr) => expr.span(),
            StmtKind::Let(stmt) => {
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
            StmtKind::Return(stmt) => {
                let mut spans = vec![
                    &stmt.return_keyword.span,
                ];
                let return_value_span = stmt.expr.as_ref().map(|return_value| return_value.span());
                if let Some(return_value_span) = return_value_span.as_ref() {
                    spans.push(return_value_span);
                }
                TextSpan::merge(
                    spans
                )
            }
        }
    }
}
