use crate::ast::{Ast, FunctionDeclaration, ModuleDeclaration, StructDeclaration};
use crate::ast::expr::{AssignExpr, BinExpr, BlockExpr, BoolExpr, CallExpr, CastExpr, CharExpr, DerefExpr, Expr, IdenExpr, IfExpr, IndexExpr, MemberAccessExpr, NumberExpr, ParenExpr, RefExpr, StringExpr, StructInitExpr, UnaryExpr, WhileExpr};
use crate::ast::stmt::{  LetStmt, ReturnStmt, Stmt, ParameterSyntax};
use crate::ast::visitor::ASTVisitor;
use crate::text::span::TextSpan;

pub struct Formatter<'a> {
    ast: &'a Ast,
    indent: usize,
    buffer: String,
}

impl<'a> Formatter<'a> {
    pub fn new(ast: &'a Ast) -> Self {
        Self {
            ast,
            indent: 0,
            buffer: String::new(),
        }
    }

    fn write(&mut self, text: &str) {
        self.buffer.push_str(text);
    }

    fn new_line(&mut self) {
        self.buffer.push_str("
");
    }

    fn whitespace(&mut self) {
        self.write(" ");
    }

    fn write_indent(&mut self) {
        self.write(&format!("{}", " ".repeat(self.indent)));
    }

    fn indent(&mut self) {
        self.indent += 4;
    }

    fn dedent(&mut self) {
        self.indent -= 4;
    }

    pub fn format(mut self) -> String {
        self.ast.visit(&mut self);
        self.buffer
    }

    fn visit_block(&mut self, block: &BlockExpr) {
        self.visit_block_expr(block);
    }
}

impl ASTVisitor for Formatter<'_> {
    fn get_ast(&self) -> &Ast {
        self.ast
    }

    fn visit_module_declaration(&mut self, mod_decl_stmt: &ModuleDeclaration) {
        self.write("mod");
        self.whitespace();
        self.write(&mod_decl_stmt.identifier.span.literal);
    }

    fn visit_struct_declaration(&mut self, struct_decl_stmt: &StructDeclaration) {
        self.write("struct");
        self.whitespace();
        self.write(&struct_decl_stmt.identifier.span.literal);
        self.write(" {");
        self.new_line();
        self.indent();
        for (i, field) in struct_decl_stmt.fields.iter().enumerate() {
            self.write_indent();
            self.write(&field.identifier.span.literal);
            self.write(":");
            self.whitespace();
            self.write(format!("{}", &field.ty.ty).as_str());
            if i < struct_decl_stmt.fields.len() - 1 {
                self.write(",");
            }
            self.new_line();
        }
        self.dedent();
        self.write("}");
        self.new_line();
    }


    fn visit_function_declaration(&mut self, func_decl_statement: &FunctionDeclaration) {
        self.write("func");
        self.whitespace();
        for modifier in &func_decl_statement.modifier_tokens {
            self.write(&modifier.span.literal);
            self.whitespace();
        }
        self.write(&func_decl_statement.identifier.span.literal);
        for (i, parameter) in func_decl_statement.parameters.iter().enumerate() {
            if i == 0 {
                self.write("(");
            }
            self.write(&parameter.identifier.span.literal);
            self.write(":");
            self.whitespace();
            self.write(format!("{}", &parameter.type_annotation.ty).as_str());


            if i < func_decl_statement.parameters.len() - 1 {
                self.write(",");
                self.whitespace();
            } else {
                self.write(")");
            }
        }
        self.whitespace();
        if let Some(return_type) = &func_decl_statement.return_type {
            self.write("->");
            self.whitespace();
            self.write(format!("{}", &return_type.ty).as_str());
            self.whitespace();
        }
        if let Some(body) = &func_decl_statement.body {
            self.write("{");
            self.new_line();
            self.indent();
            for statement in body.stmts.iter() {
                self.visit_statement(statement);
            }
            self.dedent();
            self.write_indent();
            self.write("}");
        }
    }

    fn visit_return_statement(&mut self, return_statement: &ReturnStmt) {
        self.write("return");
        if let Some(expr) = &return_statement.expr {
            self.whitespace();
            self.visit_expression(expr);
        }
    }

    fn visit_while_expr(&mut self, while_statement: &WhileExpr) {
        self.write("while");
        self.whitespace();
        self.visit_expression(&while_statement.condition);
        self.whitespace();
        self.visit_block(&while_statement.body);
    }

    fn visit_block_expr(&mut self, block: &BlockExpr) {
        self.write("{");
        self.new_line();
        self.indent();
        for statement in &block.stmts {
            self.visit_statement(statement);
        }
        self.dedent();
        self.write_indent();
        self.write("}");
    }

