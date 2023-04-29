use crate::ast::{Ast, ASTAssignmentExpression, ASTBinaryExpression, ASTBinaryOperatorKind, ASTBlockStatement, ASTBooleanExpression, ASTCallExpression, ASTClassMember, ASTClassStatement, ASTExpression, ASTFuncDeclStatement, ASTIdentifierExpression, ASTIdentifierKind, ASTIfStatement, ASTLetStatement, ASTMemberAccessExpression, ASTNumberExpression, ASTParenthesizedExpression, ASTReturnStatement, ASTSelfExpression, ASTStatement, ASTStmtId, ASTStringExpression, ASTUnaryExpression, ASTUnaryOperatorKind, ASTWhileStatement};
use crate::ast::visitor::ASTVisitor;
use crate::compilation::scopes::Scopes;
use crate::diagnostics::DiagnosticsBagCell;
use crate::text::span::TextSpan;
use crate::typings::{FunctionType, Type};
use crate::compilation;
pub struct Resolver<'a> {
    scopes: &'a mut Scopes,
    diagnostics: DiagnosticsBagCell,
    ast: &'a mut Ast,
}

impl<'a> Resolver<'a> {
    pub fn new(diagnostics: DiagnosticsBagCell, scopes: &'a mut Scopes, ast: &'a mut Ast) -> Self {
        Resolver {
            scopes,
            diagnostics,
            ast,
        }
    }


    pub fn resolve(&mut self) {
        let stmt_ids: Vec<ASTStmtId> = self.ast.top_level_statements.iter().map(|stmt| stmt.clone()).collect();
        for stmt_id in stmt_ids {
            self.visit_statement(&stmt_id);
        }
    }

    pub fn resolve_binary_expression(
        &self,
        left: &ASTExpression,
        right: &ASTExpression,
        operator: &ASTBinaryOperatorKind,
    ) -> Type {
        let matrix: (Type, Type, Type) = match operator {
            ASTBinaryOperatorKind::Plus => (Type::I64, Type::I64, Type::I64),
            ASTBinaryOperatorKind::Minus => (Type::I64, Type::I64, Type::I64),
            ASTBinaryOperatorKind::Multiply => (Type::I64, Type::I64, Type::I64),
            ASTBinaryOperatorKind::Divide => (Type::I64, Type::I64, Type::I64),
            ASTBinaryOperatorKind::Power => (Type::I64, Type::I64, Type::I64),
            ASTBinaryOperatorKind::BitwiseAnd => (Type::I64, Type::I64, Type::I64),
            ASTBinaryOperatorKind::BitwiseOr => (Type::I64, Type::I64, Type::I64),
            ASTBinaryOperatorKind::BitwiseXor => (Type::I64, Type::I64, Type::I64),
            ASTBinaryOperatorKind::Equals => (Type::I64, Type::I64, Type::Bool),
            ASTBinaryOperatorKind::NotEquals => (Type::I64, Type::I64, Type::Bool),
            ASTBinaryOperatorKind::LessThan => (Type::I64, Type::I64, Type::Bool),
            ASTBinaryOperatorKind::LessThanOrEqual => (Type::I64, Type::I64, Type::Bool),
            ASTBinaryOperatorKind::GreaterThan => (Type::I64, Type::I64, Type::Bool),
            ASTBinaryOperatorKind::GreaterThanOrEqual => (Type::I64, Type::I64, Type::Bool),
        };

        self.expect_type(matrix.0, &left.ty, &left.span(&self.ast));

        self.expect_type(matrix.1, &right.ty, &right.span(&self.ast));

        matrix.2
    }

    fn expect_type(&self, expected: Type, actual: &Type, span: &TextSpan) {
        compilation::expect_type(&self.diagnostics, expected, actual, span)
    }


    pub fn resolve_unary_expression(&self, operand: &ASTExpression, operator: &ASTUnaryOperatorKind) -> Type {
        let matrix: (Type, Type) = match operator {
            ASTUnaryOperatorKind::Minus => (Type::I64, Type::I64),
            ASTUnaryOperatorKind::BitwiseNot => (Type::I64, Type::I64),
        };

        self.expect_type(matrix.0, &operand.ty, &operand.span(&self.ast));

        matrix.1
    }
}

