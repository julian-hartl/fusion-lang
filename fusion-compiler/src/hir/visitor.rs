use crate::hir::{FunctionId, HIR, HIRAssignmentExpression, HIRBinaryExpression, HIRBinaryOperator, HIRBlockStatement, HIRCallExpression, HIRDerefExpression, HIRExpression, HIRExpressionKind, HIRExpressionStatement, HIRIfStatement, HIRLiteralExpression, HIRParenthesizedExpression, HIRRefExpression, HIRReturnStatement, HIRStatement, HIRStatementKind, HIRUnaryExpression, HIRUnaryOperator, HIRVariableDeclarationStatement, HIRVariableExpression, HIRWhileStatement};

pub trait HIRVisitor {
    fn visit(&mut self, hir: &HIR) {
        for (id, statements) in hir.function_bodies.iter() {
            self.visit_function(id, statements);
        }
    }

    fn visit_function(&mut self, function_id: &FunctionId, statements: &Vec<HIRStatement>) {
        self.default_visit_function(function_id, statements);
    }

    fn default_visit_function(&mut self, function_id: &FunctionId, statements: &Vec<HIRStatement>) {
        for statement in statements {
            self.visit_stmt(statement);
        }
    }

    fn visit_stmt(&mut self, statement: &HIRStatement) {
        self.default_visit_stmt(&statement);
    }

    fn default_visit_stmt(&mut self, statement: &HIRStatement) {
        match &statement.kind {
            HIRStatementKind::Return(stmt) => {
                self.visit_return_stmt(stmt);
            }
            HIRStatementKind::Expression(stmt) => {
                self.visit_expression_stmt(stmt);
            }
            HIRStatementKind::VariableDeclaration(stmt) => {
                self.visit_variable_declaration_stmt(stmt);
            }

            HIRStatementKind::If(stmt) => {
                self.visit_if_stmt(stmt);
            }

            HIRStatementKind::While(stmt) => {
                self.visit_while_stmt(stmt);
            }

            HIRStatementKind::Block(stmt) => {
                self.visit_block_stmt(stmt);
            }
        };
    }

    fn visit_return_stmt(&mut self, stmt: &HIRReturnStatement);
    fn visit_expression_stmt(&mut self, stmt: &HIRExpressionStatement) {
        self.visit_expr(&stmt.expression);
    }
    fn visit_variable_declaration_stmt(&mut self, stmt: &HIRVariableDeclarationStatement);
    fn visit_if_stmt(&mut self, stmt: &HIRIfStatement);
    fn visit_while_stmt(&mut self, stmt: &HIRWhileStatement);
    fn visit_block_stmt(&mut self, stmt: &HIRBlockStatement);

    fn visit_expr(&mut self, expr: &HIRExpression) {
        self.default_visit_expr(expr);
    }

    fn default_visit_expr(&mut self, expr: &HIRExpression) {
        match &expr.kind {
            HIRExpressionKind::Binary(expr) => {
                self.visit_binary_expr(expr);
            }
            HIRExpressionKind::Unary(expr) => {
                self.visit_unary_expr(expr);
            }
            HIRExpressionKind::Literal(expr) => {
                self.visit_literal_expr(expr);
            }
            HIRExpressionKind::Variable(expr) => {
                self.visit_variable_expr(expr);
            }
            HIRExpressionKind::Assignment(expr) => {
                self.visit_assignment_expr(expr);
            }
            HIRExpressionKind::Call(expr) => {
                self.visit_call_expr(expr);
            }
            HIRExpressionKind::MemberAccess(_) => {}
            HIRExpressionKind::Parenthesized(expr) => {
                self.visit_parenthesized_expr(expr);
            }
            HIRExpressionKind::Void => {
                self.visit_void_expr();
            }
            HIRExpressionKind::Ref(expr) => {
                self.visit_ref_expr(expr);
            }
            HIRExpressionKind::Deref(expr) => {
                self.visit_deref_expr(expr);
            }
        }
    }

    fn visit_ref_expr(&mut self, expr: &HIRRefExpression);
    fn visit_deref_expr(&mut self, expr: &HIRDerefExpression);
    fn visit_binary_expr(&mut self, expr: &HIRBinaryExpression);
    fn visit_unary_expr(&mut self, expr: &HIRUnaryExpression);
    fn visit_literal_expr(&mut self, expr: &HIRLiteralExpression);
    fn visit_variable_expr(&mut self, expr: &HIRVariableExpression);
    fn visit_assignment_expr(&mut self, expr: &HIRAssignmentExpression);
    fn visit_call_expr(&mut self, expr: &HIRCallExpression);
    fn visit_parenthesized_expr(&mut self, expr: &HIRParenthesizedExpression);
    fn visit_void_expr(&mut self);
}

