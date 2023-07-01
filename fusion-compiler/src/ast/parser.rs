use crate::ast::{Ast, StructField, PtrSyntax, QualifiedIdentifier, StaticTypeAnnotation, TypeSyntax, Item, StructDeclaration, ItemKind, FunctionDeclaration, Module, ModuleDeclaration, ReturnTypeSyntax};
use crate::ast::expr::{BinOperator, BinOperatorKind, Expr, ASTStructInitField, UnOperator, UnOperatorKind, BlockExpr, ElseBranch, ExprKind};
use crate::ast::expr::ExprKind::Block;
use crate::ast::lexer::stream::TokenStream;
use crate::ast::lexer::token::{Token, TokenKind};
use crate::ast::stmt::{ Stmt, ParameterSyntax};
use crate::diagnostics::DiagnosticsBagCell;
use crate::modules::scopes::GlobalScopeCell;
use crate::modules::symbols::Function;
use crate::typings::Type::Struct;

pub struct ParseResult {
    pub(crate) ast: Ast,
    pub(crate) module_declarations: Vec<Token>,
}

pub struct Parser<'a> {
    tokens: TokenStream<'a>,
    diagnostics_bag: DiagnosticsBagCell,
    is_parsing_condition: bool,
    encountered_module_declarations: Vec<Token>,
    global_scope: GlobalScopeCell,
    ast: Ast,
}

impl<'a> Parser<'a> {
    pub fn new(
        tokens: TokenStream<'a>,
        diagnostics_bag: DiagnosticsBagCell,
        global_scope: GlobalScopeCell,
    ) -> Self {
        Self {
            tokens,
            global_scope,
            diagnostics_bag,
            is_parsing_condition: false,
            encountered_module_declarations: Vec::new(),
            ast: Ast::new(),
        }
    }

    pub fn parse(mut self) -> ParseResult {
        while let Some(item) = self.next_item() {
            self.ast.items.push(item);
        }
        ParseResult {
            ast: self.ast,
            module_declarations: self.encountered_module_declarations,
        }
    }

    fn next_item(&mut self) -> Option<Item> {
        if self.tokens.is_at_end() {
            return None;
        }
        Some(self.parse_item())
    }

    fn parse_item(&mut self ) -> Item {
        let kind = &self.tokens.current().kind;
        let kind = match kind {
            TokenKind::Struct => {
                ItemKind::StructDeclaration(self.parse_struct_declaration())
            }
            TokenKind::Func => {
                ItemKind::FunctionDeclaration(self.parse_function_declaration())
            }
            TokenKind::Mod => {
                ItemKind::ModuleDeclaration(self.parse_module_declaration())
            }
            _ => {
                let stmt = self.parse_statement();
                self.diagnostics_bag.borrow_mut().report_not_allowed_item(&stmt.span());
                ItemKind::NotAllowed(stmt)
            }
        };
        Item::new(kind)
    }

    fn parse_statement(&mut self) -> Stmt {
        let kind = &self.tokens.current().kind;
        let stmt = match kind {

            TokenKind::Let => {
                self.parse_let_statement()
            }

            TokenKind::Return => {
                self.parse_return_statement()
            }

            _ => {
                self.parse_expression_statement()
            }
        };
        self.tokens.consume_if(TokenKind::SemiColon);
        stmt
    }

    fn parse_module_declaration(&mut self) -> ModuleDeclaration {
        let mod_token = self.consume_and_check(TokenKind::Mod);
        let identifier = self.consume_and_check(TokenKind::Identifier);
        self.encountered_module_declarations.push(identifier.clone());

        ModuleDeclaration {
            mod_token,
            identifier,
        }
    }

    fn parse_struct_declaration(&mut self) -> StructDeclaration {
        let struct_token = self.consume_and_check(TokenKind::Struct);
        let identifier = self.consume_and_check(TokenKind::Identifier);
        let id = self.global_scope.borrow_mut().declare_struct(identifier.clone());
        if id.is_err() {
            self.diagnostics_bag.borrow_mut().report_struct_already_declared(
                &identifier,
            );
        }
        let mut fields = Vec::new();
        let open_brace = self.consume_and_check(TokenKind::OpenBrace);
        while self.tokens.current().kind != TokenKind::CloseBrace && !self.tokens.is_at_end() {
            let field_identifier = self.consume_and_check(TokenKind::Identifier);
            let field_type = self.parse_type_annotation();
            fields.push(StructField {
                ty: field_type,
                identifier: field_identifier,
            });
            self.tokens.consume_if(TokenKind::Comma);
        }
        let close_brace = self.consume_and_check(TokenKind::CloseBrace);
        StructDeclaration {
            struct_token,
            identifier,
            open_brace,
            fields,
            close_brace,
        }
    }


