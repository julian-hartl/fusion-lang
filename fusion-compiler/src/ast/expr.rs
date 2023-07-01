use crate::ast::{QualifiedIdentifier, TypeSyntax};
use crate::ast::lexer::{EscapedCharacter, StringToken};
use crate::ast::lexer::token::Token;
use crate::ast::stmt::Stmt;
use crate::text::span::TextSpan;

#[derive(Debug, Clone)]
pub enum ExprKind {
    Number(
        NumberExpr
    ),
    String(
        StringExpr
    ),
    Char(
        CharExpr
    ),
    Binary(
        BinExpr
    ),
    Unary(
        UnaryExpr
    ),
    Parenthesized(
        ParenExpr
    ),

    Identifier(
        IdenExpr
    ),
    Assignment(
        AssignExpr
    ),
    Boolean(
        BoolExpr
    ),
    Call(
        CallExpr
    ),
    Ref(
        RefExpr
    ),
    Deref(
        DerefExpr
    ),
    Cast(
        CastExpr
    ),
    MemberAccess(
        MemberAccessExpr
    ),
    StructInit(
        StructInitExpr
    ),
    Index(
        IndexExpr
    ),
    Error(
        TextSpan
    ),
    Block(
        BlockExpr
    ),
    While(
        WhileExpr
    ),
    If(
        IfExpr
    ),
}

#[derive(Debug, Clone)]
pub struct ElseBranch {
    pub else_keyword: Token,
    pub expr: Box<BlockExpr>,
}

#[derive(Debug, Clone)]
pub struct IfExpr {
    pub if_keyword: Token,
    pub condition: Box<Expr>,
    pub then_branch: Box<BlockExpr>,
    pub else_branch: Option<ElseBranch>,
}

#[derive(Debug, Clone)]
pub struct WhileExpr {
    pub while_keyword: Token,
    pub condition: Box<Expr>,
    pub body: Box<BlockExpr>,
}


#[derive(Debug, Clone)]
pub struct BlockExpr {
    pub open_brace: Token,
    pub stmts: Vec<Stmt>,
    pub close_brace: Token,
}

impl BlockExpr {
    pub fn span(&self) -> TextSpan {
        TextSpan::merge(
            vec![
                &self.open_brace.span,
                &TextSpan::merge(
                    self.stmts.iter().map(|stmt| &stmt.span())
                        .collect::<Vec<&TextSpan>>()),
                &self.close_brace.span,
            ]
        )
    }
}


#[derive(Debug, Clone)]
pub struct IndexExpr {
    pub target: Box<Expr>,
    pub open_bracket: Token,
    pub index: Box<Expr>,
    pub close_bracket: Token,
}

#[derive(Debug, Clone)]
pub struct StructInitExpr {
    pub identifier: QualifiedIdentifier,
    pub open_brace: Token,
    pub close_brace: Token,
    pub fields: Vec<ASTStructInitField>,
}

#[derive(Debug, Clone)]
pub struct MemberAccessExpr {
    pub expr: Box<Expr>,
    pub access_operator: Token,
    pub member: Token,
}

#[derive(Debug, Clone)]
pub struct CastExpr {
    pub expr: Box<Expr>,
    pub as_keyword: Token,
    pub ty: TypeSyntax,
}

#[derive(Debug, Clone)]
pub struct CharExpr {
    pub open_quote: Token,
    pub value: char,
    pub close_quote: Token,
}

#[derive(Debug, Clone)]
pub struct RefExpr {
    pub ampersand: Token,
    pub mut_token: Option<Token>,
    pub expr: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct DerefExpr {
    pub star: Token,
    pub expr: Box<Expr>,
}


#[derive(Debug, Clone)]
pub struct StringExpr {
    pub open_quote: Token,
    pub string: StringToken,
    pub close_quote: Token,
}

#[derive(Debug, Clone)]
pub struct CallExpr {
    pub callee: Box<Expr>,
    pub left_paren: Token,
    pub arguments: Vec<Expr>,
    pub right_paren: Token,
}

#[derive(Debug, Clone)]
pub struct BoolExpr {
    pub value: bool,
    pub token: Token,
}

#[derive(Debug, Clone)]
pub struct AssignExpr {
    pub left: Box<Expr>,
    pub equals: Token,
    pub right: Box<Expr>,
}

#[derive(Debug, Clone)]
pub enum UnOperatorKind {
    Minus,
    BitwiseNot,
}

#[derive(Debug, Clone)]
pub struct UnaryExpr {
    pub operator: UnOperator,
    pub operand: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct IdenExpr {
    pub identifier: QualifiedIdentifier,
}


#[derive(Debug, Clone)]
pub enum BinOperatorKind {
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
    // Logial
    LogicalAnd,
}

#[derive(Debug, Clone)]
pub struct BinExpr {
    pub left: Box<Expr>,
    pub operator: BinOperator,
    pub right: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct NumberExpr {
    pub number: i64,
    pub token: Token,
    pub size_specifier: Option<Token>,
}

#[derive(Debug, Clone)]
pub struct ParenExpr {
    pub left_paren: Token,
    pub expr: Box<Expr>,
    pub right_paren: Token,
}

#[derive(Debug, Clone)]
pub struct ASTStructInitField {
    pub identifier: Token,
    pub colon: Token,
    pub initializer: Box<Expr>,
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
pub struct UnOperator {
    pub(crate) kind: UnOperatorKind,
    pub token: Token,
}

impl UnOperator {
    pub fn new(kind: UnOperatorKind, token: Token) -> Self {
        UnOperator { kind, token }
    }
}

#[derive(Debug, Clone)]
pub struct BinOperator {
    pub kind: BinOperatorKind,
    pub token: Token,
}

impl BinOperator {
    pub fn new(kind: BinOperatorKind, token: Token) -> Self {
        BinOperator { kind, token }
    }

