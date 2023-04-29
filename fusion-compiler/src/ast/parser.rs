use std::cell::Cell;

use crate::ast::{Ast, ASTBinaryOperator, ASTBinaryOperatorKind, ASTClassBody, ASTClassConstructor, ASTClassField, ASTClassMember, ASTClassMethod, ASTElseStatement, ASTExpression, ASTExprId, ASTFunctionReturnType, ASTStatement, ASTString, ASTUnaryExpression, ASTUnaryOperator, ASTUnaryOperatorKind, EscapedCharacter, FuncDeclParameter, NormalFuncDeclParameter, StaticTypeAnnotation, StringPart};
use crate::ast::lexer::{Lexer, Token, TokenKind};
use crate::diagnostics::DiagnosticsBagCell;

#[derive(Debug, Clone)]
pub struct Counter {
    value: Cell<usize>,
}

impl Counter {
    pub fn new() -> Self {
        Self {
            value: Cell::new(0)
        }
    }

    pub fn increment(&self) {
        let current_value = self.value.get();
        self.value.set(current_value + 1);
    }

    pub fn get_value(&self) -> usize {
        self.value.get()
    }
}

pub struct Parser<'a> {
    tokens: Vec<Token>,
    current: Counter,
    diagnostics_bag: DiagnosticsBagCell,
    ast: &'a mut Ast,
}

impl<'a> Parser<'a> {
    pub fn new(
        tokens: Vec<Token>,
        diagnostics_bag: DiagnosticsBagCell,
        ast: &'a mut Ast,
    ) -> Self {
        Self {
            tokens,
            current: Counter::new(),
            diagnostics_bag,
            ast,
        }
    }

    pub fn parse(&mut self) {
        while let Some(stmt) = self.next_statement().map(|stmt| stmt.id) {
            self.ast.mark_top_level_statement(stmt);
        }
    }

    fn next_statement(&mut self) -> Option<ASTStatement> {
        if self.is_at_end() {
            return None;
        }
        Some(self.parse_statement())
    }

    fn is_at_end(&self) -> bool {
        self.current().kind == TokenKind::Eof
    }

    fn parse_statement(&mut self) -> ASTStatement {
        self.consume_whitespace();
        let kind = &self.current().kind;
        let stmt = match kind {
            TokenKind::Let => {
                self.parse_let_statement()
            }
            TokenKind::If => {
                self.parse_if_statement()
            }
            TokenKind::OpenBrace => {
                self.parse_block_statement()
            }
            TokenKind::While => {
                self.parse_while_statement()
            }
            TokenKind::Func => {
                self.parse_function_declaration()
            }
            TokenKind::Return => {
                self.parse_return_statement()
            }
            TokenKind::Class => self.parse_class_statement(),
            _ => {
                self.parse_expression_statement()
            }
        }.clone();
        self.consume_whitespace();
        stmt
    }

    fn parse_class_statement(&mut self) -> &ASTStatement {
        let class_token = self.consume_and_check(TokenKind::Class).clone();
        let identifier = self.consume_and_check(TokenKind::Identifier).clone();
        let constructor = self.parse_optional_class_constructor();
        let body = self.parse_class_body();
        self.ast.class_statement(class_token, identifier, constructor, body)
    }

    fn parse_class_body(&mut self) -> ASTClassBody {
        let open_brace = self.consume_and_check(TokenKind::OpenBrace).clone();
        let mut members = Vec::new();
        while self.current().kind != TokenKind::CloseBrace && !self.is_at_end() {
            members.push(self.parse_class_member());
        }
        let close_brace = self.consume_and_check(TokenKind::CloseBrace).clone();
        ASTClassBody {
            open_brace,
            members,
            close_brace,
        }
    }