    fn parse_function_declaration(&mut self) -> FunctionDeclaration {
        let funct_token = self.consume_and_check(TokenKind::Func);
        let modifier_tokens = self.parse_optional_function_modifiers();
        let identifier = self.consume_and_check(TokenKind::Identifier);
        let parameters = self.parse_optional_parameter_list();
        let return_type = self.parse_optional_return_type();
        let body = if self.tokens.current().kind == TokenKind::OpenBrace {
            Some(self.parse_block_expr())
        } else {
            None
        };
        FunctionDeclaration {
            func_token: funct_token,
            modifier_tokens,
            identifier,
            parameters,
            return_type,
            body,
        }
    }

    fn parse_optional_function_modifiers(&mut self) -> Vec<Token> {
        let mut modifiers = Vec::new();
        while self.tokens.current().kind != TokenKind::Identifier && !self.tokens.is_at_end() {
            modifiers.push(self.tokens.consume_or_eof());
        }
        modifiers
    }

    fn parse_optional_return_type(&mut self) -> Option<ReturnTypeSyntax> {
        if self.tokens.current().kind == TokenKind::Arrow {
            let arrow = self.consume_and_check(TokenKind::Arrow);
            let ty = self.parse_type();
            return Some(ReturnTypeSyntax::new(
                arrow,
                ty,
            ));
        }
        None
    }

    fn parse_optional_parameter_list(&mut self) -> Vec<ParameterSyntax> {
        if self.tokens.current().kind != TokenKind::LeftParen {
            return Vec::new();
        }
        self.consume_and_check(TokenKind::LeftParen);
        let parameters = self.parse_comma_separated_list(
            TokenKind::RightParen,
            |parser| {
                ParameterSyntax{
                    mut_token: parser.tokens.consume_if(TokenKind::Mut),
                    identifier: parser.consume_and_check(TokenKind::Identifier),
                    type_annotation: parser.parse_type_annotation(),
                }
            },
        );
        self.consume_and_check(TokenKind::RightParen);
        parameters
    }

    fn parse_comma_separated_list<Item>(&mut self, terminator: TokenKind, parse: impl Fn(&mut Self) -> Item) -> Vec<Item> {
        let mut list = Vec::new();
        while self.tokens.current().kind != terminator && !self.tokens.is_at_end() {
            list.push(parse(self));
            if self.tokens.current().kind == TokenKind::Comma {
                self.consume_and_check(TokenKind::Comma);
            } else {
                break;
            }
        }
        list
    }

    fn parse_return_statement(&mut self) -> Stmt {
        let return_keyword = self.consume_and_check(TokenKind::Return);
        if self.tokens.current().kind == TokenKind::SemiColon || self.tokens.is_at_end() {
            return self.ast.return_statement(return_keyword, None, true);
        }
        let expression = self.parse_expression();
        self.ast.return_statement(return_keyword, Some(expression), true)
    }

    fn parse_while_expr(&mut self) -> Expr {
        let while_keyword = self.consume_and_check(TokenKind::While);
        self.is_parsing_condition = true;
        let condition_expr = self.parse_expression();
        self.is_parsing_condition = false;
        let body = self.parse_block_expr();
        self.ast.while_expression(while_keyword, condition_expr, body)
    }

    fn parse_block_expr(&mut self) -> BlockExpr {
        let open_brace = self.consume_and_check(TokenKind::OpenBrace);
        let mut statements = Vec::new();
        while self.tokens.current().kind != TokenKind::CloseBrace && !self.tokens.is_at_end() {
            statements.push(self.parse_statement());
        }
        let close_brace = self.consume_and_check(TokenKind::CloseBrace);
        BlockExpr {
            stmts: statements,
            open_brace,
            close_brace,
        }
    }