    pub fn precedence(&self) -> u8 {
        match self.kind {
            BinOperatorKind::Power => 20,
            BinOperatorKind::Multiply => 19,
            BinOperatorKind::Divide => 19,
            BinOperatorKind::Modulo => 19,
            BinOperatorKind::Plus => 18,
            BinOperatorKind::Minus => 18,
            BinOperatorKind::BitwiseAnd => 17,
            BinOperatorKind::BitwiseXor => 16,
            BinOperatorKind::BitwiseOr => 15,
            BinOperatorKind::NotEquals => 30,
            BinOperatorKind::LessThan => 29,
            BinOperatorKind::LessThanOrEqual => 29,
            BinOperatorKind::GreaterThan => 29,
            BinOperatorKind::GreaterThanOrEqual => 29,
            BinOperatorKind::Equals => 11,
            BinOperatorKind::LogicalAnd => 10,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
}

impl Expr {
    pub fn new(kind: ExprKind) -> Self {
        Expr { kind }
    }

    pub fn span(&self) -> TextSpan {
        match &self.kind {
            ExprKind::Number(expr) => expr.token.span.clone(),
            ExprKind::Binary(expr) => {
                let left = &expr.left.span();
                let operator = &expr.operator.token.span;
                let right = &expr.right.span();
                TextSpan::merge(vec![left, operator, right])
            }
            ExprKind::Unary(expr) => {
                let operator = &expr.operator.token.span;
                let operand = &expr.operand.span();
                TextSpan::merge(vec![operator, operand])
            }
            ExprKind::Parenthesized(expr) => {
                let open_paren = &expr.left_paren.span;
                let expression = &expr.expr.span();
                let close_paren = &expr.right_paren.span;
                TextSpan::merge(vec![open_paren, expression, close_paren])
            }
            ExprKind::Identifier(expr) => expr.identifier.span(),
            ExprKind::Assignment(expr) => {
                let identifier = &expr.left.span();
                let equals = &expr.equals.span;
                let expression = &expr.right.span();
                TextSpan::merge(vec![identifier, equals, expression])
            }
            ExprKind::Boolean(expr) => expr.token.span.clone(),
            ExprKind::Call(expr) => {
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
            ExprKind::Error(span) => span.clone(),
            ExprKind::String(expr) => {
                let spans = vec![&expr.open_quote.span, &expr.close_quote.span];
                TextSpan::merge(spans)
            }
            ExprKind::Ref(expr) => {
                let span1 = expr.expr.span();
                let spans = vec![&expr.ampersand.span, &span1];
                TextSpan::merge(spans)
            }
            ExprKind::Deref(expr) => {
                let span2 = expr.expr.span();
                let spans = vec![&expr.star.span, &span2];
                TextSpan::merge(spans)
            }
            ExprKind::Char(expr) => {
                let spans = vec![&expr.open_quote.span, &expr.close_quote.span];
                TextSpan::merge(spans)
            }
            ExprKind::Cast(expr) => {
                let span1 = &expr.as_keyword.span;
                let span2 = &expr.ty.span();
                let span3 = expr.expr.span();
                TextSpan::merge(vec![&span1, &span2, &span3])
            }
            ExprKind::MemberAccess(expr) => {
                let span1 = expr.expr.span();
                let span2 = &expr.access_operator.span;
                let span3 = &expr.member.span;
                TextSpan::merge(vec![&span1, &span2, &span3])
            }
            ExprKind::StructInit(expr) => {
                let span2 = &expr.open_brace.span;
                let span3 = &expr.close_brace.span;
                let mut spans = vec![span2, span3];
                let field_spans: Vec<TextSpan> = expr.fields.iter().map(|field| field.span()).collect();
                for span in &field_spans {
                    spans.push(span);
                }
                TextSpan::merge(spans)
            }
            ExprKind::Index(expr) => {
                let span1 = expr.target.span();
                let span2 = &expr.open_bracket.span;
                let span3 = &expr.index.span();
                let span4 = &expr.close_bracket.span;
                TextSpan::merge(vec![&span1, &span2, &span3, &span4])
            }
            ExprKind::Block(stmt) => {
                let mut spans = vec![
                    stmt.open_brace.span.clone(),
                    stmt.close_brace.span.clone(),
                ];
                spans.extend(
                    stmt.stmts.iter().map(|stmt| stmt.span()));
                TextSpan::merge(
                    spans.iter().map(|span| span).collect()
                )
            }
            ExprKind::While(stmt) => {
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
            ExprKind::If(stmt) => {
                let cond_span = stmt.condition.span();
                let then_branch_span = stmt.then_branch.span();
                let mut spans = vec![
                    &stmt.if_keyword.span,
                    &cond_span,
                    &then_branch_span,
                ];

                if let Some(else_branch) = &stmt.else_branch {
                    spans.push(&else_branch.else_keyword.span);
                    let span1 = else_branch.expr.span();
                    // todo: fix this
                    // spans.push(&span1);
                }
                TextSpan::merge(
                    spans
                )
            }
        }
    }
}