    fn parse_class_member(&mut self) -> ASTClassMember {
        match self.current().kind {
            TokenKind::Identifier => ASTClassMember::Field(self.parse_class_field()),
            TokenKind::Func => {
                let func_decl = self.parse_function_declaration();
                ASTClassMember::Method(ASTClassMethod
                {
                    func_decl: func_decl.id,
                })
            }
            _ => {
                let stmt = self.parse_statement();
                self.diagnostics_bag.borrow_mut().report_invalid_class_member(&stmt.span(self.ast));
                ASTClassMember::Invalid(stmt.id)
            }
        }
    }

    fn parse_optional_class_constructor(&mut self) -> Option<ASTClassConstructor> {
        match self.current().kind {
            TokenKind::LeftParen => {
                self.consume();
                let fields = self.parse_comma_separated_list(
                    TokenKind::RightParen,
                    |parser| parser.parse_class_field(),
                );
                self.consume_and_check(TokenKind::RightParen);
                Some(ASTClassConstructor {
                    fields,
                })
            }
            _ => None,
        }
    }

    fn parse_class_field(&mut self) -> ASTClassField {
        ASTClassField {
            identifier: self.consume_and_check(TokenKind::Identifier).clone(),
            type_annotation: self.parse_type_annotation(),
        }
    }

    fn parse_function_declaration(&mut self) -> &ASTStatement {
        let funct_token = self.consume_and_check(TokenKind::Func).clone();
        let modifier_tokens = self.parse_optional_function_modifiers();
        let identifier = self.consume_and_check(TokenKind::Identifier).clone();
        let parameters = self.parse_optional_parameter_list();
        let return_type = self.parse_optional_return_type();
        let body = match self.current().kind {
            TokenKind::OpenBrace => {
                Some(self.parse_block_statement().id)
            }
            _ => None,
        };
        self.ast.func_decl_statement(funct_token, modifier_tokens, identifier, parameters, body, return_type)
    }

    fn parse_optional_function_modifiers(&mut self) -> Vec<Token> {
        let mut modifiers = Vec::new();
        while self.current().kind != TokenKind::Identifier && !self.is_at_end() {
            modifiers.push(self.consume().clone());
        }
        modifiers
    }

    fn parse_optional_return_type(&mut self) -> Option<ASTFunctionReturnType> {
        if self.current().kind == TokenKind::Arrow {
            let arrow = self.consume_and_check(TokenKind::Arrow).clone();
            let type_name = self.parse_type().clone();
            return Some(ASTFunctionReturnType::new(
                arrow,
                type_name,
            ));
        }
        return None;
    }

    fn parse_optional_parameter_list(&mut self) -> Vec<FuncDeclParameter> {
        if self.current().kind != TokenKind::LeftParen {
            return Vec::new();
        }
        self.consume_and_check(TokenKind::LeftParen);
        let parameters = self.parse_comma_separated_list(
            TokenKind::RightParen,
            |parser| {
                match parser.current().kind {
                    TokenKind::Self_ => {
                        FuncDeclParameter::Self_(parser.consume_and_check(TokenKind::Self_).clone())
                    }
                    _ => {
                        FuncDeclParameter::Normal(NormalFuncDeclParameter {
                            identifier: parser.consume_and_check(TokenKind::Identifier).clone(),
                            type_annotation: parser.parse_type_annotation(),
                        })
                    }
                }
            },
        );
        self.consume_and_check(TokenKind::RightParen);
        parameters
    }

    fn parse_comma_separated_list<Item>(&mut self, terminator: TokenKind, parse: impl Fn(&mut Self) -> Item) -> Vec<Item> {
        let mut list = Vec::new();
        while self.current().kind != terminator && !self.is_at_end() {
            list.push(parse(self));
            if self.current().kind == TokenKind::Comma {
                self.consume_and_check(TokenKind::Comma);
            }
        }
        list
    }