    fn parse_if_expr(&mut self) -> Expr {
        let if_keyword = self.consume_and_check(TokenKind::If).clone();
        self.is_parsing_condition = true;
        let condition_expr = self.parse_expression();
        self.is_parsing_condition = false;
        let then = self.parse_block_expr();
        let else_statement = self.parse_optional_else_branch();
        self.ast.if_expr(if_keyword, condition_expr, then, else_statement)
    }

    fn parse_optional_else_branch(&mut self) -> Option<ElseBranch> {
        if self.tokens.current().kind == TokenKind::Else {
            let else_keyword = self.consume_and_check(TokenKind::Else).clone();
            let expr = self.parse_block_expr();
            return Some(ElseBranch {
                expr: Box::new(expr),
                else_keyword,
            });
        }
        None
    }


    fn parse_let_statement(&mut self) -> Stmt {
        self.consume_and_check(TokenKind::Let);
        let mut_token = self.tokens.consume_if(TokenKind::Mut);
        let identifier = self.consume_and_check(TokenKind::Identifier).clone();
        let optional_type_annotation = self.parse_optional_type_annotation();
        self.consume_and_check(TokenKind::Equals);
        let expr = self.parse_expression();

        self.ast.let_statement(mut_token, identifier, expr, optional_type_annotation)
    }

    fn parse_optional_type_annotation(&mut self) -> Option<StaticTypeAnnotation> {
        if self.tokens.current().kind == TokenKind::Colon {
            return Some(self.parse_type_annotation());
        }
        return None;
    }

    fn parse_type_annotation(&mut self) -> StaticTypeAnnotation {
        let colon = self.consume_and_check(TokenKind::Colon);
        let ty = self.parse_type();
        StaticTypeAnnotation::new(colon, ty)
    }

    fn parse_type(&mut self) -> TypeSyntax {
        let ptr = if self.tokens.current().kind == TokenKind::Asterisk {
            let mut ptrs = vec![];
            while self.tokens.current().kind == TokenKind::Asterisk {
                ptrs.push((self.consume_and_check(TokenKind::Asterisk), self.tokens.consume_if(TokenKind::Mut)));
            }
            Some(ptrs)
        } else {
            None
        };
        let starting_id = self.consume_and_check(TokenKind::Identifier);
        let type_name = self.parse_qualified_identifier(starting_id);
        let ptrs = ptr.map(|ptrs| ptrs.into_iter().map(|(asterisk, mut_token)| PtrSyntax {
            star: asterisk,
            mut_token,
        }).collect());
        TypeSyntax::new(type_name, ptrs)
    }

    fn parse_expression_statement(&mut self) -> Stmt {
        let expr = self.parse_expression();
        self.ast.expression_statement(expr)
    }

    fn parse_expression(&mut self) -> Expr {
        self.parse_assignment_expression()
    }

    fn parse_assignment_expression(&mut self) -> Expr {
        let assignee = self.parse_binary_expression(0);
        if self.tokens.current().kind == TokenKind::Equals {
            let equals = self.consume_and_check(TokenKind::Equals);
            let expr = self.parse_expression();
            return self.ast.assignment_expression(assignee, equals, expr);
        }
        assignee
    }

    fn parse_binary_expression(&mut self, precedence: u8) -> Expr {
        let mut left = self.parse_cast_expression();

        while let Some(operator) = self.parse_binary_operator() {
            let operator_precedence = operator.precedence();
            if operator_precedence < precedence {
                break;
            }
            self.tokens.consume();
            let right = self.parse_binary_expression(operator_precedence);
            left = self.ast.binary_expression(operator, left, right);
        }
        left
    }

    fn parse_cast_expression(&mut self) -> Expr {
        let expr = self.parse_deref_expression();
        if self.tokens.current().kind == TokenKind::As {
            let as_keyword = self.consume_and_check(TokenKind::As);
            let ty = self.parse_type();
            return self.ast.cast_expression(expr, as_keyword, ty);
        }
        expr
    }

    fn parse_deref_expression(&mut self) -> Expr {
        if self.tokens.current().kind == TokenKind::Asterisk {
            let star = self.consume_and_check(TokenKind::Asterisk);
            let expr = self.parse_deref_expression();
            return self.ast.deref_expression(star, expr);
        }
        self.parse_ref_expression()
    }

    fn parse_ref_expression(&mut self) -> Expr {
        if self.tokens.current().kind == TokenKind::Ampersand {
            let ampersand = self.consume_and_check(TokenKind::Ampersand).clone();
            let mut_token = self.tokens.consume_if(TokenKind::Mut);
            let expr = self.parse_deref_expression();
            return self.ast.ref_expression(ampersand, mut_token, expr);
        }
        self.parse_unary_expression()
    }

