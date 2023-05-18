use std::fmt::format;
use std::ops::Not;

use crate::hir::{HIR, HIRAssignmentExpression, HIRAssignmentTargetKind, HIRBinaryExpression, HIRBlockStatement, HIRCallee, HIRCallExpression, HIRCastExpression, HIRDerefExpression, HIRGlobal, HIRIfStatement, HIRIndexExpression, HIRLiteralExpression, HIRLiteralValue, HIRParenthesizedExpression, HIRRefExpression, HIRReturnStatement, HIRStatement, HIRStructInitExpression, HIRUnaryExpression, HIRVariableDeclarationStatement, HIRVariableExpression, HIRWhileStatement};
use crate::hir::visitor::HIRVisitor;
use crate::modules::scopes::{GlobalScope, GlobalScopeCell};

pub struct HIRVisualizer<'a> {
    output: String,
    indent: usize,
    hir: &'a HIR,
    scope: GlobalScopeCell,
}

impl<'a> HIRVisualizer<'a> {
    pub fn new(
        hir: &'a HIR,
        scope: GlobalScopeCell,
    ) -> Self {
        HIRVisualizer {
            output: String::new(),
            indent: 0,
            hir,
            scope,
        }
    }

    pub fn visualize(mut self) -> String {
        let scope = self.scope.clone();
        let scope_ref = scope.borrow();
        for global in self.hir.globals.iter() {
            match global {
                HIRGlobal::Variable {
                    id,
                    initializer
                } => {
                    let variable = scope_ref.get_variable(id);
                    self.write("let");
                    self.write_whitespace();
                    if variable.is_mutable {
                        self.write("mut");
                        self.write_whitespace();
                    }
                    self.write(&variable.name);
                    self.write_whitespace();
                    self.write(":");
                    self.write_whitespace();
                    self.write(format!("{}", variable.ty).as_str());
                    self.write_whitespace();
                    self.write("=");
                    self.write_whitespace();
                    self.visit_expr(initializer);
                    self.new_line();
                }
            }
        }
        for (function, body) in self.hir.functions(&scope_ref) {
            self.write("func");
            self.write_whitespace();
            self.write(&function.name.name);
            if !function.parameters.is_empty() {
                self.write("(");
                for (i, parameter_id) in function.parameters.iter().enumerate() {
                    let parameter = scope_ref.get_variable(parameter_id);
                    let param = format!("{}: {}", parameter.name, parameter.ty);
                    let param = param.as_str();
                    if i == 0 {
                        if parameter.is_mutable {
                            self.write("mut ");
                        }
                        self.write(param);
                    } else {
                        self.write(", ");
                        if parameter.is_mutable {
                            self.write("mut ");
                        }
                        self.write(param);
                    }
                }
                self.write(")");
            }
            self.write_whitespace();
            self.write("->");
            self.write_whitespace();
            self.write(format!("{}", function.return_type).as_str());
            if let Some(body) = body {
                self.write_whitespace();
                self.block(body);
            }
            self.new_line();
        }
        self.output
    }

    fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn new_line(&mut self) {
        self.write("\n");
    }

    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.write("    ");
        }
    }

    fn indent(&mut self) {
        self.indent += 1;
    }

    fn unindent(&mut self) {
        self.indent -= 1;
    }

    fn write_whitespace(&mut self) {
        self.write(" ");
    }

    fn visit_stmts(&mut self, stmts: &Vec<HIRStatement>) {
        for stmt in stmts {
            self.visit_stmt(stmt);
        }
    }

    fn block(&mut self, stmts: &Vec<HIRStatement>) {
        self.write("{");
        self.new_line();
        self.indent();
        self.visit_stmts(stmts);
        self.unindent();
        self.write_indent();
        self.write("}");
        self.new_line();
    }
}