    fn parse_return_statement(&mut self) -> &ASTStatement {
        let return_keyword = self.consume_and_check(TokenKind::Return).clone();
        if self.current().kind == TokenKind::Newline || self.is_at_end() {
            return self.ast.return_statement(return_keyword, None, true);
        }
        let expression = self.parse_expression().id;
        self.ast.return_statement(return_keyword, Some(expression), true)
    }

    fn parse_while_statement(&mut self) -> &ASTStatement {
        let while_keyword = self.consume_and_check(TokenKind::While).clone();
        let condition_expr = self.parse_expression().id;
        let body = self.parse_statement().id;
        self.ast.while_statement(while_keyword, condition_expr, body)
    }

    fn parse_block_statement(&mut self) -> &ASTStatement {
        let open_brace = self.consume_and_check(TokenKind::OpenBrace).clone();
        let mut statements = Vec::new();
        while self.current().kind != TokenKind::CloseBrace && !self.is_at_end() {
            statements.push(self.parse_statement().id);
        }
        let close_brace = self.consume_and_check(TokenKind::CloseBrace).clone();
        self.ast.block_statement(open_brace, statements, close_brace)
    }

    fn parse_if_statement(&mut self) -> &ASTStatement {
        let if_keyword = self.consume_and_check(TokenKind::If).clone();
        let condition_expr = self.parse_expression().id;
        let then = self.parse_statement().id;
        let else_statement = self.parse_optional_else_statement();
        self.ast.if_statement(if_keyword, condition_expr, then, else_statement)
    }

    fn parse_optional_else_statement(&mut self) -> Option<ASTElseStatement> {
        if self.current().kind == TokenKind::Else {
            let else_keyword = self.consume_and_check(TokenKind::Else).clone();
            let else_statement = self.parse_statement().id;
            return Some(ASTElseStatement::new(else_keyword, else_statement));
        }
        return None;
    }


    fn parse_let_statement(&mut self) -> &ASTStatement {
        self.consume_and_check(TokenKind::Let);
        let identifier = self.consume_and_check(TokenKind::Identifier).clone();
        let optional_type_annotation = self.parse_optional_type_annotation();
        self.consume_and_check(TokenKind::Equals);
        let expr = self.parse_expression().id;

        self.ast.let_statement(identifier, expr, optional_type_annotation, None)
    }

    fn parse_optional_type_annotation(&mut self) -> Option<StaticTypeAnnotation> {
        if self.current().kind == TokenKind::Colon {
            return Some(self.parse_type_annotation());
        }
        return None;
    }

    fn parse_type_annotation(&mut self) -> StaticTypeAnnotation {
        let colon = self.consume_and_check(TokenKind::Colon).clone();
        let type_name = self.parse_type().clone();
        return StaticTypeAnnotation::new(colon, type_name);
    }

    fn parse_type(&mut self) -> &Token {
        self.consume_and_check_multiple([
            TokenKind::Identifier,
            TokenKind::I64,
            TokenKind::Str,
            TokenKind::Bool,
        ].as_slice())
    }

    fn parse_expression_statement(&mut self) -> &ASTStatement {
        let expr = self.parse_expression().id;
        self.ast.expression_statement(expr)
    }

    fn parse_expression(&mut self) -> &ASTExpression {
        self.parse_assignment_expression()
    }

    fn parse_assignment_expression(&mut self) -> &ASTExpression {
        if self.current().kind == TokenKind::Identifier {
            if self.peek(1).kind == TokenKind::Equals {
                let identifier = self.consume_and_check(TokenKind::Identifier).clone();
                let equals = self.consume_and_check(TokenKind::Equals).clone();
                let expr = self.parse_expression().id;
                return self.ast.assignment_expression(identifier, equals, expr, None);
            }
        }
        return self.parse_binary_expression(0);
    }

