use termion::color::{Fg, Reset};

use crate::ast::{Ast, FunctionDeclaration, Item, ItemKind, ModuleDeclaration, StructDeclaration};
use crate::ast::expr::{AssignExpr, BinExpr, BoolExpr, CallExpr, CastExpr, CharExpr, DerefExpr, IdenExpr, IndexExpr, MemberAccessExpr, ParenExpr, RefExpr, StringExpr, StructInitExpr, UnaryExpr, BlockExpr, Expr, ExprKind, IfExpr, NumberExpr, WhileExpr};
use crate::ast::printer::ASTPrinter;
use crate::ast::stmt::{ LetStmt, ReturnStmt, Stmt, StmtKind};
use crate::text::span::TextSpan;

pub trait ASTVisitor {
    fn get_ast(&self) -> &Ast;

    fn visit_item(&mut self, item: &Item) {
        Self::visit_item_default(self, item);
    }

    fn visit_item_default(&mut self, item: &Item) {
        match &item.kind {
            ItemKind::FunctionDeclaration(func_decl) => {
                self.visit_function_declaration(func_decl);
            }
            ItemKind::ModuleDeclaration(module_decl) => {
                self.visit_module_declaration(module_decl);
            }
            ItemKind::StructDeclaration(struct_decl) => {
                self.visit_struct_declaration(struct_decl);
            }
            ItemKind::NotAllowed(_) => {
                unreachable!()
            }
        }
    }

    fn do_visit_statement(&mut self, statement: &Stmt) {
        match &statement.kind {
            StmtKind::Expr(expr) => {
                self.visit_expression(expr);
            }
            StmtKind::Let(stmt) => {
                self.visit_let_statement(stmt, &statement);
            }
            StmtKind::Return(return_stmt) => {
                self.visit_return_statement(return_stmt);
            }
        }
    }


    fn visit_module_declaration(&mut self, mod_decl_stmt: &ModuleDeclaration);

    fn visit_struct_declaration(&mut self, struct_decl_stmt: &StructDeclaration);

    fn visit_function_declaration(&mut self, func_decl_statement: &FunctionDeclaration);

    fn visit_return_statement(&mut self, return_statement: &ReturnStmt) {
        if let Some(expr) = &return_statement.expr {
            self.visit_expression(expr);
        }
    }

    fn visit_while_expr(&mut self, expr: &WhileExpr) {
        self.visit_expression(&expr.condition);
        self.visit_block_expr(&expr.body);
    }
    fn visit_block_expr(&mut self, block_expr: &BlockExpr) {
        for stmt in &block_expr.stmts {
            self.visit_statement(stmt);
        }
    }

    fn visit_if_expr(&mut self, expr: &IfExpr) {
        self.visit_expression(&expr.condition);
        self.visit_block_expr(&expr.then_branch);
        if let Some(else_branch) = &expr.else_branch {
            self.visit_block_expr(&else_branch.expr);
        }
    }
    fn visit_let_statement(&mut self, let_statement: &LetStmt, statement: &Stmt);
    fn visit_statement(&mut self, statement: &Stmt) {
        self.do_visit_statement(statement);
    }
    fn do_visit_expression(&mut self, expression: &Expr) {
        match &expression.kind {
            ExprKind::Number(number) => {
                self.visit_number_expression(number, &expression);
            }
            ExprKind::Binary(expr) => {
                self.visit_binary_expression(expr, &expression);
            }
            ExprKind::Parenthesized(expr) => {
                self.visit_parenthesized_expression(expr, &expression);
            }
            ExprKind::Error(span) => {
                self.visit_error(span);
            }
            ExprKind::Identifier(expr) => {
                self.visit_identifier_expression(expr, &expression);
            }
            ExprKind::Unary(expr) => {
                self.visit_unary_expression(expr, &expression);
            }
            ExprKind::Assignment(expr) => {
                self.visit_assignment_expression(expr, &expression);
            }
            ExprKind::Boolean(expr) => {
                self.visit_boolean_expression(expr, &expression);
            }
            ExprKind::Call(expr) => {
                self.visit_call_expression(expr, &expression);
            }

            ExprKind::String(expr) => {
                self.visit_string_expression(expr, &expression);
            }
            ExprKind::Ref(expr) => {
                self.visit_ref_expression(expr);
            }
            ExprKind::Deref(expr) => {
                self.visit_deref_expression(expr);
            }
            ExprKind::Char(expr) => {
                self.visit_char_expression(expr, &expression);
            }
            ExprKind::Cast(expr) => {
                self.visit_cast_expression(expr, &expression);
            }
            ExprKind::MemberAccess(expr) => {
                self.visit_member_access_expression(expr, &expression);
            }
            ExprKind::StructInit(expr) => {
                self.visit_struct_init_expression(expr, &expression);
            }
            ExprKind::Index(expr) => {
                self.visit_index_expression(expr, &expression);
            }
            ExprKind::Block(expr) => {
                self.visit_block_expr(expr);
            }
            ExprKind::While(expr) => {
                self.visit_while_expr(expr);
            }
            ExprKind::If(expr) => {
                self.visit_if_expr(expr);
            }
        }
    }

    fn visit_index_expression(&mut self, index_expression: &IndexExpr, expr: &Expr);

    fn visit_struct_init_expression(&mut self, struct_init_expression: &StructInitExpr, expr: &Expr);

    fn visit_member_access_expression(&mut self, member_access_expression: &MemberAccessExpr, expr: &Expr);

    fn visit_cast_expression(&mut self, cast_expression: &CastExpr, expr: &Expr);

    fn visit_char_expression(&mut self, char_expression: &CharExpr, expr: &Expr);

    fn visit_deref_expression(&mut self, deref_expression: &DerefExpr);
    fn visit_ref_expression(&mut self, ref_expression: &RefExpr);

    fn visit_string_expression(&mut self, string_expression: &StringExpr, expr: &Expr);

    fn visit_call_expression(&mut self, call_expression: &CallExpr, expr: &Expr) {
        for argument in &call_expression.arguments {
            self.visit_expression(argument);
        }
    }
    fn visit_expression(&mut self, expression: &Expr) {
        self.do_visit_expression(expression);
    }

    fn visit_assignment_expression(&mut self, assignment_expression: &AssignExpr, expr: &Expr);

    fn visit_identifier_expression(&mut self, variable_expression: &IdenExpr, expr: &Expr);

    fn visit_number_expression(&mut self, number: &NumberExpr, expr: &Expr);

    fn visit_boolean_expression(&mut self, boolean: &BoolExpr, expr: &Expr);

    fn visit_error(&mut self, span: &TextSpan);

    fn visit_unary_expression(&mut self, unary_expression: &UnaryExpr, expr: &Expr);

    fn visit_binary_expression(&mut self, binary_expression: &BinExpr, expr: &Expr) {
        self.visit_expression(&binary_expression.left);
        self.visit_expression(&binary_expression.right);
    }
    fn visit_parenthesized_expression(&mut self, parenthesized_expression: &ParenExpr, expr: &Expr) {
        self.visit_expression(&parenthesized_expression.expr);
    }
}


