use std::cell::RefCell;
use std::rc::Rc;

use crate::ast::lexer::{Token, TokenKind};
use crate::ast::QualifiedIdentifier;
use crate::modules::symbols::ModuleIdx;
use crate::text::span::{TextLocation, TextSpan};
use crate::typings::Type;

pub mod printer;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}


#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub message: String,
    pub location: TextLocation,
    pub severity: DiagnosticSeverity,
}

impl Diagnostic {
    pub fn new(message: String, location: TextLocation, kind: DiagnosticSeverity) -> Self {
        Diagnostic { message, location, severity: kind }
    }
}

pub type DiagnosticsBagCell = Rc<RefCell<DiagnosticsBag>>;

#[derive(Debug)]
pub struct DiagnosticsBag {
    pub diagnostics: Vec<Diagnostic>,
    current_module_id: ModuleIdx,
}

impl DiagnosticsBag {
    pub fn new(
        current_module_id: ModuleIdx,
    ) -> Self {
        DiagnosticsBag { diagnostics: vec![] , current_module_id }
    }


    pub fn set_current_module(&mut self, id: ModuleIdx) {
        self.current_module_id = id;
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == DiagnosticSeverity::Error)
    }

    pub fn report_error(&mut self, message: String, span: TextSpan) {
        let location = self.span_to_location(span);
        let error = Diagnostic::new(message, location, DiagnosticSeverity::Error);
        self.diagnostics.push(error);
    }

    pub fn report_warning(&mut self, message: String, span: TextSpan) {
        let location = self.span_to_location(span);
        let warning = Diagnostic::new(message, location, DiagnosticSeverity::Warning);
        self.diagnostics.push(warning);
    }

    fn span_to_location(&mut self, span: TextSpan) -> TextLocation {
        TextLocation { span, module_id: self.current_module_id }
    }

    pub fn report_unexpected_token(&mut self, expected: &TokenKind, token: &Token) {
        self.report_error(format!("Expected <{}>, found <{}>", expected, token.kind), token.span.clone());
    }
    pub fn report_unexpected_token_multiple(&mut self, expected: &[TokenKind], token: &Token) {
        let expected = expected.iter().map(|t| format!("<{}>", t)).collect::<Vec<_>>().join(", ");
        self.report_error(format!("Expected one of {}, found <{}>", expected, token.kind), token.span.clone());
    }
    pub fn report_expected_expression(&mut self, token: &Token) {
        self.report_error(format!("Expected expression, found <{}>", token.kind), token.span.clone());
    }

    pub fn report_undeclared_variable(&mut self, token: &Token) {
        self.report_error(format!("Undeclared variable '{}'", token.span.literal), token.span.clone());
    }

    pub fn report_undeclared_function(&mut self, token: &Token) {
        self.report_error(format!("Undeclared function '{}'", token.span.literal), token.span.clone());
    }

    pub fn report_invalid_argument_count(&mut self, callee_span: &TextSpan, expected: usize, actual: usize) {
        self.report_error(format!("Function expects {} arguments, but was given {}", expected, actual), callee_span.clone());
    }

    pub fn report_function_already_declared(&mut self, token: &Token) {
        self.report_error(format!("Function '{}' already declared", token.span.literal), token.span.clone());
    }

    pub fn report_type_mismatch(&mut self, span: &TextSpan, expected: &Type, actual: &Type) {
        self.report_error(format!("Expected type '{}', found '{}'", expected, actual), span.clone());
    }

    pub fn report_undeclared_type(&mut self, token: &Token) {
        self.report_error(format!("Undeclared type '{}'", token.span.literal), token.span.clone());
    }

    pub fn report_cannot_return_outside_function(&mut self, token: &Token) {
        self.report_error(format!("Cannot use 'return' outside of function"), token.span.clone());
    }

    pub fn report_unreachable_code(&mut self, span: &TextSpan) {
        self.report_warning(format!("Unreachable code"), span.clone());
    }

    pub fn report_illegal_function_modifier(&mut self, token: &Token) {
        self.report_error(format!("Illegal function modifier '{}'", token.span.literal), token.span.clone());
    }

    pub fn report_invalid_escape_sequence(&mut self, token: &Token) {
        self.report_error(format!("Invalid escape sequence '{}'", token.span.literal), token.span.clone());
    }

    pub fn report_missing_return(&mut self, span: &TextSpan) {
        self.report_error(format!("Missing return statement"), span.clone());
    }

    pub fn report_invalid_class_member(&mut self, span: &TextSpan) {
        self.report_error(format!("Invalid class member '{}'", span.literal), span.clone());
    }

    pub fn report_class_already_declared(&mut self, token: &Token) {
        self.report_error(format!("Class '{}' already declared", token.span.literal), token.span.clone());
    }

    pub fn report_expression_not_callable(&mut self, ty: &Type, span: &TextSpan) {
        self.report_error(format!("'{}' is not callable", ty), span.clone());
    }

    pub fn report_invalid_member_access(&mut self, span: &TextSpan, ty: &Type) {
        self.report_error(format!("Invalid member access on type '{}'", ty), span.clone());
    }

    pub fn report_member_not_found(&mut self, span: &TextSpan, ty: &Type) {
        self.report_error(format!("Member '{}' not found on type '{}'", span.literal, ty), span.clone());
    }

    pub fn report_self_outside_class(&mut self, span: &TextSpan) {
        self.report_error(format!("'self' can only be used inside a class"), span.clone());
    }

    pub fn report_self_not_declared(&mut self, span: &TextSpan) {
        self.report_error(format!("'self' not declared"), span.clone());
    }

    pub fn report_self_not_first_parameter(&mut self, span: &TextSpan) {
        self.report_error(format!("'self' must be the first parameter"), span.clone());
    }

    pub fn report_invalid_callee(&mut self, span: &TextSpan) {
        self.report_error(format!("Invalid callee"), span.clone());
    }

    pub fn report_invalid_assignment_target(&mut self, span: &TextSpan) {
        self.report_error(format!("Invalid assignment target"), span.clone());
    }

    pub fn report_binary_operator_mismatch(&mut self, span: &TextSpan, lhs: &Type, rhs: &Type) {
        self.report_error(format!("Binary operator '{}' not declared for types '{}' and '{}'", span.literal, lhs, rhs), span.clone());
    }

    pub fn report_unary_operator_mismatch(&mut self, span: &TextSpan, ty: &Type) {
        self.report_error(format!("Unary operator '{}' not declared for type '{}'", span.literal, ty), span.clone());
    }

    pub fn report_invalid_function_modifier(&mut self, span: &TextSpan) {
        self.report_error(format!("Invalid function modifier '{}'", span.literal), span.clone());
    }

    pub fn report_cannot_assign_to(&mut self, span: &TextSpan) {
        self.report_error(format!("Cannot assign to '{}'", span.literal), span.clone());
    }

    pub fn report_cannot_deref(&mut self, span: &TextSpan) {
        self.report_error(format!("Cannot dereference '{}'", span.literal), span.clone());
    }

    pub fn report_cannot_deref_void(&mut self, span: &TextSpan) {
        self.report_error(format!("Cannot dereference a void pointer"), span.clone());
    }

    pub fn report_invalid_character_literal(&mut self, span: &TextSpan) {
        self.report_error(format!("Invalid character literal '{}'", span.literal), span.clone());
    }

    pub fn report_cannot_assign_twice_to_immutable_variable(&mut self, span: &TextSpan) {
        self.report_error(format!("Cannot assign twice to immutable variable '{}'", span.literal), span.clone());
    }

    pub fn report_cannot_assign_to_immutable_pointer(&mut self, span: &TextSpan) {
        self.report_error(format!("Cannot assign to immutable pointer '{}'", span.literal), span.clone());
    }

    pub fn report_struct_already_declared(&mut self, token: &Token) {
        self.report_error(format!("Struct '{}' already declared", token.span.literal), token.span.clone());
    }

    pub fn report_struct_has_no_member(&mut self, span: &TextSpan, struct_name: &str) {
        self.report_error(format!("Struct '{}' has no member '{}'", struct_name, span.literal), span.clone());
    }

    pub fn report_cannot_access_member_of_non_struct(&mut self, span: &TextSpan, ty: &Type) {
        self.report_error(format!("Cannot access member '{}' of non-struct '{}'", span.literal, ty), span.clone());
    }

    pub fn report_cannot_assign_to_immutable_field(&mut self, span: &TextSpan) {
        self.report_error(format!("Cannot assign to immutable field '{}'", span.literal), span.clone());
    }

    pub fn report_cannot_access_non_ptr(&mut self, span: &TextSpan, ty: &Type) {
        self.report_error(format!("Cannot access non-pointer type '{}'", ty), span.clone());
    }

    pub fn report_undeclared_struct(&mut self, span: &TextSpan, name: &str) {
        self.report_error(format!("Undeclared struct '{}'", name), span.clone());
    }

    pub fn report_unexpected_qualified_identifier(&mut self, id: &QualifiedIdentifier) {
        self.report_error(format!("Unexpected qualified identifier '{}'", id), id.span());
    }

    pub fn report_could_not_open_module(&mut self, span: &TextSpan) {
        self.report_error(format!("Could not open module '{}'", span.literal), span.clone());
    }

    pub fn report_module_already_declared(&mut self, span: &TextSpan) {
        self.report_error(format!("Module '{}' already declared", span.literal), span.clone());
    }
    pub fn report_module_not_found(&mut self, span: &TextSpan) {
        self.report_error(format!("Module '{}' not found", span.literal), span.clone());
    }

    pub fn report_cannot_index_type(&mut self, span: &TextSpan, ty: &Type) {
        self.report_error(format!("Cannot index type '{}'", ty), span.clone());
    }

    pub fn report_struct_has_infinite_size(&mut self, decl_token: &Token) {
        self.report_error(format!("Struct '{}' has infinite size", decl_token.span.literal), decl_token.span.clone());
    }

    pub fn report_missing_field_in_struct(&mut self, span: &TextSpan, struct_name: &str, field_name: &str) {
        self.report_error(format!("Struct '{}' is missing field '{}'", struct_name, field_name), span.clone());
    }

    pub fn report_cannot_assign_to_immutable_index(&mut self, span: &TextSpan) {
        self.report_error(format!("Cannot assign to immutable index '{}'", span.literal), span.clone());
    }
}

