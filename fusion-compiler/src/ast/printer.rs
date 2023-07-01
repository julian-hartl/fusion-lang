use std::fmt::format;
use clap::builder::Str;

use termion::color;

use crate::ast::*;
use crate::text::span::TextSpan;

pub struct ASTPrinter<'a> {
    indent: usize,
    pub result: String,
    pub ast: &'a Ast,
}

impl<'a> ASTPrinter<'a> {
    const NUMBER_COLOR: color::Cyan = color::Cyan;
    const TEXT_COLOR: color::LightWhite = color::LightWhite;
    const KEYWORD_COLOR: color::Magenta = color::Magenta;
    const VARIABLE_COLOR: color::Green = color::Green;
    const BOOLEAN_COLOR: color::Yellow = color::Yellow;
    const TYPE_COLOR: color::LightBlue = color::LightBlue;
    const STRING_COLOR: color::LightRed = color::LightRed;

    fn add_whitespace(&mut self) {
        self.result.push_str(" ");
    }

    fn add_newline(&mut self) {
        self.result.push_str("
");
    }

    fn add_keyword(&mut self, keyword: &str) {
        self.result.push_str(&format!("{}{}",
                                      Self::KEYWORD_COLOR.fg_str(),
                                      keyword, ));
    }

    fn add_text(&mut self, text: &str) {
        self.result.push_str(&format!("{}{}",
                                      Self::TEXT_COLOR.fg_str(),
                                      text, ));
    }

    fn add_variable(&mut self, variable: &str) {
        self.result.push_str(&format!("{}{}",
                                      Self::VARIABLE_COLOR.fg_str(),
                                      variable, ));
    }

    fn add_padding(&mut self) {
        for _ in 0..self.indent {
            self.result.push_str("  ");
        }
    }

    fn add_boolean(&mut self, boolean: bool) {
        self.result.push_str(&format!("{}{}",
                                      Self::BOOLEAN_COLOR.fg_str(),
                                      boolean, ));
    }

    fn add_type(&mut self, type_: &str) {
        self.result.push_str(&format!("{}{}",
                                      Self::TYPE_COLOR.fg_str(),
                                      type_, ));
    }

    fn add_type_annotation(
        &mut self,
        type_annotation: &StaticTypeAnnotation,
    ) {
        self.add_text(":");
        self.add_whitespace();
        self.add_type(format!("{}", &type_annotation.ty).as_str());
    }

    fn add_string(&mut self, string: &str) {
        self.result.push_str(&format!("{}{}",
                                      Self::STRING_COLOR.fg_str(),
                                      string, ));
    }

    pub fn new(ast: &'a Ast) -> Self {
        Self { indent: 0, result: String::new(), ast }
    }

    pub fn print(mut self) -> String {
        self.ast.visit(&mut self);
        self.result
    }
}

impl ASTVisitor for ASTPrinter<'_> {
    fn get_ast(&self) -> &Ast {
        self.ast
    }

    fn visit_module_declaration(&mut self, mod_decl_stmt: &ModuleDeclaration) {
        self.add_keyword("mod");
        self.add_whitespace();
        self.add_text(&mod_decl_stmt.identifier.span.literal);
    }

    fn visit_struct_declaration(&mut self, struct_decl_stmt: &StructDeclaration) {
        self.add_keyword("struct");
        self.add_whitespace();
        self.add_text(&struct_decl_stmt.identifier.span.literal);
        self.add_whitespace();
        self.add_text("{");
        self.add_newline();
        self.indent += 1;
        for field in &struct_decl_stmt.fields {
            self.add_padding();
            self.add_text(&field.identifier.span.literal);
            self.add_type_annotation(&field.ty);
            self.add_newline();
        }
        self.indent -= 1;
        self.add_padding();
        self.add_text("}");
    }


    fn visit_function_declaration(&mut self, func_decl_statement: &FunctionDeclaration) {
        self.add_keyword("func");
        self.add_whitespace();
        self.add_text(&func_decl_statement.identifier.span.literal);
        let are_parameters_empty = func_decl_statement.parameters.is_empty();
        if !are_parameters_empty {
            self.add_text("(");
        } else {
            self.add_whitespace();
        }
        for (i, parameter) in func_decl_statement.parameters.iter().enumerate() {
            if i != 0 {
                self.add_text(",");
                self.add_whitespace();
            }
            self.add_text(&parameter.identifier.span.literal);
            self.add_type_annotation(&parameter.type_annotation);
        }
        if !are_parameters_empty {
            self.add_text(")");
            self.add_whitespace();
        }
        if let Some(body) = &func_decl_statement.body {
            self.visit_block_expr(body);
        }
    }
    fn visit_return_statement(&mut self, return_statement: &ReturnStmt) {
        self.add_keyword("return");
        if let Some(expression) = &return_statement.expr {
            self.add_whitespace();
            self.visit_expression(expression);
        }
    }
    fn visit_while_expr(&mut self, while_expr: &WhileExpr) {
        self.add_keyword("while");
        self.add_whitespace();
        self.visit_expression(&while_expr.condition);
        self.add_whitespace();
        self.visit_block_expr(&while_expr.body);
    }
    fn visit_block_expr(&mut self, block_expr: &BlockExpr) {
        self.add_text("{");
        self.add_newline();
        self.indent += 1;
        for statement in &block_expr.stmts {
            self.visit_statement(statement);
        }
        self.indent -= 1;
        self.add_padding();
        self.add_text("}");
    }

    fn visit_if_expr(&mut self, if_statement: &IfExpr) {
        self.add_keyword("if");
        self.add_whitespace();
        self.visit_expression(&if_statement.condition);
        self.add_whitespace();
        self.visit_block_expr(&if_statement.then_branch);

        if let Some(else_branch) = &if_statement.else_branch {
            self.add_keyword("else");
            self.add_whitespace();
            self.visit_block_expr(&else_branch.expr);
        }
    }