impl HIRVisitor for HIRVisualizer<'_> {
    fn visit_return_stmt(&mut self, stmt: &HIRReturnStatement) {
        self.write("return");
        self.write_whitespace();
        self.visit_expr(&stmt.expression);
    }

    fn visit_variable_declaration_stmt(&mut self, stmt: &HIRVariableDeclarationStatement) {
        self.write("let");
        self.write_whitespace();
        let scope = self.scope.clone();
        let scope_ref = scope.borrow();
        let variable = scope_ref.get_variable(&stmt.variable_id);
        if variable.is_mutable {
            self.write("mut");
            self.write_whitespace();
        }
        self.write(&variable.name);
        self.write(":");
        self.write_whitespace();
        self.write(format!("{}", &variable.ty).as_str());
        self.write_whitespace();
        self.write("=");
        self.write_whitespace();
        self.visit_expr(&stmt.initializer);
    }

    fn visit_if_stmt(&mut self, stmt: &HIRIfStatement) {
        self.write("if");
        self.write_whitespace();
        self.visit_expr(&stmt.condition);
        self.write_whitespace();
        self.block(&stmt.then);
        if let Some(else_stmts) = &stmt.else_ {
            self.write("else");
            self.write_whitespace();
            self.block(else_stmts);
        }
    }

    fn visit_while_stmt(&mut self, stmt: &HIRWhileStatement) {
        self.write("while");
        self.write_whitespace();
        self.visit_expr(&stmt.condition);
        self.write_whitespace();
        self.block(&stmt.body);
    }

    fn visit_block_stmt(&mut self, stmt: &HIRBlockStatement) {
        self.block(&stmt.statements);
    }

    fn visit_binary_expr(&mut self, expr: &HIRBinaryExpression) {
        self.visit_expr(&expr.left);
        self.write_whitespace();
        self.write(format!("{}", expr.op).as_str());
        self.write_whitespace();
        self.visit_expr(&expr.right);
    }

    fn visit_unary_expr(&mut self, expr: &HIRUnaryExpression) {
        self.write(format!("{}", expr.op).as_str());
        self.visit_expr(&expr.operand);
    }

    fn visit_literal_expr(&mut self, expr: &HIRLiteralExpression) {
        let value = match &expr.value {
            HIRLiteralValue::Integer(value) => {
                value.to_string()
            }
            HIRLiteralValue::Boolean(value) => {
                value.to_string()
            }
            HIRLiteralValue::String(value) => {
                format!("\"{}\"", value)
            }
            HIRLiteralValue::Char(value) => {
                format!("'{}'", value)
            }
        };
        self.write(value.as_str());
    }

    fn visit_variable_expr(&mut self, expr: &HIRVariableExpression) {
        let scope = self.scope.clone();
        let scope_ref = scope.borrow();
        let variable = scope_ref.get_variable(&expr.variable_id);
        self.write(&variable.name);
    }

    fn visit_assignment_expr(&mut self, expr: &HIRAssignmentExpression) {
        match &expr.target.kind {
            HIRAssignmentTargetKind::Variable(variable_id) => {
                let scope = self.scope.clone();
                let scope_ref = scope.borrow();
                let variable = scope_ref.get_variable(variable_id);
                self.write(&variable.name)
            }
            HIRAssignmentTargetKind::Deref(expr) => {
                self.write("*");
                self.visit_expr(expr);
            }
            HIRAssignmentTargetKind::Error => {}
            HIRAssignmentTargetKind::Field(id, target) => {
                self.visit_expr(target);
                self.write(".");
                let scope = self.scope.clone();
                let scope_ref = scope.borrow();
                let field = scope_ref.get_field(id);
                self.write(&field.name);
            }
        };
        self.write_whitespace();
        self.write("=");
        self.write_whitespace();
        self.visit_expr(&expr.value);
    }

    fn visit_call_expr(&mut self, expr: &HIRCallExpression) {
        match &expr.callee {
            HIRCallee::Function(id) => {
                let scope = self.scope.clone();
                let scope_ref = scope.borrow();
                let function = scope_ref.get_function(id);
                self.write(&function.name.name);
            }
            HIRCallee::Undeclared(name) => {
                self.write(name);
            }
            HIRCallee::Invalid => {
                self.write("<invalid>");
            }
        };

        self.write("(");
        for (i, arg) in expr.arguments.iter().enumerate() {
            self.visit_expr(arg);
            if i < expr.arguments.len() - 1 {
                self.write(",");
                self.write_whitespace();
            }
        }
        self.write(")");
    }

    fn visit_parenthesized_expr(&mut self, expr: &HIRParenthesizedExpression) {
        self.write("(");
        self.visit_expr(&expr.expression);
        self.write(")");
    }

    fn visit_void_expr(&mut self) {
        self.write("()");
    }

    fn visit_stmt(&mut self, statement: &HIRStatement) {
        self.write_indent();
        self.default_visit_stmt(statement);
        self.new_line();
    }

    fn visit_ref_expr(&mut self, expr: &HIRRefExpression) {
        self.write("&");
        self.visit_expr(&expr.expression);
    }

    fn visit_deref_expr(&mut self, expr: &HIRDerefExpression) {
        self.write("*");
        self.visit_expr(&expr.expression);
    }

    fn visit_cast_expr(&mut self, expr: &HIRCastExpression) {
        self.visit_expr(&expr.expression);
        self.write(format!(" as {}", expr.ty).as_str());
    }

    fn visit_struct_init_expr(&mut self, expr: &HIRStructInitExpression) {
        let scope = self.scope.clone();
        let scope_ref = scope.borrow();
        let struct_ = scope_ref.get_struct(&expr.struct_id);
        self.write(&struct_.name.name);
        self.write("{");
        for (i, field) in expr.fields.iter().enumerate() {
            let field_name = &scope_ref.get_field(&field.field_id).name;
            self.write(&field_name);
            self.write(":");
            self.write_whitespace();
            self.visit_expr(&field.value);
            if i < expr.fields.len() - 1 {
                self.write(",");
                self.write_whitespace();
            }
        }
        self.write("}");
    }

    fn visit_index_expr(&mut self, expr: &HIRIndexExpression) {
        self.visit_expr(&expr.target);
        self.write("[");
        self.visit_expr(&expr.index);
        self.write("]");
    }
}