#[cfg(test)]
mod test {
    use crate::compilation::CompilationUnit;
    use crate::diagnostics::{Diagnostic, DiagnosticSeverity};
    use crate::text::SourceText;
    use crate::text::span::TextSpan;

    struct DiagnosticsVerifier {
        actual: Vec<Diagnostic>,
        expected: Vec<Diagnostic>,
    }

    impl DiagnosticsVerifier {
        pub fn new(input: &str, messages: Vec<&str>) -> Self {
            let messages_len = messages.len();
            let expected = Self::parse_input(input, messages);
            assert_eq!(expected.len(), messages_len);
            let actual = Self::compile(input);
            Self { expected, actual }
        }

        fn compile(input: &str) -> Vec<Diagnostic> {
            let raw = Self::get_raw_text(input);
            let source_text = SourceText::new(&raw, None);
            let compilation_unit = CompilationUnit::compile(&source_text);
            match compilation_unit {
                Ok(_) => vec![],
                Err(e) => e.borrow().diagnostics.clone(),
            }
        }

        fn get_raw_text(input: &str) -> String {
            input.replace("«", "").replace("»", "")
        }

        fn parse_input(input: &str, messages: Vec<&str>) -> Vec<Diagnostic> {
            let raw_text = Self::get_raw_text(input);
            let mut start_index_stack = vec![];

            let mut current_position: usize = 0;
            let mut diagnostics = vec![];
            for c in input.chars() {
                match c {
                    '«' => {
                        start_index_stack.push(current_position);
                    }
                    '»' => {
                        let start_index = start_index_stack.pop().unwrap();
                        let end_index = current_position;
                        let literal = &raw_text[start_index..end_index];
                        let span = TextSpan::new(start_index, end_index, literal.to_string());
                        let message = messages[diagnostics.len()].to_string();
                        let diagnostic = Diagnostic::new(message, span, DiagnosticSeverity::Error);
                        diagnostics.push(diagnostic);
                    }
                    _ => {
                        current_position += 1;
                    }
                };
            }

            diagnostics
        }