    fn parse_unary_expression(&mut self) -> Expr {
        if let Some(operator) = self.parse_unary_operator() {
            self.tokens.consume();
            let operand = self.parse_unary_expression();
            return self.ast.unary_expression(operator, operand);
        }
        return self.parse_primary_expression();
    }

    fn parse_unary_operator(&mut self) -> Option<UnOperator> {
        let token = self.tokens.current();
        let kind = match token.kind {
            TokenKind::Minus => {
                Some(UnOperatorKind::Minus)
            }
            TokenKind::Tilde => {
                Some(UnOperatorKind::BitwiseNot)
            }
            _ => {
                None
            }
        };
        return kind.map(|kind| UnOperator::new(kind, token.clone()));
    }

    fn parse_binary_operator(&mut self) -> Option<BinOperator> {
        let token = self.tokens.current();
        let kind = match token.kind {
            TokenKind::Plus => {
                Some(BinOperatorKind::Plus)
            }
            TokenKind::Minus => {
                Some(BinOperatorKind::Minus)
            }
            TokenKind::Percent => {
                Some(BinOperatorKind::Modulo)
            }
            TokenKind::Asterisk => {
                Some(BinOperatorKind::Multiply)
            }
            TokenKind::Slash => {
                Some(BinOperatorKind::Divide)
            }
            TokenKind::Ampersand => {
                Some(BinOperatorKind::BitwiseAnd)
            }
            TokenKind::Pipe => {
                Some(BinOperatorKind::BitwiseOr)
            }
            TokenKind::Caret => {
                Some(BinOperatorKind::BitwiseXor)
            }
            TokenKind::DoubleAsterisk => {
                Some(BinOperatorKind::Power)
            }
            TokenKind::EqualsEquals => {
                Some(BinOperatorKind::Equals)
            }
            TokenKind::BangEquals => {
                Some(BinOperatorKind::NotEquals)
            }
            TokenKind::LessThan => {
                Some(BinOperatorKind::LessThan)
            }
            TokenKind::LessThanEquals => {
                Some(BinOperatorKind::LessThanOrEqual)
            }
            TokenKind::GreaterThan => {
                Some(BinOperatorKind::GreaterThan)
            }
            TokenKind::GreaterThanEquals => {
                Some(BinOperatorKind::GreaterThanOrEqual)
            }

            TokenKind::DoubleAmpersand => {
                Some(BinOperatorKind::LogicalAnd)
            }
            _ => {
                None
            }
        };
        kind.map(|kind| BinOperator::new(kind, token.clone()))
    }

    fn parse_primary_expression(&mut self) -> Expr {
        let token = self.tokens.consume_or_eof();
        let expr = match token.kind {
            TokenKind::Number(number) => {
                let ty = if self.tokens.current().kind == TokenKind::Identifier {
                    Some(self.consume_and_check(TokenKind::Identifier))
                } else {
                    None
                };
                self.ast.number_expression(token, number, ty)
            }
            TokenKind::LeftParen => {
                let expr = self.parse_expression();
                let left_paren = token;
                let right_paren = self.consume_and_check(TokenKind::RightParen);
                self.ast.parenthesized_expression(left_paren, expr, right_paren)
            }
            TokenKind::Identifier => {
                let qualified = self.parse_qualified_identifier(token);
                self.tokens.consume();
                if self.tokens.current().kind == TokenKind::OpenBrace && !self.is_parsing_condition {
                    self.parse_struct_init_expression(qualified)
                } else {
                    self.ast.identifier_expression(qualified)
                }
            }
            TokenKind::True | TokenKind::False => {
                let value = token.kind == TokenKind::True;
                self.ast.boolean_expression(token, value)
            }
            TokenKind::String(open_quote, token, close_quote) => {
                self.ast.string_expression(*open_quote, token, *close_quote)
            }
            TokenKind::Character(open_quote, char, close_quote) => {
                self.ast.character_expression(*open_quote, char, *close_quote)
            }
            TokenKind::If => {
                self.parse_if_expr()
            }
            TokenKind::OpenBrace => {
                Expr::new(ExprKind::Block(self.parse_block_expr()))
            }
            TokenKind::While => {
                self.parse_while_expr()
            }
            _ => {
                self.diagnostics_bag.borrow_mut().report_expected_expression(&token);
                self.ast.error_expression(token.span.clone())
            }
        };
        let mut expr = expr;
        while !self.tokens.is_at_end() && self.tokens.current().kind != TokenKind::SemiColon {
            match self.tokens.current().kind {
                TokenKind::LeftParen => {
                    expr = self.parse_call_expression(expr);
                }
                TokenKind::Dot => {
                    expr = self.parse_member_access_expression(expr);
                }
                TokenKind::Arrow => {
                    expr = self.parse_member_access_expression(expr);
                }
                TokenKind::OpenBracket => {
                    expr = self.parse_index_expression(expr);
                }
                _ => {
                    return expr;
                }
            }
        }
        return expr;
    }

