use termion::color::{Fg, Reset};

use crate::ast::{Ast, ASTAssignmentExpression, ASTBinaryExpression, ASTBlockStatement, ASTBooleanExpression, ASTCallExpression, ASTCastExpression, ASTCharExpression, ASTDerefExpression, ASTExpression, ASTExpressionKind, ASTFuncDeclStatement, ASTIdentifierExpression, ASTIfStatement, ASTIndexExpression, ASTLetStatement, ASTMemberAccessExpression, ASTModDeclStatement, ASTNumberExpression, ASTParenthesizedExpression, ASTRefExpression, ASTReturnStatement, ASTStatement, ASTStatementKind, ASTStringExpression, ASTStructDeclStatement, ASTStructInitExpression, ASTUnaryExpression, ASTWhileStatement};
use crate::ast::printer::ASTPrinter;
use crate::text::span::TextSpan;

pub trait ASTVisitor {
    fn get_ast(&self) -> &Ast;

    fn do_visit_statement(&mut self, statement: &ASTStatement) {
        match &statement.kind {
            ASTStatementKind::Expression(expr) => {
                self.visit_expression(expr);
            }
            ASTStatementKind::Let(stmt) => {
                self.visit_let_statement(stmt, &statement);
            }
            ASTStatementKind::If(stmt) => {
                self.visit_if_statement(stmt);
            }
            ASTStatementKind::Block(stmt) => {
                self.visit_block_statement(stmt);
            }
            ASTStatementKind::While(stmt) => {
                self.visit_while_statement(stmt);
            }
            ASTStatementKind::FuncDecl(stmt) => {
                self.visit_func_decl_statement(stmt);
            }
            ASTStatementKind::Return(return_stmt) => {
                self.visit_return_statement(return_stmt, &statement);
            }
            ASTStatementKind::StructDecl(struct_decl_stmt) => {
                self.visit_struct_decl_statement(struct_decl_stmt);
            }
            ASTStatementKind::ModDecl(mod_decl_stmt) => {
                self.visit_mod_decl_statement(mod_decl_stmt);
            }
        }
    }


    fn visit_mod_decl_statement(&mut self, mod_decl_stmt: &ASTModDeclStatement);

    fn visit_struct_decl_statement(&mut self, struct_decl_stmt: &ASTStructDeclStatement);

    fn visit_func_decl_statement(&mut self, func_decl_statement: &ASTFuncDeclStatement);

    fn visit_return_statement(&mut self, return_statement: &ASTReturnStatement, stmt: &ASTStatement) {
        if let Some(expr) = &return_statement.return_value {
            self.visit_expression(expr);
        }
    }

    fn visit_while_statement(&mut self, while_statement: &ASTWhileStatement) {
        self.visit_expression(&while_statement.condition);
        self.visit_statement(&while_statement.body);
    }
    fn visit_block_statement(&mut self, block_statement: &ASTBlockStatement) {
        for statement in &block_statement.statements {
            self.visit_statement(statement);
        }
    }