        fn verify(&self) {
            assert_eq!(self.actual.len(), self.expected.len(), "Expected {} diagnostics, found {}", self.expected.len(), self.actual.len());

            for (actual, expected) in self.actual.iter().zip(self.expected.iter()) {
                assert_eq!(actual.message, expected.message, "Expected message '{}', found '{}'", expected.message, actual.message);
                assert_eq!(actual.location.start, expected.location.start, "Expected start index {}, found {}", expected.location.start, actual.location.start);
                assert_eq!(actual.location.end, expected.location.end, "Expected end index {}, found {}", expected.location.end, actual.location.end);
                assert_eq!(actual.location.literal, expected.location.literal, "Expected literal '{}', found '{}'", expected.location.literal, actual.location.literal);
            }
        }
    }

    fn assert_diagnostics(input: &str, expected: Vec<&str>) {
        let verifier = DiagnosticsVerifier::new(input, expected);
        verifier.verify();
    }

    #[test]
    fn should_report_undeclared_variable() {
        let input = "let a = «b»";
        let expected = vec![
            "Undeclared variable 'b'"
        ];

        assert_diagnostics(input, expected);
    }


    #[test]
    fn should_report_expected_expression() {
        let input = "let a = «+»";
        let expected = vec![
            "Expected expression, found <+>"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    fn should_report_bad_token() {
        let input = "let a = 8 «@» 2";
        let expected = vec![
            "Expected expression, found <Bad>"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    fn should_report_undeclared_variable_when_variable_was_declared_in_another_scope() {
        let input = "\
        let a = 0
        let b = -1
        if b > a {
            a = 10
           b = 2
            let c = 10
        }
         else
            a = 5
        a
b
«c»
    ";
        let expected = vec![
            "Undeclared variable 'c'"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    fn should_not_report_any_errors_when_shadowing_variable() {
        let input = "\
        let a = 0
        {
            let a = 10
        }
    ";
        let expected = vec![];

        assert_diagnostics(input, expected);
    }

    #[test]
    fn should_report_undeclared_variable_when_variable_was_declared_in_if_without_block() {
        let input = "\
        let b = -1
        if b > 10
            let a = 10
        «a»
    ";
        let expected = vec![
            "Undeclared variable 'a'"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    fn should_report_function_already_declared() {
        let input = "\
        func a {}
        func «a» {}
    ";

        let expected = vec![
            "Function 'a' already declared"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    fn should_report_error_when_calling_undeclared_function() {
        let input = "\
        «a»()
    ";

        let expected = vec![
            "Undeclared function 'a'"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    pub fn should_report_error_when_function_is_called_with_wrong_number_of_arguments() {
        let input = "\
        func a(a: int, b: int) {}
        «a»(1)
    ";

        let expected = vec![
            "Function 'a' expects 2 arguments, but was given 1"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    pub fn should_report_type_mismatch_when_int_is_used_in_if_condition() {
        let input = "\
        if «1» {
            let a = 10
        }
    ";

        let expected = vec![
            "Expected type 'bool', found 'int'"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    pub fn should_report_type_mismatch_when_variable_of_type_int_is_used_in_if_condition() {
        let input = "\
        let a = 1
        if «a» {
            let a = 10
        }
    ";

        let expected = vec![
            "Expected type 'bool', found 'int'"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    pub fn should_report_type_mismatch_when_binary_expression_of_type_int_is_used_in_if_condition() {
        let input = "\
        let a = 1
        if «a + 1» {
            let a = 10
        }
    ";

        let expected = vec![
            "Expected type 'bool', found 'int'"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    pub fn should_report_type_mismatch_when_adding_int_with_bool() {
        let input = "\
        let a = 1
        a + «true»
    ";

        let expected = vec![
            "Expected type 'int', found 'bool'"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    pub fn should_report_type_mismatch_when_using_minus_unary_operator_on_bool() {
        let input = "\
        let a = true
        -«a»
    ";

        let expected = vec![
            "Expected type 'int', found 'bool'"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    pub fn should_report_tm_when_assigning_function_call_result_to_variable_of_another_type() {
        let input = "\
        let b = false
        b = «a()»
        func a -> int {
            return 1
        }
        ";

        let expected = vec![
            "Expected type 'bool', found 'int'"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    pub fn should_report_tm_when_using_binop_on_incompatible_types_in_function_params() {
        let input = "\
        func a(a: int, b: bool) {
            a + «b»
        }
        ";

        let expected = vec![
            "Expected type 'int', found 'bool'"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    pub fn should_report_wrong_return_type_when_function_returns_incompatible_type() {
        let input = "\
        func a -> int {
            return «true»
        }
        ";

        let expected = vec![
            "Expected type 'int', found 'bool'"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    pub fn should_report_tm_when_assigning_wrong_type_to_variable_with_static_type() {
        let input = "\
        let a: int = «true»
        ";

        let expected = vec![
            "Expected type 'int', found 'bool'"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    pub fn should_report_cannot_use_return_outside_of_function() {
        let input = "\
        «return» 2
        ";

        let expected = vec![
            "Cannot use 'return' outside of function"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    pub fn should_not_allow_addition_with_void_type() {
        let input = "\
        let a = 1
        a + «a()»
        func a {}
        ";

        let expected = vec![
            "Expected type 'int', found 'void'"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    pub fn should_report_undeclared_type_in_let_assignment() {
        let input = "\
        let a: «b» = 1
        ";

        let expected = vec![
            "Undeclared type 'b'"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    pub fn should_report_undeclared_type_in_function_return_type() {
        let input = "\
        func a -> «b» {}
        ";

        let expected = vec![
            "Undeclared type 'b'"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    pub fn should_report_undeclared_type_in_function_param_type() {
        let input = "\
        func a(a: «b») {}
        ";

        let expected = vec![
            "Undeclared type 'b'"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    pub fn should_report_tm_in_arguments_in_function_call() {
        let input = "\
        func a(a: int) {}
        a(«true»)
        ";

        let expected = vec![
            "Expected type 'int', found 'bool'"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    pub fn should_report_error_when_returning_value_from_void_function() {
        let input = "\
        func a() {
            return «1»
        }
        ";

        let expected = vec![
            "Expected type 'void', found 'int'"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    pub fn should_report_error_when_using_return_without_expression_in_non_void_function() {
        let input = "\
        func a() -> int {
            «return»
        }
        ";

        let expected = vec![
            "Expected type 'int', found 'void'"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    pub fn should_report_error_when_assigning_to_undeclared_variable() {
        let input = "\
        «a» = 1
        ";

        let expected = vec![
            "Undeclared variable 'a'"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    pub fn should_not_allow_non_bool_types_in_while_condition() {
        let input = "\
        let a = add(1, 2)
        func add(a: int, b: int) -> int {
            return a + b
        }
        while «a + 1» {
            a = a + 1
        }

    ";

        let expected = vec![
            "Expected type 'bool', found 'int'"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    pub fn should_report_dead_code_when_using_return_in_if() {
        let input = "\
        func a() -> int {
            if true {
                return 1
            }
            «return 2»
        }
        ";

        let expected = vec![
            "Unreachable code"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    pub fn should_report_unused_function() {
        let input = "\
        func «b»() {}
        ";

        let expected = vec![
            "Unused function 'b'"
        ];

        assert_diagnostics(input, expected);
    }

    #[test]
    pub fn should_report_missing_return_in_not_void_function() {
        let input = "\
        func a() -> int {
            «if true {
                return 1
            }»
        }
        ";

        let expected = vec![
            "Missing return statement"
        ];

        assert_diagnostics(input, expected);
    }
}