    fn parse_index_expression(&mut self, expr: Expr) -> Expr {
        let open_bracket = self.consume_and_check(TokenKind::OpenBracket).clone();
        let index = self.parse_expression();
        let close_bracket = self.consume_and_check(TokenKind::CloseBracket).clone();
        self.ast.index_expression(expr, open_bracket, index, close_bracket)
    }

    fn parse_qualified_identifier(&mut self, token: Token) -> QualifiedIdentifier {
        let identifiers = self.parse_identifier_chain(token);
        let qualified = QualifiedIdentifier::new(identifiers);
        qualified
    }

    fn parse_identifier_chain(&mut self, identifier: Token) -> Vec<Token> {
        let mut identifiers = vec![identifier];
        while self.tokens.current().kind == TokenKind::ColonColon {
            self.tokens.consume();
            let identifier = self.consume_and_check(TokenKind::Identifier).clone();
            identifiers.push(identifier);
        }
        return identifiers;
    }

    fn parse_struct_init_expression(&mut self, identifier: QualifiedIdentifier) -> Expr {
        let open_brace = self.consume_and_check(TokenKind::OpenBrace).clone();
        let fields = self.parse_comma_separated_list(TokenKind::CloseBrace, |parser| {
            let identifier = parser.consume_and_check(TokenKind::Identifier).clone();
            let colon = parser.consume_and_check(TokenKind::Colon).clone();
            let expr = parser.parse_expression();
            ASTStructInitField {
                identifier,
                colon,
                initializer: Box::new(expr),
            }
        });
        let close_brace = self.consume_and_check(TokenKind::CloseBrace).clone();
        self.ast.struct_init_expression(identifier, open_brace, fields, close_brace)
    }

    fn parse_member_access_expression(&mut self, expr: Expr) -> Expr {
        let dot = self.consume_and_check_multiple(&[TokenKind::Dot, TokenKind::Arrow]).clone();
        let identifier = self.consume_and_check(TokenKind::Identifier).clone();
        self.ast.member_access_expression(expr, dot, identifier)
    }

    fn parse_call_expression(&mut self, callee: Expr) -> Expr {
        let left_paren = self.consume_and_check(TokenKind::LeftParen);
        let arguments = self.parse_comma_separated_list(TokenKind::RightParen, |parser| parser.parse_expression()).to_vec();
        let right_paren = self.consume_and_check(TokenKind::RightParen);
        Ast::call_expression(&mut self.ast, callee, left_paren, arguments, right_paren)
    }

    fn consume(&mut self) -> Token {
        let token = self.tokens.consume_or_eof();
        token
    }

    fn consume_and_check_multiple(&mut self, kinds: &[TokenKind]) -> Token {
        self.consume_and_check_multiple_nnl(kinds)
    }

    fn consume_and_check_multiple_nnl(&mut self, kinds: &[TokenKind]) -> Token {
        let token = self.tokens.consume_or_eof();
        if !kinds.contains(&token.kind) {
            self.diagnostics_bag.borrow_mut().report_unexpected_token_multiple(
                kinds,
                &token,
            );
        }
        token
    }

    fn consume_and_check(&mut self, kind: TokenKind) -> Token {
        self.consume_and_check_nnl(kind)
    }

    fn consume_and_check_nnl(&mut self, kind: TokenKind) -> Token {
        let token = self.tokens.consume_or_eof();
        if token.kind != kind {
            self.diagnostics_bag.borrow_mut().report_unexpected_token(
                &kind,
                &token,
            );
        }
        token
    }
}