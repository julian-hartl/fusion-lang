use std::cell::Cell;

use crate::ast::{Ast, ASTBinaryOperator, ASTBinaryOperatorKind, ASTElseStatement, ASTExpression, ASTFunctionReturnType, ASTStatement, ASTString, ASTStructField, ASTStructInitField, ASTUnaryExpression, ASTUnaryOperator, ASTUnaryOperatorKind, EscapedCharacter, FuncDeclParameter, NormalFuncDeclParameter, PtrSyntax, QualifiedIdentifier, StaticTypeAnnotation, StringPart, TypeSyntax};
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
    is_parsing_condition: bool,
    encountered_module_declarations: Vec<Token>
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
            is_parsing_condition: false,
            encountered_module_declarations: Vec::new(),
        }
    }

    pub fn get_encountered_module_declarations(&self) -> &Vec<Token> {
        &self.encountered_module_declarations
    }

    pub fn parse(&mut self) {
        while let Some(stmt) = self.next_statement() {
            self.ast.statements.push(stmt);
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
            TokenKind::Mod => {
                self.parse_module_declaration()
            }
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
            TokenKind::Struct => {
                self.parse_struct_declaration()
            }
            _ => {
                self.parse_expression_statement()
            }
        };
        self.consume_whitespace();
        if self.current().kind == TokenKind::SemiColon {
            self.consume();
        }
        stmt
    }

    fn parse_module_declaration(&mut self) -> ASTStatement {
        let mod_token = self.consume_and_check(TokenKind::Mod).clone();
        let identifier = self.consume_and_check(TokenKind::Identifier).clone();
        self.encountered_module_declarations.push(identifier.clone());

        self.ast.module_decl_statement(mod_token, identifier)
    }

    fn parse_struct_declaration(&mut self) -> ASTStatement {
        let struct_token = self.consume_and_check(TokenKind::Struct).clone();
        let identifier = self.consume_and_check(TokenKind::Identifier).clone();
        self.ast.structs.push(identifier.clone());
        let mut fields = Vec::new();
        let open_brace = self.consume_and_check(TokenKind::OpenBrace).clone();
        while self.current().kind != TokenKind::CloseBrace && !self.is_at_end() {
            let field_identifier = self.consume_and_check(TokenKind::Identifier).clone();
            let field_type = self.parse_type_annotation();
            fields.push(ASTStructField {
                ty: field_type,
                identifier: field_identifier,
            });
            if self.current().kind == TokenKind::Comma {
                self.consume();
            }
        }
        let close_brace = self.consume_and_check(TokenKind::CloseBrace).clone();
        self.ast.struct_decl_statement(struct_token, identifier, fields, open_brace, close_brace)
    }


    fn parse_function_declaration(&mut self) -> ASTStatement {
        let funct_token = self.consume_and_check(TokenKind::Func).clone();
        let modifier_tokens = self.parse_optional_function_modifiers();
        let identifier = self.consume_and_check(TokenKind::Identifier).clone();
        let parameters = self.parse_optional_parameter_list();
        let return_type = self.parse_optional_return_type();
        let body = match self.current().kind {
            TokenKind::OpenBrace => {
                let mut statements = Vec::new();
                self.consume_and_check(TokenKind::OpenBrace);
                while self.current().kind != TokenKind::CloseBrace && !self.is_at_end() {
                    statements.push(self.parse_statement());
                }
                self.consume_and_check(TokenKind::CloseBrace);
                Some(statements)
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
            let ty = self.parse_type().clone();
            return Some(ASTFunctionReturnType::new(
                arrow,
                ty,
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
                    _ => {
                        FuncDeclParameter::Normal(NormalFuncDeclParameter {
                            mut_token: parser.maybe_consume(TokenKind::Mut).cloned(),
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

    fn parse_return_statement(&mut self) -> ASTStatement {
        let return_keyword = self.consume_and_check(TokenKind::Return).clone();
        if self.current().kind == TokenKind::Newline || self.is_at_end() {
            return self.ast.return_statement(return_keyword, None, true);
        }
        let expression = self.parse_expression();
        self.ast.return_statement(return_keyword, Some(expression), true)
    }

    fn parse_while_statement(&mut self) -> ASTStatement {
        let while_keyword = self.consume_and_check(TokenKind::While).clone();
        self.is_parsing_condition = true;
        let condition_expr = self.parse_expression();
        self.is_parsing_condition = false;
        let body = self.parse_statement();
        self.ast.while_statement(while_keyword, condition_expr, body)
    }

    fn parse_block_statement(&mut self) -> ASTStatement {
        let open_brace = self.consume_and_check(TokenKind::OpenBrace).clone();
        let mut statements = Vec::new();
        while self.current().kind != TokenKind::CloseBrace && !self.is_at_end() {
            statements.push(self.parse_statement());
        }
        let close_brace = self.consume_and_check(TokenKind::CloseBrace).clone();
        self.ast.block_statement(open_brace, statements, close_brace)
    }

    fn parse_if_statement(&mut self) -> ASTStatement {
        let if_keyword = self.consume_and_check(TokenKind::If).clone();
        self.is_parsing_condition = true;
        let condition_expr = self.parse_expression();
        self.is_parsing_condition = false;
        let then = self.parse_statement();
        let else_statement = self.parse_optional_else_statement();
        self.ast.if_statement(if_keyword, condition_expr, then, else_statement)
    }

    fn parse_optional_else_statement(&mut self) -> Option<ASTElseStatement> {
        if self.current().kind == TokenKind::Else {
            let else_keyword = self.consume_and_check(TokenKind::Else).clone();
            let else_statement = self.parse_statement();
            return Some(ASTElseStatement::new(else_keyword, else_statement));
        }
        return None;
    }


    fn parse_let_statement(&mut self) -> ASTStatement {
        self.consume_and_check(TokenKind::Let);
        let mut_token = self.maybe_consume(TokenKind::Mut).cloned();
        let identifier = self.consume_and_check(TokenKind::Identifier).clone();
        let optional_type_annotation = self.parse_optional_type_annotation();
        self.consume_and_check(TokenKind::Equals);
        let expr = self.parse_expression();

        self.ast.let_statement(mut_token, identifier, expr, optional_type_annotation)
    }

    fn parse_optional_type_annotation(&mut self) -> Option<StaticTypeAnnotation> {
        if self.current().kind == TokenKind::Colon {
            return Some(self.parse_type_annotation());
        }
        return None;
    }

    fn parse_type_annotation(&mut self) -> StaticTypeAnnotation {
        let colon = self.consume_and_check(TokenKind::Colon).clone();
        let ty = self.parse_type();
        return StaticTypeAnnotation::new(colon, ty);
    }

    fn parse_type(&mut self) -> TypeSyntax {
        self.consume_whitespace();
        let ptr = if self.current().kind == TokenKind::Asterisk {
            Some((self.consume_and_check(TokenKind::Asterisk).clone(), self.maybe_consume(TokenKind::Mut).cloned()))
        } else {
            None
        };
        let starting_id = self.consume_and_check(TokenKind::Identifier).clone();
        let type_name = self.parse_qualified_identifier(starting_id);
        TypeSyntax::new(type_name, ptr.map(|(asterisk, mut_)| PtrSyntax {
            mut_token: mut_,
            star: asterisk,
        }))
    }

    fn parse_expression_statement(&mut self) -> ASTStatement {
        let expr = self.parse_expression();
        self.ast.expression_statement(expr)
    }

    fn parse_expression(&mut self) -> ASTExpression {
        self.parse_assignment_expression()
    }

    fn parse_assignment_expression(&mut self) -> ASTExpression {
        let assignee = self.parse_binary_expression(0);
        self.consume_whitespace();
        if self.current().kind == TokenKind::Equals {
            let equals = self.consume_and_check(TokenKind::Equals).clone();
            let expr = self.parse_expression();
            return self.ast.assignment_expression(assignee, equals, expr);
        }
        assignee
    }

    fn parse_binary_expression(&mut self, precedence: u8) -> ASTExpression {
        let mut left = self.parse_cast_expression();

        while let Some(operator) = self.parse_binary_operator() {
            let operator_precedence = operator.precedence();
            if operator_precedence < precedence {
                break;
            }
            self.consume();
            let right = self.parse_binary_expression(operator_precedence);
            left = self.ast.binary_expression(operator, left, right);
        }
        left
    }

    fn parse_cast_expression(&mut self) -> ASTExpression {
        let expr = self.parse_deref_expression();
        if self.current().kind == TokenKind::As {
            let as_keyword = self.consume_and_check(TokenKind::As).clone();
            let ty = self.parse_type();
            return self.ast.cast_expression(expr, as_keyword, ty);
        }
        expr
    }

    fn parse_deref_expression(&mut self) -> ASTExpression {
        if self.current().kind == TokenKind::Asterisk {
            let star = self.consume_and_check(TokenKind::Asterisk).clone();
            let expr = self.parse_deref_expression();
            return self.ast.deref_expression(star, expr);
        }
        self.parse_ref_expression()
    }

    fn parse_ref_expression(&mut self) -> ASTExpression {
        if self.current().kind == TokenKind::Ampersand {
            let ampersand = self.consume_and_check(TokenKind::Ampersand).clone();
            let mut_token = self.maybe_consume(TokenKind::Mut).cloned();
            let expr = self.parse_deref_expression();
            return self.ast.ref_expression(ampersand, mut_token, expr);
        }
        self.parse_unary_expression()
    }

    fn parse_unary_expression(&mut self) -> ASTExpression {
        if let Some(operator) = self.parse_unary_operator() {
            self.consume();
            let operand = self.parse_unary_expression();
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
            TokenKind::Percent => {
                Some(ASTBinaryOperatorKind::Modulo)
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

    fn parse_primary_expression(&mut self) -> ASTExpression {
        let token = self.consume().clone();
        let expr = match token.kind {
            TokenKind::Number(number) => {
                self.ast.number_expression(token, number)
            }
            TokenKind::LeftParen => {
                let expr = self.parse_expression();
                let left_paren = token;
                let right_paren = self.consume_and_check(TokenKind::RightParen).clone();
                self.ast.parenthesized_expression(left_paren, expr, right_paren)
            }
            TokenKind::Identifier => {
                let qualified = self.parse_qualified_identifier(token);
                self.consume_whitespace();
                if self.current().kind == TokenKind::OpenBrace && !self.is_parsing_condition {
                    self.parse_struct_init_expression(qualified)
                } else {
                    self.ast.identifier_expression(qualified)
                }
            }
            TokenKind::True | TokenKind::False => {
                let value = token.kind == TokenKind::True;
                self.ast.boolean_expression(token, value)
            }
            TokenKind::DoubleQuote => {
                self.parse_string_expression(token)
            }
            TokenKind::SingleQuote => {
                self.parse_char_expression(token)
            }
            _ => {
                self.diagnostics_bag.borrow_mut().report_expected_expression(&token);
                self.ast.error_expression(token.span.clone())
            }
        };
        let mut expr = expr;
        while self.current().kind != TokenKind::Newline || self.current().kind != TokenKind::SemiColon {
            match self.current().kind {
                TokenKind::LeftParen => {
                    expr = self.parse_call_expression(expr);
                }
                TokenKind::Dot => {
                    expr = self.parse_member_access_expression(expr);
                }
                TokenKind::Arrow => {
                    expr = self.parse_member_access_expression(expr);
                }
                _ => {
                    return expr;
                }
            }
        }
        return expr;
    }

    fn parse_qualified_identifier(&mut self, token: Token) -> QualifiedIdentifier {
        let identifiers = self.parse_identifier_chain(token);
        let qualified = QualifiedIdentifier::new(identifiers);
        qualified
    }

    fn parse_identifier_chain(&mut self, identifier: Token) -> Vec<Token> {
        let mut identifiers = vec![identifier];
        while self.current().kind == TokenKind::ColonColon {
            self.consume();
            let identifier = self.consume_and_check(TokenKind::Identifier).clone();
            identifiers.push(identifier);
        }
        return identifiers;
    }

    fn parse_struct_init_expression(&mut self, identifier: QualifiedIdentifier) -> ASTExpression {
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

    fn parse_member_access_expression(&mut self, expr: ASTExpression) -> ASTExpression {
        let dot = self.consume_and_check_multiple(&[TokenKind::Dot, TokenKind::Arrow]).clone();
        let identifier = self.consume_and_check(TokenKind::Identifier).clone();
        self.ast.member_access_expression(expr, dot, identifier)
    }

    fn parse_char_expression(&mut self, token: Token) -> ASTExpression {
        let open_quote = token;
        let mut char = '\0';
        let mut has_error = false;
        while self.current().kind != TokenKind::SingleQuote && !self.is_at_end() {
            let token = self.consume();
            let literal = &token.span.literal;
            if !has_error {
                if literal.len() != 1 {
                    self.diagnostics_bag.borrow_mut().report_invalid_character_literal(&token.span);
                    has_error = true;
                } else {
                    char = literal.chars().next().unwrap();
                }
            }
        }
        let close_quote = self.consume_and_check(TokenKind::SingleQuote).clone();
        self.ast.character_expression(open_quote, char, close_quote)
    }

    fn parse_string_expression(&mut self, token: Token) -> ASTExpression {
        let open_quote = token;
        let mut is_escape = false;
        let mut parts: Vec<StringPart> = Vec::new();
        while self.current().kind != TokenKind::DoubleQuote && !self.is_at_end() {
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
        let close_quote = self.consume_and_check(TokenKind::DoubleQuote).clone();
        self.ast.string_expression(open_quote, ASTString::new(parts), close_quote)
    }

    fn parse_call_expression(&mut self, callee: ASTExpression) -> ASTExpression {
        let left_paren = self.consume_and_check(TokenKind::LeftParen).clone();
        let mut arguments = self.parse_comma_separated_list(TokenKind::RightParen, |parser| parser.parse_expression()).iter().map(|expr| expr.clone()).collect();
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

    fn consume_whitespace_only(&self) {
        while self.current().kind == TokenKind::Whitespace {
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

    fn maybe_consume(&self, kind: TokenKind) -> Option<&Token> {
        if self.current().kind == kind {
            Some(self.consume())
        } else {
            None
        }
    }
}