    fn parse_binary_expression(&mut self, precedence: u8) -> &ASTExpression {
        let mut left = self.parse_unary_expression().id;

        while let Some(operator) = self.parse_binary_operator() {
            let operator_precedence = operator.precedence();
            if operator_precedence < precedence {
                break;
            }
            self.consume();
            let right = self.parse_binary_expression(operator_precedence).id;
            left = self.ast.binary_expression(operator, left, right).id;
        }
        let left = self.ast.query_expr(&left);
        return left;
    }

    fn parse_unary_expression(&mut self) -> &ASTExpression {
        if let Some(operator) = self.parse_unary_operator() {
            self.consume();
            let operand = self.parse_unary_expression().id;
            return self.ast.unary_expression(operator, operand);
        }
        return self.parse_primary_expression();
    }

    fn parse_unary_operator(&mut self) -> Option<ASTUnaryOperator> {
        let token = self.current();
        let kind = match token.kind {
            TokenKind::Minus => {
                Some(ASTUnaryOperatorKind::Minus)
            }
            TokenKind::Tilde => {
                Some(ASTUnaryOperatorKind::BitwiseNot)
            }
            _ => {
                None
            }
        };
        return kind.map(|kind| ASTUnaryOperator::new(kind, token.clone()));
    }

    fn parse_binary_operator(&mut self) -> Option<ASTBinaryOperator> {
        let token = self.current();
        let kind = match token.kind {
            TokenKind::Plus => {
                Some(ASTBinaryOperatorKind::Plus)
            }
            TokenKind::Minus => {
                Some(ASTBinaryOperatorKind::Minus)
            }
            TokenKind::Asterisk => {
                Some(ASTBinaryOperatorKind::Multiply)
            }
            TokenKind::Slash => {
                Some(ASTBinaryOperatorKind::Divide)
            }
            TokenKind::Ampersand => {
                Some(ASTBinaryOperatorKind::BitwiseAnd)
            }
            TokenKind::Pipe => {
                Some(ASTBinaryOperatorKind::BitwiseOr)
            }
            TokenKind::Caret => {
                Some(ASTBinaryOperatorKind::BitwiseXor)
            }
            TokenKind::DoubleAsterisk => {
                Some(ASTBinaryOperatorKind::Power)
            }
            TokenKind::EqualsEquals => {
                Some(ASTBinaryOperatorKind::Equals)
            }
            TokenKind::BangEquals => {
                Some(ASTBinaryOperatorKind::NotEquals)
            }
            TokenKind::LessThan => {
                Some(ASTBinaryOperatorKind::LessThan)
            }
            TokenKind::LessThanEquals => {
                Some(ASTBinaryOperatorKind::LessThanOrEqual)
            }
            TokenKind::GreaterThan => {
                Some(ASTBinaryOperatorKind::GreaterThan)
            }
            TokenKind::GreaterThanEquals => {
                Some(ASTBinaryOperatorKind::GreaterThanOrEqual)
            }

            _ => {
                None
            }
        };
        return kind.map(|kind| ASTBinaryOperator::new(kind, token.clone()));
    }