    fn visit_let_statement(&mut self, let_statement: &LetStmt, statement: &Stmt) {
        self.add_keyword("let");
        self.add_whitespace();
        self.add_text(
            let_statement.identifier.span.literal.as_str(), );
        if let Some(type_annotation) = &let_statement.type_annotation {
            self.add_type_annotation(type_annotation);
        }
        self.add_whitespace();
        self.add_text("=");
        self.add_whitespace();
        self.visit_expression(&let_statement.initializer);
    }

    fn visit_statement(&mut self, statement: &Stmt) {
        self.add_padding();
        Self::do_visit_statement(self, statement);
        self.result.push_str(&format!("{}\n",
                                      Fg(Reset),
        ));
    }

    fn visit_index_expression(&mut self, index_expression: &IndexExpr, expr: &Expr) {
        self.visit_expression(&index_expression.target);
        self.add_text("[");
        self.visit_expression(&index_expression.index);
        self.add_text("]");
    }

    fn visit_struct_init_expression(&mut self, struct_init_expression: &StructInitExpr, expr: &Expr) {
        self.add_text(&struct_init_expression.identifier.to_string());
        self.add_text("{");
        for (i, field_init) in struct_init_expression.fields.iter().enumerate() {
            if i != 0 {
                self.add_text(",");
                self.add_whitespace();
            }
            self.add_text(&field_init.identifier.span.literal);
            self.add_text(":");
            self.add_whitespace();
            self.visit_expression(&field_init.initializer);
        }
        self.add_text("}");
    }

    fn visit_member_access_expression(&mut self, member_access_expression: &MemberAccessExpr, expr: &Expr) {
        self.visit_expression(&member_access_expression.expr);
        self.add_text(".");
        self.add_text(&member_access_expression.member.span.literal);
    }

    fn visit_cast_expression(&mut self, cast_expression: &CastExpr, expr: &Expr) {
        self.visit_expression(&cast_expression.expr);
        self.add_whitespace();
        self.add_text("as");
        self.add_whitespace();
        self.add_type(format!("{}", &cast_expression.ty).as_str());
    }

    fn visit_char_expression(&mut self, char_expression: &CharExpr, expr: &Expr) {
        self.add_string("'");
        self.add_string(&char_expression.value.to_string());
        self.add_string("'");
    }

    fn visit_deref_expression(&mut self, deref_expression: &DerefExpr) {
        self.add_text("*");
        self.visit_expression(&deref_expression.expr);
    }

    fn visit_ref_expression(&mut self, ref_expression: &RefExpr) {
        self.add_text("&");
        self.visit_expression(&ref_expression.expr);
    }


    fn visit_string_expression(&mut self, string_expression: &StringExpr, expr: &Expr) {
        self.add_string("\"");
        self.add_string(&string_expression.string.to_raw_string());
        self.add_string("\"");
    }

    fn visit_call_expression(&mut self, call_expression: &CallExpr, expr: &Expr) {
        self.visit_expression(&call_expression.callee);
        self.add_text("(");
        for (i, argument) in call_expression.arguments.iter().enumerate() {
            if i != 0 {
                self.add_text(",");
                self.add_whitespace();
            }
            self.visit_expression(argument);
        }
        self.add_text(")");
    }

    fn visit_assignment_expression(&mut self, assignment_expression: &AssignExpr, expr: &Expr) {
        self.visit_expression(&assignment_expression.left);
        self.add_whitespace();
        self.add_text("=");
        self.add_whitespace();
        self.visit_expression(&assignment_expression.right);
    }

    fn visit_identifier_expression(&mut self, variable_expression: &IdenExpr, expr: &Expr) {
        self.result.push_str(&format!("{}{}",
                                      Self::VARIABLE_COLOR.fg_str(),
                                      variable_expression.identifier.to_string(), ));
    }

    fn visit_number_expression(&mut self, number: &NumberExpr, expr: &Expr) {
        self.result.push_str(&format!("{}{}",
                                      Self::NUMBER_COLOR.fg_str(),
                                      number.number, ));
    }

    fn visit_boolean_expression(&mut self, boolean: &BoolExpr, expr: &Expr) {
        self.add_boolean(boolean.value);
    }

    fn visit_error(&mut self, span: &TextSpan) {
        self.result.push_str(&format!("{}{}",
                                      Self::TEXT_COLOR.fg_str(),
                                      span.literal, ));
    }

    fn visit_unary_expression(&mut self, unary_expression: &UnaryExpr, expr: &Expr) {
        self.result.push_str(&format!("{}{}",
                                      Self::TEXT_COLOR.fg_str(),
                                      unary_expression.operator.token.span.literal, ));
        self.visit_expression(&unary_expression.operand);
    }

    fn visit_binary_expression(&mut self, binary_expression: &BinExpr, expr: &Expr) {
        self.visit_expression(&binary_expression.left);
        self.add_whitespace();
        self.result.push_str(&format!("{}{}",
                                      Self::TEXT_COLOR.fg_str(),
                                      binary_expression.operator.token.span.literal, ));
        self.add_whitespace();
        self.visit_expression(&binary_expression.right);
    }

    fn visit_parenthesized_expression(&mut self, parenthesized_expression: &ParenExpr, expr: &Expr) {
        self.result.push_str(&format!("{}{}",
                                      Self::TEXT_COLOR.fg_str(),
                                      "(", ));
        self.visit_expression(&parenthesized_expression.expr);
        self.result.push_str(&format!("{}{}",
                                      Self::TEXT_COLOR.fg_str(),
                                      ")", ));
    }
}
