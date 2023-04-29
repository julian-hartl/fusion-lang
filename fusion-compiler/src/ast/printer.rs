use termion::color;

use crate::ast::*;
use crate::text::span::TextSpan;

pub struct ASTPrinter<'a> {
    indent: usize,
    pub result: String,
    pub ast: &'a Ast,
}

impl <'a> ASTPrinter<'a> {
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
        self.add_type(&type_annotation.type_name.span.literal);
    }

    fn add_string(&mut self, string: &str) {
        self.result.push_str(&format!("{}{}",
                                      Self::STRING_COLOR.fg_str(),
                                      string, ));
    }

    pub fn new(ast: &'a Ast) -> Self {
        Self { indent: 0, result: String::new(),ast }
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

    fn visit_class_statement(&mut self, class_statement: &ASTClassStatement, statement: &ASTStatement) {
        self.add_padding();
        self.add_keyword("class");
        self.add_whitespace();
        self.add_text(&class_statement.identifier.span.literal);
        self.add_whitespace();
        if let Some(constructor) = &class_statement.constructor {
            self.add_text("(");
            for (i, field) in constructor.fields.iter().enumerate() {
                if i != 0 {
                    self.add_text(",");
                    self.add_whitespace();
                }
                self.add_text(&field.identifier.span.literal);
                self.add_type_annotation(&field.type_annotation);
            }
            self.add_text(")");
        }
        self.add_text("{");
        self.add_newline();
        self.indent += 1;
        for member in class_statement.body.members.iter() {
            match member {
                ASTClassMember::Field(field) => {
                    self.add_padding();
                    self.add_text(&field.identifier.span.literal);
                    self.add_type_annotation(&field.type_annotation);
                    self.add_text(";");
                    self.add_newline();
                }
                ASTClassMember::Method(method) => {
                    self.add_padding();
                    let stmt = self.ast.query_stmt(&method.func_decl).into_func_decl();
                    self.visit_func_decl_statement(
                        &stmt,
                    );
                }
                ASTClassMember::Invalid(_) => {}
            }
        }
        self.indent -= 1;
        self.add_padding();
        self.add_text("}");
        self.add_newline();
    }

    fn visit_func_decl_statement(&mut self, func_decl_statement: &ASTFuncDeclStatement) {
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
            match parameter {
                FuncDeclParameter::Normal(parameter) => {
                    self.add_text(&parameter.identifier.span.literal);
                    self.add_type_annotation(&parameter.type_annotation);
                }
                FuncDeclParameter::Self_(_) => {
                    self.add_keyword("self");
                }
            }

        }
        if !are_parameters_empty {
            self.add_text(")");
            self.add_whitespace();
        }
        if let Some(body) = &func_decl_statement.body {
            self.visit_statement(body);
        }
    }
    fn visit_return_statement(&mut self, return_statement: &ASTReturnStatement, stmt: &ASTStatement) {
        self.add_keyword("return");
        if let Some(expression) = &return_statement.return_value {
            self.add_whitespace();
            self.visit_expression(expression);
        }
    }
    fn visit_while_statement(&mut self, while_statement: &ASTWhileStatement) {
        self.add_keyword("while");
        self.add_whitespace();
        self.visit_expression(&while_statement.condition);
        self.add_whitespace();
        self.visit_statement(&while_statement.body);
    }
    fn visit_block_statement(&mut self, block_statement: &ASTBlockStatement) {
        self.add_text("{");
        self.add_newline();
        self.indent += 1;
        for statement in &block_statement.statements {
            self.visit_statement(statement);
        }
        self.indent -= 1;
        self.add_padding();
        self.add_text("}");
    }

    fn visit_if_statement(&mut self, if_statement: &ASTIfStatement) {
        self.add_keyword("if");
        self.add_whitespace();
        self.visit_expression(&if_statement.condition);
        self.add_whitespace();
        self.visit_statement(&if_statement.then_branch);

        if let Some(else_branch) = &if_statement.else_branch {
            self.add_keyword("else");
            self.add_whitespace();
            self.visit_statement(&else_branch.else_statement);
        }
    }

    fn visit_let_statement(&mut self, let_statement: &ASTLetStatement, statement: &ASTStatement) {
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

    fn visit_statement(&mut self, statement: &ASTStmtId) {
        self.add_padding();
        Self::do_visit_statement(self, statement);
        self.result.push_str(&format!("{}\n",
                                      Fg(Reset),
        ));
    }

    fn visit_self_expression(&mut self, self_expression: &ASTSelfExpression, expr: &ASTExpression) {
        self.add_keyword("self");
    }

    fn visit_member_access_expression(&mut self, member_access_expression: &ASTMemberAccessExpression, expr: &ASTExpression) {
        self.visit_expression(&member_access_expression.object);
        self.add_text(".");
        self.add_text(&member_access_expression.target.span.literal);
    }

    fn visit_string_expression(&mut self, string_expression: &ASTStringExpression, expr: &ASTExpression) {
        self.add_string("\"");
        self.add_string(&string_expression.string.to_raw_string());
        self.add_string("\"");
    }

    fn visit_call_expression(&mut self, call_expression: &ASTCallExpression, expr: &ASTExpression) {
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

    fn visit_assignment_expression(&mut self, assignment_expression: &ASTAssignmentExpression, expr: &ASTExpression) {
        self.add_variable(assignment_expression.identifier.span.literal.as_str());
        self.add_whitespace();
        self.add_text("=");
        self.add_whitespace();
        self.visit_expression(&assignment_expression.expression);
    }

    fn visit_identifier_expression(&mut self, variable_expression: &ASTIdentifierExpression, expr: &ASTExpression) {
        self.result.push_str(&format!("{}{}",
                                      Self::VARIABLE_COLOR.fg_str(),
                                      variable_expression.identifier.span.literal, ));
    }

    fn visit_number_expression(&mut self, number: &ASTNumberExpression, expr: &ASTExpression) {
        self.result.push_str(&format!("{}{}",
                                      Self::NUMBER_COLOR.fg_str(),
                                      number.number, ));
    }

    fn visit_boolean_expression(&mut self, boolean: &ASTBooleanExpression, expr: &ASTExpression) {
        self.add_boolean(boolean.value);
    }

    fn visit_error(&mut self, span: &TextSpan) {
        self.result.push_str(&format!("{}{}",
                                      Self::TEXT_COLOR.fg_str(),
                                      span.literal, ));
    }

    fn visit_unary_expression(&mut self, unary_expression: &ASTUnaryExpression, expr: &ASTExpression) {
        self.result.push_str(&format!("{}{}",
                                      Self::TEXT_COLOR.fg_str(),
                                      unary_expression.operator.token.span.literal, ));
        self.visit_expression(&unary_expression.operand);
    }

    fn visit_binary_expression(&mut self, binary_expression: &ASTBinaryExpression, expr: &ASTExpression) {
        self.visit_expression(&binary_expression.left);
        self.add_whitespace();
        self.result.push_str(&format!("{}{}",
                                      Self::TEXT_COLOR.fg_str(),
                                      binary_expression.operator.token.span.literal, ));
        self.add_whitespace();
        self.visit_expression(&binary_expression.right);
    }

    fn visit_parenthesized_expression(&mut self, parenthesized_expression: &ASTParenthesizedExpression, expr: &ASTExpression) {
        self.result.push_str(&format!("{}{}",
                                      Self::TEXT_COLOR.fg_str(),
                                      "(", ));
        self.visit_expression(&parenthesized_expression.expression);
        self.result.push_str(&format!("{}{}",
                                      Self::TEXT_COLOR.fg_str(),
                                      ")", ));
    }
}