    fn parse_primary_expression(&mut self) -> &ASTExpression {
        let token = self.consume().clone();
        let expr = match token.kind {
            TokenKind::Number(number) => {
                self.ast.number_expression(token, number)
            }
            TokenKind::LeftParen => {
                let expr = self.parse_expression().id;
                let left_paren = token;
                let right_paren = self.consume_and_check(TokenKind::RightParen).clone();
                self.ast.parenthesized_expression(left_paren, expr, right_paren)
            }
            TokenKind::Identifier => {
                self.ast.identifier_expression(token)
            }
            TokenKind::True | TokenKind::False => {
                let value = token.kind == TokenKind::True;
                self.ast.boolean_expression(token, value)
            }
            TokenKind::Quote => {
                let open_quote = token;
                // todo: handle escape sequences
                let mut is_escape = false;
                let mut parts: Vec<StringPart> = Vec::new();
                while self.current().kind != TokenKind::Quote && !self.is_at_end() {
                    let literal = &self.current().span.literal;
                    if literal == "\\" {
                        is_escape = true;
                    } else {
                        if is_escape {
                            let mut char_iter = literal.chars();
                            let c = EscapedCharacter::from_char(char_iter.next().unwrap());
                            if let Some(c) = c {
                                parts.push(StringPart::EscapeSequence(c))
                            } else {
                                self.diagnostics_bag.borrow_mut().report_invalid_escape_sequence(&self.current());
                            }
                            // push the rest of the string
                            let remainder = char_iter.as_str();
                            if remainder.len() > 0 {
                                parts.push(StringPart::Literal(remainder.to_string()));
                            }
                            is_escape = false;
                        } else {
                            parts.push(StringPart::Literal(literal.clone()));
                        }
                    }
                    self.consume_no_whitespace();
                }
                let close_quote = self.consume_and_check(TokenKind::Quote).clone();
                self.ast.string_expression(open_quote, ASTString::new(parts), close_quote)
            }
            TokenKind::Self_ => {
                self.ast.self_expression(token)
            }
            _ => {
                self.diagnostics_bag.borrow_mut().report_expected_expression(&token);
                self.ast.error_expression(token.span.clone())
            }
        }.id.clone();
        let mut expr = expr;
        loop {
            match self.current().kind {
                TokenKind::LeftParen => {
                    expr = self.parse_call_expression(expr).id;
                }
                TokenKind::Dot => {
                    expr = self.parse_member_expression(expr).id;
                }
                _ => {
                    return self.ast.query_expr(&expr)
                }
            }
        }
    }

    fn parse_member_expression(&mut self, object: ASTExprId) -> &ASTExpression {
        let dot = self.consume_and_check(TokenKind::Dot).clone();
        let property = self.consume_and_check(TokenKind::Identifier).clone();
        return self.ast.member_expression(object, dot, property);
    }

    fn parse_call_expression(&mut self, callee: ASTExprId) -> &ASTExpression {
        let left_paren = self.consume_and_check(TokenKind::LeftParen).clone();
        let mut arguments = Vec::new();
        while self.current().kind != TokenKind::RightParen && !self.is_at_end() {
            arguments.push(self.parse_expression().id);
            if self.current().kind != TokenKind::RightParen {
                self.consume_and_check(TokenKind::Comma);
            }
        }
        let right_paren = self.consume_and_check(TokenKind::RightParen).clone();
        return self.ast.call_expression(callee, left_paren, arguments, right_paren);
    }

    fn peek(&self, offset: isize) -> &Token {
        let index = (self.current.get_value() as isize + offset) as usize;
        self.get_token(index)
    }

    fn get_token(&self, mut index: usize) -> &Token {
        if index >= self.tokens.len() {
            index = self.tokens.len() - 1;
        }
        self.tokens.get(index).unwrap()
    }

    fn current(&self) -> &Token {
        self.peek(0)
    }

    fn consume(&self) -> &Token {
        let pos = self.current.get_value();
        self.current.increment();
        self.consume_whitespace();
        self.get_token(pos)
    }

    fn consume_no_whitespace(&self) -> &Token {
        self.current.increment();
        self.peek(-1)
    }

    fn consume_whitespace(&self) {
        while self.current().kind == TokenKind::Newline || self.current().kind == TokenKind::Whitespace {
            self.consume_no_whitespace();
        }
    }

    fn consume_and_check_multiple(&self, kinds: &[TokenKind]) -> &Token {
        let token = self.consume();
        if !kinds.contains(&token.kind) {
            self.diagnostics_bag.borrow_mut().report_unexpected_token_multiple(
                kinds,
                token,
            );
        }
        token
    }

    fn consume_and_check(&self, kind: TokenKind) -> &Token {
        let token = self.consume();
        if token.kind != kind {
            self.diagnostics_bag.borrow_mut().report_unexpected_token(
                &kind,
                token,
            );
        }
        token
    }
}