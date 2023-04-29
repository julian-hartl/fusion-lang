use crate::ast::{Ast, ASTAssignmentExpression, ASTBinaryExpression, ASTBlockStatement, ASTBooleanExpression, ASTCallExpression, ASTClassMember, ASTClassStatement, ASTExpression, ASTFuncDeclStatement, ASTIdentifierExpression, ASTIfStatement, ASTLetStatement, ASTMemberAccessExpression, ASTNumberExpression, ASTParenthesizedExpression, ASTReturnStatement, ASTSelfExpression, ASTStatement, ASTStmtId, ASTStringExpression, ASTUnaryExpression, ASTWhileStatement, FuncDeclParameter};
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

    pub fn format(mut self) -> String {
        self.ast.visit(&mut self);
        self.buffer
    }
}

impl ASTVisitor for Formatter<'_> {
    fn get_ast(&self) -> &Ast {
        self.ast
    }

    fn visit_class_statement(&mut self, class_statement: &ASTClassStatement, statement: &ASTStatement) {
        self.write_indent();
        self.write("class");
        self.whitespace();
        self.write(&class_statement.identifier.span.literal);
        self.whitespace();
        if let Some(constructor) = &class_statement.constructor {
            for (i, parameter) in constructor.fields.iter().enumerate() {
                if i == 0 {
                    self.write("(");
                }
                self.write(&parameter.identifier.span.literal);
                self.write(":");
                self.whitespace();
                self.write(&parameter.type_annotation.type_name.span.literal);
                if i < constructor.fields.len() - 1 {
                    self.write(",");
                    self.whitespace();
                } else {
                    self.write(")");
                }
            }
            self.whitespace();
        }

        self.write("{");
        self.new_line();
        self.indent += 4;
        for member in &class_statement.body.members {
            match member {
                ASTClassMember::Field(field) => {
                    self.write_indent();
                    self.write(&field.identifier.span.literal);
                    self.write(":");
                    self.whitespace();
                    self.write(&field.type_annotation.type_name.span.literal);
                    self.write(";");
                    self.new_line();
                }
                ASTClassMember::Method(method) => {
                    self.write_indent();
                    let func = self.ast.query_stmt(&method.func_decl).into_func_decl();
                    self.visit_func_decl_statement(&func);
                }
                ASTClassMember::Invalid(_) => {}
            }
        }
        self.indent -= 4;
        self.write_indent();
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
                    self.write(&parameter.type_annotation.type_name.span.literal);
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
            self.write(&return_type.type_name.span.literal);
            self.whitespace();
        }
        if let Some(body) = &func_decl_statement.body {
            self.visit_statement(body);
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
        self.visit_statement(&while_statement.body);
    }

    fn visit_block_statement(&mut self, block_statement: &ASTBlockStatement) {
        self.write("{");
        self.new_line();
        self.indent += 4;
        for statement in &block_statement.statements {
            self.write_indent();
            self.visit_statement(statement);
        }
        self.indent -= 4;
        self.write_indent();
        self.write("}");
    }

    fn visit_if_statement(&mut self, if_statement: &ASTIfStatement) {
        self.write("if");
        self.whitespace();
        self.visit_expression(&if_statement.condition);
        self.whitespace();
        self.visit_statement(&if_statement.then_branch);
        if let Some(else_statement) = &if_statement.else_branch {
            self.whitespace();
            self.write("else");
            self.whitespace();
            self.visit_statement(&else_statement.else_statement);
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

    fn visit_statement(&mut self, statement: &ASTStmtId) {
        self.do_visit_statement(statement);
        self.new_line();
    }

    fn visit_self_expression(&mut self, self_expression: &ASTSelfExpression, expr: &ASTExpression) {
        self.write("self");
    }

    fn visit_member_access_expression(&mut self, member_access_expression: &ASTMemberAccessExpression, expr: &ASTExpression) {
        self.visit_expression(&member_access_expression.object);
        self.write(".");
        self.write(&member_access_expression.target.span.literal);
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
        self.write(&assignment_expression.identifier.span.literal);
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
