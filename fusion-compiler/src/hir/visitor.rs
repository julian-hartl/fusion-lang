use crate::hir::{FunctionIdx, HIR, HIRAssignmentExpression, HIRBinaryExpression, BinOperator, HIRBlockExpr, HIRCallExpression, HIRCastExpression, HIRDerefExpression, HIRExpr, HIRExprKind, HIRExpressionStatement, HIRIfExpr, HIRIndexExpression, HIRLiteralExpression, HIRParenthesizedExpression, HIRRefExpression, HIRReturnStatement, HIRStatement, HIRStatementKind, HIRStructInitExpression, HIRUnaryExpression, UnOperator, HIRVariableDeclarationStatement, HIRVariableExpression, HIRWhileExpr};

pub trait HIRVisitor {
    fn visit(&mut self, hir: &HIR) {
        for (id, statements) in hir.function_bodies.iter() {
            self.visit_function(id, statements);
        }
    }

    fn visit_function(&mut self, function_id: &FunctionIdx, statements: &Vec<HIRStatement>) {
        self.default_visit_function(function_id, statements);
    }

    fn default_visit_function(&mut self, function_id: &FunctionIdx, statements: &Vec<HIRStatement>) {
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
    fn visit_if_stmt(&mut self, stmt: &HIRIfExpr);
    fn visit_while_stmt(&mut self, stmt: &HIRWhileExpr);
    fn visit_block_stmt(&mut self, stmt: &HIRBlockExpr);

    fn visit_expr(&mut self, expr: &HIRExpr) {
        self.default_visit_expr(expr);
    }

    fn default_visit_expr(&mut self, expr: &HIRExpr) {
        match &expr.kind {
            HIRExprKind::Binary(expr) => {
                self.visit_binary_expr(expr);
            }
            HIRExprKind::Unary(expr) => {
                self.visit_unary_expr(expr);
            }
            HIRExprKind::Literal(expr) => {
                self.visit_literal_expr(expr);
            }
            HIRExprKind::Variable(expr) => {
                self.visit_variable_expr(expr);
            }
            HIRExprKind::Assignment(expr) => {
                self.visit_assignment_expr(expr);
            }
            HIRExprKind::Call(expr) => {
                self.visit_call_expr(expr);
            }
            HIRExprKind::FieldAccess(_) => {}
            HIRExprKind::Void => {
                self.visit_void_expr();
            }
            HIRExprKind::Ref(expr) => {
                self.visit_ref_expr(expr);
            }
            HIRExprKind::Deref(expr) => {
                self.visit_deref_expr(expr);
            }
            HIRExprKind::Cast(expr) => {
                self.visit_cast_expr(expr);
            }
            HIRExprKind::StructInit(expr) => {
                self.visit_struct_init_expr(expr);
            }
            HIRExprKind::Index(expr) => {
                self.visit_index_expr(expr);
            }
        }
    }

    fn visit_index_expr(&mut self, expr: &HIRIndexExpression);
    fn visit_struct_init_expr(&mut self, expr: &HIRStructInitExpression);
    fn visit_cast_expr(&mut self, expr: &HIRCastExpression);
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

