use crate::compilation;
use crate::ast::{Ast, ASTBooleanExpression, ASTClassField, ASTClassMember, ASTClassStatement, ASTExpression, ASTFuncDeclStatement, ASTIdentifierExpression, ASTLetStatement, ASTMemberAccessExpression, ASTNumberExpression, ASTSelfExpression, ASTStatement, ASTStringExpression, ASTUnaryExpression, FuncDeclParameter};
use crate::ast::lexer::TokenKind;
use crate::ast::visitor::ASTVisitor;
use crate::compilation::global_scope::GlobalScope;
use crate::compilation::symbols::class::Constructor;
use crate::compilation::symbols::function::{FunctionModifier, FunctionSymbol};
use crate::compilation::symbols::variable::VariableSymbol;
use crate::diagnostics::DiagnosticsBagCell;
use crate::text::span::TextSpan;
use crate::typings::Type;

pub struct GlobalSymbolResolver<'a> {
    diagnostics: DiagnosticsBagCell,
    pub global_scope: GlobalScope,
    ast: &'a Ast,
}

impl<'a> GlobalSymbolResolver<'a> {
    pub fn new(diagnostics: DiagnosticsBagCell, ast: &'a Ast) -> Self {
        GlobalSymbolResolver {
            diagnostics,
            global_scope: GlobalScope::new(),
            ast,
        }
    }
}

impl ASTVisitor for GlobalSymbolResolver<'_> {
    fn get_ast(&self) -> &Ast {
        self.ast
    }

    fn visit_class_statement(&mut self, class_statement: &ASTClassStatement, statement: &ASTStatement) {
        let name = class_statement.identifier.span.literal.clone();
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        fields.extend(class_statement.constructor.as_ref().map(|constructor| constructor.fields.iter().map(|field| self.map_class_field(field)).collect::<Vec<_>>()).unwrap_or_default());
        let constructor = class_statement.constructor.as_ref().map(|constructor| {
            Constructor {
                parameters: constructor.fields.iter().map(|parameter| VariableSymbol::new(
                    parameter.identifier.span.literal.clone(),
                    compilation::resolve_type(&self.diagnostics, &self.global_scope, &parameter.type_annotation.type_name),
                )).collect(),
            }
        });
        for member in &class_statement.body.members {
            match member {
                ASTClassMember::Field(field) => {
                    fields.push(self.map_class_field(field));
                }
                ASTClassMember::Method(method) => {
                    let func = self.ast.query_stmt(&method.func_decl).into_func_decl();
                    let (
                        parameters,
                        name,
                        return_type,
                        modifiers,
                    ) = self.map_function(Some(name.as_str()), &func);

                    methods.push(FunctionSymbol::new(
                        parameters, func.body,
                        return_type,
                        name,
                        modifiers,
                    ));
                }
                ASTClassMember::Invalid(_) => {}
            }
        }
        self.global_scope.declare_class(name.as_str(), fields, methods, constructor).unwrap_or_else(|_| {
            self.diagnostics.borrow_mut().report_class_already_declared(&class_statement.identifier);
        });
    }

    fn visit_func_decl_statement(&mut self, func_decl_statement: &ASTFuncDeclStatement) {
        let (parameters, name, return_type, modifiers) = self.map_function(None, func_decl_statement);
        match self.global_scope.declare_function(name.as_str(), func_decl_statement.body.as_ref(), parameters, return_type, modifiers) {
            Ok(_) => {}
            Err(_) => {
                self.diagnostics.borrow_mut().report_function_already_declared(&func_decl_statement.identifier);
            }
        }
    }
    fn visit_let_statement(&mut self, let_statement: &ASTLetStatement, statement: &ASTStatement) {}

    fn visit_self_expression(&mut self, self_expression: &ASTSelfExpression, expr: &ASTExpression) {}

    fn visit_member_access_expression(&mut self, member_access_expression: &ASTMemberAccessExpression, expr: &ASTExpression) {}

    fn visit_string_expression(&mut self, string_expression: &ASTStringExpression, expr: &ASTExpression) {}

    fn visit_identifier_expression(&mut self, variable_expression: &ASTIdentifierExpression, expr: &ASTExpression) {}

    fn visit_number_expression(&mut self, number: &ASTNumberExpression, expr: &ASTExpression) {}

    fn visit_boolean_expression(&mut self, boolean: &ASTBooleanExpression, expr: &ASTExpression) {}

    fn visit_error(&mut self, span: &TextSpan) {}

    fn visit_unary_expression(&mut self, unary_expression: &ASTUnaryExpression, expr: &ASTExpression) {}
}

impl GlobalSymbolResolver<'_> {
    fn map_class_field(&mut self, field: &ASTClassField) -> VariableSymbol {
        VariableSymbol::new(
            field.identifier.span.literal.clone(),
            compilation::resolve_type(&self.diagnostics, &self.global_scope, &field.type_annotation.type_name),
        )
    }

    fn map_function(&self, surrounding_class: Option<&str>, func_decl_statement: &ASTFuncDeclStatement) -> (Vec<VariableSymbol>, String, Type, Vec<FunctionModifier>) {
        let parameters = func_decl_statement.parameters.iter().enumerate().map(|(index, parameter)| {
            match parameter {
                FuncDeclParameter::Normal(parameter) => {
                    VariableSymbol::new(
                        parameter.identifier.span.literal.clone(),
                        compilation::resolve_type(&self.diagnostics, &self.global_scope,&parameter.type_annotation.type_name),
                    )
                }
                FuncDeclParameter::Self_(keyword) => {
                    if index != 0 {
                        self.diagnostics.borrow_mut().report_self_not_first_parameter(&keyword.span);
                    }
                    match surrounding_class {
                        Some(surrounding_class) => {
                            VariableSymbol::new(
                                keyword.span.literal.clone(),
                                Type::Class(surrounding_class.to_string()),
                            )
                        }
                        None => {
                            self.diagnostics.borrow_mut().report_self_outside_class(&keyword.span);
                            VariableSymbol::new(
                                keyword.span.literal.clone(),
                                Type::Error,
                            )
                        }
                    }
                }
            }
        }).collect();
        let literal_span = &func_decl_statement.identifier.span;
        let return_type = match &func_decl_statement.return_type {
            None => Type::Void,
            Some(return_type) => {
                compilation::resolve_type(&self.diagnostics, &self.global_scope,&return_type.type_name)
            }
        };
        let modifiers = func_decl_statement.modifier_tokens.iter().map(|modifier| {
            match &modifier.kind {
                TokenKind::External => Some(FunctionModifier::External),
                _ => {
                    self.diagnostics.borrow_mut().report_illegal_function_modifier(&modifier);
                    None
                }
            }
        }).filter_map(|x| x).collect();
        (parameters, literal_span.literal.clone(), return_type, modifiers)
    }
}