impl ASTVisitor for Resolver<'_> {
    fn get_ast(&self) -> &Ast {
        self.ast
    }

    fn visit_class_statement(&mut self, class_statement: &ASTClassStatement, statement: &ASTStatement) {
        let class = &self.scopes.global_scope.lookup_class(&class_statement.identifier.span.literal).unwrap().clone();
        self.scopes.enter_class_scope(class.clone());
        for member in &class_statement.body.members {
            match member {
                ASTClassMember::Field(_) => {}
                ASTClassMember::Invalid(_) => {}
                ASTClassMember::Method(method) => {
                    self.visit_func_decl_statement(&self.ast.query_stmt(&method.func_decl).into_func_decl().clone());
                }
            }
        }
    }

    fn visit_func_decl_statement(&mut self, func_decl_statement: &ASTFuncDeclStatement) {
        if let Some(body) = &func_decl_statement.body {
            let function_symbol = self.scopes.lookup_function(&func_decl_statement.identifier.span.literal).unwrap().clone();
            self.scopes.enter_function_scope(
                function_symbol.clone(),
            );
            for parameter in &function_symbol.parameters {
                self.scopes.declare_parameter(parameter);
            }
            self.visit_statement(body);
            self.scopes.exit_scope();
        }
    }

    fn visit_return_statement(&mut self, return_statement: &ASTReturnStatement, stmt: &ASTStatement) {
        let return_keyword = return_statement.return_keyword.clone();
        // todo: do not clone
        match self.scopes.surrounding_function().map(|function| function.clone()) {
            None => {
                let mut diagnostics_binding = self.diagnostics.borrow_mut();
                diagnostics_binding.report_cannot_return_outside_function(&return_statement.return_keyword);
            }
            Some(function) => {
                let is_top_level = self.scopes.local_scopes.len() == 1;
                self.ast.set_top_level_on_return(&stmt.id, is_top_level);
                if let Some(return_expression) = &return_statement.return_value {
                    self.visit_expression(return_expression);
                    let return_expression = self.ast.query_expr(return_expression);
                    self.expect_type(function.return_type.clone(), &return_expression.ty, &return_expression.span(&self.ast));
                } else {
                    self.expect_type(Type::Void, &function.return_type, &return_keyword.span);
                }
            }
        }
    }

    fn visit_while_statement(&mut self, while_statement: &ASTWhileStatement) {
        self.visit_expression(&while_statement.condition);
        let condition = self.ast.query_expr(&while_statement.condition);
        self.expect_type(Type::Bool, &condition.ty, &condition.span(&self.ast));
        self.visit_statement(&while_statement.body);
    }

    fn visit_block_statement(&mut self, block_statement: &ASTBlockStatement) {
        let enter_scope = self.scopes.current_local_scope().map(|local| local.function.is_none()).unwrap_or(false);
        if enter_scope {
            self.scopes.enter_nested_scope();
        }
        for statement in &block_statement.statements {
            self.visit_statement(statement);
        }
        if enter_scope {
            self.scopes.exit_scope();
        }
    }

    fn visit_if_statement(&mut self, if_statement: &ASTIfStatement) {
        self.scopes.enter_nested_scope();
        self.visit_expression(&if_statement.condition);
        let condition_expression = self.ast.query_expr(&if_statement.condition);
        self.expect_type(Type::Bool, &condition_expression.ty, &condition_expression.span(&self.ast));
        self.visit_statement(&if_statement.then_branch);
        self.scopes.exit_scope();
        if let Some(else_branch) = &if_statement.else_branch {
            self.scopes.enter_nested_scope();
            self.visit_statement(&else_branch.else_statement);
            self.scopes.exit_scope();
        }
    }

    fn visit_let_statement(&mut self, let_statement: &ASTLetStatement, statement: &ASTStatement) {
        let identifier = let_statement.identifier.span.literal.clone();
        self.visit_expression(&let_statement.initializer);
        let initializer_expression = self.ast.query_expr(&let_statement.initializer);
        let ty = match &let_statement.type_annotation {
            Some(type_annotation) => {
                let ty = compilation::resolve_type(&self.diagnostics, &self.scopes.global_scope,&type_annotation.type_name);
                self.expect_type(ty.clone(), &initializer_expression.ty, &initializer_expression.span(&self.ast));
                ty
            }
            None => {
                initializer_expression.ty.clone()
            }
        };
        let variable = self.scopes.declare_variable(&identifier, ty);
        self.ast.set_symbol_for_stmt(&statement.id, variable.clone());
    }

    fn visit_self_expression(&mut self, self_expression: &ASTSelfExpression, expr: &ASTExpression) {
        let surrounding_function = self.scopes.surrounding_function();
        if let Some(function) = surrounding_function {
            let self_symbol = function.parameters.iter().find(
                |parameter| parameter.name == "self"
            );
            let ty = match self_symbol {
                Some(self_symbol) => {
                    self_symbol.ty.clone()
                }
                None => {
                    let mut diagnostics_binding = self.diagnostics.borrow_mut();
                    diagnostics_binding.report_self_not_declared(&self_expression.self_keyword.span);
                    Type::Error
                }
            };
            self.ast.set_type(&expr.id, ty);
        }
    }

    fn visit_member_access_expression(&mut self, member_access_expression: &ASTMemberAccessExpression, expr: &ASTExpression) {
        self.visit_expression(&member_access_expression.object);
        let object = self.ast.query_expr(&member_access_expression.object);
        let member = member_access_expression.target.span.literal.clone();
        let ty = match &object.ty {
            Type::Class(class) => {
                let (field, method) = self.scopes.global_scope.lookup_class_member(class.as_str(), member.as_str());
                match (field, method) {
                    (Some(field), None) => {
                        // self.ast.set_symbol_for_expr(&expr.id, field.clone());
                        field.ty.clone()
                    }
                    (None, Some(method)) => {
                        Type::Function(FunctionType::from(method))
                    }
                    (None, None) => {
                        let mut diagnostics_binding = self.diagnostics.borrow_mut();
                        diagnostics_binding.report_member_not_found(&member_access_expression.target.span, &object.ty);
                        Type::Error
                    }
                    (Some(_), Some(_)) => {
                        // todo: handle this case properly
                        let mut diagnostics_binding = self.diagnostics.borrow_mut();
                        if object.ty != Type::Error {
                            diagnostics_binding.report_member_not_found(&member_access_expression.target.span, &object.ty);
                        }
                        Type::Error
                    }
                }
            }
            _ => {
                let mut diagnostics_binding = self.diagnostics.borrow_mut();
                if object.ty != Type::Error {
                    diagnostics_binding.report_member_not_found(&member_access_expression.target.span, &object.ty);
                }
                Type::Error
            }
        };
        self.ast.set_type(&expr.id, ty);
    }

    fn visit_string_expression(&mut self, string_expression: &ASTStringExpression, expr: &ASTExpression) {
        self.ast.set_type(&expr.id, Type::Str);
    }

    fn visit_call_expression(&mut self, call_expression: &ASTCallExpression, expr: &ASTExpression) {
        // todo: do not clone
        self.visit_expression(&call_expression.callee);
        let callee_ty = &self.ast.query_expr(&call_expression.callee).ty.clone();
        let ty = match callee_ty {
            Type::Function(FunctionType { parameters, return_type, .. }) => {
                if parameters.len() != call_expression.arguments.len() {
                    let mut diagnostics_binding = self.diagnostics.borrow_mut();
                    diagnostics_binding.report_invalid_argument_count(
                        &self.ast.span(&call_expression.callee.into()),
                        parameters.len(),
                        call_expression.arguments.len(),
                    );
                }
                let return_type = return_type.clone();
                for (argument, param) in call_expression.arguments.iter().zip(parameters.iter()) {
                    self.visit_expression(argument);
                    let argument_expression = self.ast.query_expr(argument);
                    self.expect_type(
                        param.clone(),
                        &argument_expression.ty,
                        &argument_expression.span(&self.ast),
                    );
                }
                *return_type
            }
            Type::Class(name) => {
                let class= self.scopes.global_scope.lookup_class(name.as_str()).unwrap();
                // todo: do not clone
                let constructor = class.constructor.clone();
                let expected_length = constructor.as_ref().map(|constructor| constructor.parameters.len()).unwrap_or(0);
                if expected_length != call_expression.arguments.len() {
                    let mut diagnostics_binding = self.diagnostics.borrow_mut();
                    diagnostics_binding.report_invalid_argument_count(
                        &self.ast.span(&call_expression.callee.into()),
                        expected_length,
                        call_expression.arguments.len(),
                    );
                }
                let rt_type = callee_ty.clone();
                for (argument, param) in call_expression.arguments.iter().zip(constructor.iter().flat_map(|constructor| constructor.parameters.iter())) {
                    self.visit_expression(argument);
                    let argument_expression = self.ast.query_expr(argument);
                    self.expect_type(
                        param.ty.clone(),
                        &argument_expression.ty,
                        &argument_expression.span(&self.ast),
                    );
                }
                rt_type
            }
            _ => {
                let mut diagnostics_binding = self.diagnostics.borrow_mut();
                diagnostics_binding.report_expression_not_callable(&callee_ty, &self.ast.span(&call_expression.callee.into()));
                Type::Error
            }
        };
        self.ast.set_type(&expr.id, ty);
    }

    fn visit_assignment_expression(&mut self, assignment_expression: &ASTAssignmentExpression, expr: &ASTExpression) {
        self.visit_expression(&assignment_expression.expression);
        let value_expression = self.ast.query_expr(&assignment_expression.expression);
        let identifier = assignment_expression.identifier.span.literal.clone();
        let ty = match self.scopes.lookup_variable(&identifier) {
            None => {
                let mut diagnostics_binding = self.diagnostics.borrow_mut();
                diagnostics_binding.report_undeclared_variable(&assignment_expression.identifier);
                Type::Void
            }
            Some(variable) => {
                self.expect_type(variable.ty.clone(), &value_expression.ty, &value_expression.span(&self.ast));
                self.ast.set_symbol_for_expr(&expr.id, variable.clone());
                variable.ty.clone()
            }
        };
        self.ast.set_type(&expr.id, ty);
    }

    fn visit_identifier_expression(&mut self, identifier_expression: &ASTIdentifierExpression, expr: &ASTExpression) {
        let identifier = &identifier_expression.identifier.span.literal;
        match self.scopes.lookup_variable(&identifier) {
            None => {
                match self.scopes.lookup_function(&identifier) {
                    None => {
                        match self.scopes.global_scope.lookup_class(&identifier) {
                            None => {
                                let mut diagnostics_binding = self.diagnostics.borrow_mut();
                                diagnostics_binding.report_undeclared_variable(
                                    &identifier_expression.identifier,
                                );
                            }
                            Some(class) => {
                                self.ast.set_type(&expr.id, Type::Class(class.name.clone()));
                                self.ast.set_identifier_kind(&expr.id, ASTIdentifierKind::Class(class.clone()));
                            }
                        }
                    }
                    Some(function) => {
                        self.ast.set_type(&expr.id, Type::Function(function.into()));
                        self.ast.set_identifier_kind(&expr.id, ASTIdentifierKind::Function(function.clone()));
                    }
                }
            }
            Some(variable) => {
                self.ast.set_type(&expr.id, variable.ty.clone());
                self.ast.set_identifier_kind(&expr.id, ASTIdentifierKind::Variable(variable.clone()));
            }
        };
    }

    fn visit_number_expression(&mut self, number: &ASTNumberExpression, expr: &ASTExpression) {
        self.ast.set_type(&expr.id, Type::I64);
    }

    fn visit_boolean_expression(&mut self, boolean: &ASTBooleanExpression, expr: &ASTExpression) {
        self.ast.set_type(&expr.id, Type::Bool);
    }

    fn visit_error(&mut self, span: &TextSpan) {}

    fn visit_unary_expression(&mut self, unary_expression: &ASTUnaryExpression, expr: &ASTExpression) {
        self.visit_expression(&unary_expression.operand);
        let operand = self.ast.query_expr(&unary_expression.operand);
        let ty = self.resolve_unary_expression(&operand, &unary_expression.operator.kind);
        self.ast.set_type(&expr.id, ty);
    }

    fn visit_binary_expression(&mut self, binary_expression: &ASTBinaryExpression, expr: &ASTExpression) {
        self.visit_expression(&binary_expression.left);
        self.visit_expression(&binary_expression.right);
        let left = self.ast.query_expr(&binary_expression.left);
        let right = self.ast.query_expr(&binary_expression.right);

        let ty = self.resolve_binary_expression(&left, &right, &binary_expression.operator.kind);
        self.ast.set_type(&expr.id, ty);
    }

    fn visit_parenthesized_expression(&mut self, parenthesized_expression: &ASTParenthesizedExpression, expr: &ASTExpression) {
        self.visit_expression(&parenthesized_expression.expression);

        let expression = self.ast.query_expr(&parenthesized_expression.expression);

        self.ast.set_type(&expr.id, expression.ty.clone());
    }
}