    fn visit_if_expr(&mut self, if_expr: &IfExpr) {
        self.write("if");
        self.whitespace();
        self.visit_expression(&if_expr.condition);
        self.whitespace();
        self.visit_block(&if_expr.then_branch);
        if let Some(else_statement) = &if_expr.else_branch {
            self.whitespace();
            self.write("else");
            self.whitespace();
            self.visit_block(&else_statement.expr);
        }
    }

    fn visit_let_statement(&mut self, let_statement: &LetStmt, statement: &Stmt) {
        self.write("let");
        self.whitespace();
        self.write(&let_statement.identifier.span.literal);
        self.whitespace();
        self.write("=");
        self.whitespace();
        self.visit_expression(&let_statement.initializer);
    }

    fn visit_statement(&mut self, statement: &Stmt) {
        self.write_indent();
        self.do_visit_statement(statement);
        self.new_line();
    }

    fn visit_index_expression(&mut self, index_expression: &IndexExpr, expr: &Expr) {
        self.visit_expression(&index_expression.target);
        self.write("[");
        self.visit_expression(&index_expression.index);
        self.write("]");
    }

    fn visit_struct_init_expression(&mut self, struct_init_expression: &StructInitExpr, expr: &Expr) {
        self.write(&struct_init_expression.identifier.to_string());
        self.write("{");
        for (i, field_init) in struct_init_expression.fields.iter().enumerate() {
            self.write(&field_init.identifier.span.literal);
            self.write(":");
            self.whitespace();
            self.visit_expression(&field_init.initializer);
            if i < struct_init_expression.fields.len() - 1 {
                self.write(",");
                self.whitespace();
            }
        }
        self.write("}");
    }

    fn visit_member_access_expression(&mut self, member_access_expression: &MemberAccessExpr, expr: &Expr) {
        self.visit_expression(&member_access_expression.expr);
        self.write(".");
        self.write(&member_access_expression.member.span.literal);
    }

    fn visit_cast_expression(&mut self, cast_expression: &CastExpr, expr: &Expr) {
        self.visit_expression(&cast_expression.expr);
        self.write(" as ");
        self.write(format!("{}", &cast_expression.ty).as_str());
    }

    fn visit_char_expression(&mut self, char_expression: &CharExpr, expr: &Expr) {
        self.write("'");
        self.write(&char_expression.value.to_string());
        self.write("'");
    }

    fn visit_deref_expression(&mut self, deref_expression: &DerefExpr) {
        self.write("*");
        self.visit_expression(&deref_expression.expr);
    }

    fn visit_ref_expression(&mut self, ref_expression: &RefExpr) {
        self.write("&");
        self.visit_expression(&ref_expression.expr);
    }

    fn visit_string_expression(&mut self, string_expression: &StringExpr, expr: &Expr) {
        self.write("\"");
        self.write(&string_expression.string.to_raw_string());
        self.write("\"");
    }

    fn visit_call_expression(&mut self, call_expression: &CallExpr, expr: &Expr) {
        self.visit_expression(&call_expression.callee);
        self.write("(");
        for (i, arg) in call_expression.arguments.iter().enumerate() {
            self.visit_expression(arg);
            if i < call_expression.arguments.len() - 1 {
                self.write(",");
                self.whitespace();
            }
        }
        self.write(")");
    }

    fn visit_assignment_expression(&mut self, assignment_expression: &AssignExpr, expr: &Expr) {
        self.visit_expression(&assignment_expression.left);
        self.whitespace();
        self.write("=");
        self.whitespace();
        self.visit_expression(&assignment_expression.right);
    }

    fn visit_identifier_expression(&mut self, variable_expression: &IdenExpr, expr: &Expr) {
        self.write(&variable_expression.identifier.to_string());
    }

    fn visit_number_expression(&mut self, number: &NumberExpr, expr: &Expr) {
        self.write(&number.token.span.literal);
    }

    fn visit_boolean_expression(&mut self, boolean: &BoolExpr, expr: &Expr) {
        self.write(&boolean.token.span.literal);
    }
    fn visit_error(&mut self, span: &TextSpan) {
        panic!("Error at {:?}", span);
    }

    fn visit_unary_expression(&mut self, unary_expression: &UnaryExpr, expr: &Expr) {
        self.write(&unary_expression.operator.token.span.literal);
        self.visit_expression(&unary_expression.operand);
    }

    fn visit_binary_expression(&mut self, binary_expression: &BinExpr, expr: &Expr) {
        self.visit_expression(&binary_expression.left);
        self.whitespace();
        self.write(&binary_expression.operator.token.span.literal);
        self.whitespace();
        self.visit_expression(&binary_expression.right);
    }

    fn visit_parenthesized_expression(&mut self, parenthesized_expression: &ParenExpr, expr: &Expr) {
        self.write("(");
        self.visit_expression(&parenthesized_expression.expr);
        self.write(")");
    }
}
