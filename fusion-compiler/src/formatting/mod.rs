use crate::ast::{Ast, ASTAssignmentExpression, ASTBinaryExpression, ASTBlockStatement, ASTBooleanExpression, ASTCallExpression, ASTCastExpression, ASTCharExpression, ASTDerefExpression, ASTExpression, ASTFuncDeclStatement, ASTIdentifierExpression, ASTIfStatement, ASTLetStatement, ASTMemberAccessExpression, ASTNumberExpression, ASTParenthesizedExpression, ASTRefExpression, ASTReturnStatement, ASTStatement, ASTStringExpression, ASTStructDeclStatement, ASTStructInitExpression, ASTUnaryExpression, ASTWhileStatement, FuncDeclParameter};
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

    fn visit_statement_no_indent(&mut self, statement: &ASTStatement) {
        self.do_visit_statement(&statement);
    }
}

impl ASTVisitor for Formatter<'_> {
    fn get_ast(&self) -> &Ast {
        self.ast
    }

    fn visit_struct_decl_statement(&mut self, struct_decl_stmt: &ASTStructDeclStatement) {
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


    fn visit_func_decl_statement(&mut self, func_decl_statement: &ASTFuncDeclStatement) {
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
            match parameter {
                FuncDeclParameter::Normal(parameter) => {
                    self.write(&parameter.identifier.span.literal);
                    self.write(":");
                    self.whitespace();
                    self.write(format!("{}", &parameter.type_annotation.ty).as_str());
                }
                FuncDeclParameter::Self_(_) => {
                    self.write("self");
                }
            }

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
            for statement in body {
                self.visit_statement(statement);
            }
            self.dedent();
            self.write_indent();
            self.write("}");
        }
    }

    fn visit_return_statement(&mut self, return_statement: &ASTReturnStatement, stmt: &ASTStatement) {
        self.write("return");
        if let Some(expr) = &return_statement.return_value {
            self.whitespace();
            self.visit_expression(expr);
        }
    }

    fn visit_while_statement(&mut self, while_statement: &ASTWhileStatement) {
        self.write("while");
        self.whitespace();
        self.visit_expression(&while_statement.condition);
        self.whitespace();
        self.visit_statement_no_indent(&while_statement.body);
    }

    fn visit_block_statement(&mut self, block_statement: &ASTBlockStatement) {
        self.write("{");
        self.new_line();
        self.indent();
        for statement in &block_statement.statements {
            self.visit_statement(statement);
        }
        self.dedent();
        self.write_indent();
        self.write("}");
    }

    fn visit_if_statement(&mut self, if_statement: &ASTIfStatement) {
        self.write("if");
        self.whitespace();
        self.visit_expression(&if_statement.condition);
        self.whitespace();
        self.visit_statement_no_indent(&if_statement.then_branch);
        if let Some(else_statement) = &if_statement.else_branch {
            self.whitespace();
            self.write("else");
            self.whitespace();
            self.visit_statement_no_indent(&else_statement.else_statement);
        }
    }

    fn visit_let_statement(&mut self, let_statement: &ASTLetStatement, statement: &ASTStatement) {
        self.write("let");
        self.whitespace();
        self.write(&let_statement.identifier.span.literal);
        self.whitespace();
        self.write("=");
        self.whitespace();
        self.visit_expression(&let_statement.initializer);
    }

    fn visit_statement(&mut self, statement: &ASTStatement) {
        self.write_indent();
        self.do_visit_statement(statement);
        self.new_line();
    }

    fn visit_struct_init_expression(&mut self, struct_init_expression: &ASTStructInitExpression, expr: &ASTExpression) {
        self.write(&struct_init_expression.identifier.span.literal);
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

    fn visit_member_access_expression(&mut self, member_access_expression: &ASTMemberAccessExpression, expr: &ASTExpression) {
        self.visit_expression(&member_access_expression.expr);
        self.write(".");
        self.write(&member_access_expression.member.span.literal);
    }

    fn visit_cast_expression(&mut self, cast_expression: &ASTCastExpression, expr: &ASTExpression) {
        self.visit_expression(&cast_expression.expr);
        self.write(" as ");
        self.write(format!("{}", &cast_expression.ty).as_str());
    }

    fn visit_char_expression(&mut self, char_expression: &ASTCharExpression, expr: &ASTExpression) {
        self.write("'");
        self.write(&char_expression.value.to_string());
        self.write("'");
    }

    fn visit_deref_expression(&mut self, deref_expression: &ASTDerefExpression) {
        self.write("*");
        self.visit_expression(&deref_expression.expr);
    }

    fn visit_ref_expression(&mut self, ref_expression: &ASTRefExpression) {
        self.write("&");
        self.visit_expression(&ref_expression.expr);
    }

    fn visit_string_expression(&mut self, string_expression: &ASTStringExpression, expr: &ASTExpression) {
        self.write("\"");
        self.write(&string_expression.string.to_raw_string());
        self.write("\"");
    }

    fn visit_call_expression(&mut self, call_expression: &ASTCallExpression, expr: &ASTExpression) {
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

    fn visit_assignment_expression(&mut self, assignment_expression: &ASTAssignmentExpression, expr: &ASTExpression) {
        self.visit_expression(&assignment_expression.assignee);
        self.whitespace();
        self.write("=");
        self.whitespace();
        self.visit_expression(&assignment_expression.expression);
    }

    fn visit_identifier_expression(&mut self, variable_expression: &ASTIdentifierExpression, expr: &ASTExpression) {
        self.write(&variable_expression.identifier.span.literal);
    }

    fn visit_number_expression(&mut self, number: &ASTNumberExpression, expr: &ASTExpression) {
        self.write(&number.token.span.literal);
    }

    fn visit_boolean_expression(&mut self, boolean: &ASTBooleanExpression, expr: &ASTExpression) {
        self.write(&boolean.token.span.literal);
    }
    fn visit_error(&mut self, span: &TextSpan) {
        panic!("Error at {:?}", span);
    }

    fn visit_unary_expression(&mut self, unary_expression: &ASTUnaryExpression, expr: &ASTExpression) {
        self.write(&unary_expression.operator.token.span.literal);
        self.visit_expression(&unary_expression.operand);
    }

    fn visit_binary_expression(&mut self, binary_expression: &ASTBinaryExpression, expr: &ASTExpression) {
        self.visit_expression(&binary_expression.left);
        self.whitespace();
        self.write(&binary_expression.operator.token.span.literal);
        self.whitespace();
        self.visit_expression(&binary_expression.right);
    }

    fn visit_parenthesized_expression(&mut self, parenthesized_expression: &ASTParenthesizedExpression, expr: &ASTExpression) {
        self.write("(");
        self.visit_expression(&parenthesized_expression.expression);
        self.write(")");
    }
}