    fn visit_if_statement(&mut self, if_statement: &ASTIfStatement) {
        self.visit_expression(&if_statement.condition);
        self.visit_statement(&if_statement.then_branch);
        if let Some(else_branch) = &if_statement.else_branch {
            self.visit_statement(&else_branch.else_statement);
        }
    }
    fn visit_let_statement(&mut self, let_statement: &ASTLetStatement, statement: &ASTStatement);
    fn visit_statement(&mut self, statement: &ASTStatement) {
        self.do_visit_statement(statement);
    }
    fn do_visit_expression(&mut self, expression: &ASTExpression) {
        match &expression.kind {
            ASTExpressionKind::Number(number) => {
                self.visit_number_expression(number, &expression);
            }
            ASTExpressionKind::Binary(expr) => {
                self.visit_binary_expression(expr, &expression);
            }
            ASTExpressionKind::Parenthesized(expr) => {
                self.visit_parenthesized_expression(expr, &expression);
            }
            ASTExpressionKind::Error(span) => {
                self.visit_error(span);
            }
            ASTExpressionKind::Identifier(expr) => {
                self.visit_identifier_expression(expr, &expression);
            }
            ASTExpressionKind::Unary(expr) => {
                self.visit_unary_expression(expr, &expression);
            }
            ASTExpressionKind::Assignment(expr) => {
                self.visit_assignment_expression(expr, &expression);
            }
            ASTExpressionKind::Boolean(expr) => {
                self.visit_boolean_expression(expr, &expression);
            }
            ASTExpressionKind::Call(expr) => {
                self.visit_call_expression(expr, &expression);
            }

            ASTExpressionKind::String(expr) => {
                self.visit_string_expression(expr, &expression);
            }
            ASTExpressionKind::Ref(expr) => {
                self.visit_ref_expression(expr);
            }
            ASTExpressionKind::Deref(expr) => {
                self.visit_deref_expression(expr);
            }
            ASTExpressionKind::Char(expr) => {
                self.visit_char_expression(expr, &expression);
            }
            ASTExpressionKind::Cast(expr) => {
                self.visit_cast_expression(expr, &expression);
            }
            ASTExpressionKind::MemberAccess(expr) => {
                self.visit_member_access_expression(expr, &expression);
            }
            ASTExpressionKind::StructInit(expr) => {
                self.visit_struct_init_expression(expr, &expression);
            }
            ASTExpressionKind::Index(expr) => {
                self.visit_index_expression(expr, &expression);
            }
        }
    }

    fn visit_index_expression(&mut self, index_expression: &ASTIndexExpression, expr: &ASTExpression);

    fn visit_struct_init_expression(&mut self, struct_init_expression: &ASTStructInitExpression, expr: &ASTExpression);

    fn visit_member_access_expression(&mut self, member_access_expression: &ASTMemberAccessExpression, expr: &ASTExpression);

    fn visit_cast_expression(&mut self, cast_expression: &ASTCastExpression, expr: &ASTExpression);

    fn visit_char_expression(&mut self, char_expression: &ASTCharExpression, expr: &ASTExpression);

    fn visit_deref_expression(&mut self, deref_expression: &ASTDerefExpression);
    fn visit_ref_expression(&mut self, ref_expression: &ASTRefExpression);

    fn visit_string_expression(&mut self, string_expression: &ASTStringExpression, expr: &ASTExpression);

    fn visit_call_expression(&mut self, call_expression: &ASTCallExpression, expr: &ASTExpression) {
        for argument in &call_expression.arguments {
            self.visit_expression(argument);
        }
    }
    fn visit_expression(&mut self, expression: &ASTExpression) {
        self.do_visit_expression(expression);
    }

    fn visit_assignment_expression(&mut self, assignment_expression: &ASTAssignmentExpression, expr: &ASTExpression);

    fn visit_identifier_expression(&mut self, variable_expression: &ASTIdentifierExpression, expr: &ASTExpression);

    fn visit_number_expression(&mut self, number: &ASTNumberExpression, expr: &ASTExpression);

    fn visit_boolean_expression(&mut self, boolean: &ASTBooleanExpression, expr: &ASTExpression);

    fn visit_error(&mut self, span: &TextSpan);

    fn visit_unary_expression(&mut self, unary_expression: &ASTUnaryExpression, expr: &ASTExpression);

    fn visit_binary_expression(&mut self, binary_expression: &ASTBinaryExpression, expr: &ASTExpression) {
        self.visit_expression(&binary_expression.left);
        self.visit_expression(&binary_expression.right);
    }
    fn visit_parenthesized_expression(&mut self, parenthesized_expression: &ASTParenthesizedExpression, expr: &ASTExpression) {
        self.visit_expression(&parenthesized_expression.expression);
    }
}


