use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::rc::Rc;
use std::sync::Mutex;

use inkwell::targets::{InitializationConfig, Target};
use lazy_static::lazy_static;

use global_scope::GlobalScope;

use crate::{diagnostics, optimization, text};
use crate::ast::Ast;
use crate::ast::evaluator::ASTEvaluator;
use crate::ast::lexer::{Lexer, Token};
use crate::ast::parser::Parser;
use crate::ast::visitor::ASTVisitor;
use crate::codegen::llvm::{LLVMCodegen, LLVMTypeBuilder};
use crate::compilation::global_symbol_resolver::GlobalSymbolResolver;
use crate::compilation::resolver::Resolver;
use crate::compilation::scopes::Scopes;
use crate::diagnostics::DiagnosticsBagCell;
use crate::diagnostics::printer::DiagnosticsPrinter;
use crate::formatting::Formatter;
use crate::ir::{IR, IRGen};
use crate::optimization::dead_code;
use crate::text::SourceText;
use crate::text::span::TextSpan;
use crate::typings::Type;

pub mod global_symbol_resolver;
pub mod resolver;
pub mod scopes;
pub mod symbols;
mod local_scope;
pub mod global_scope;


pub(crate) fn expect_type(diagnostics: &DiagnosticsBagCell, expected: Type, actual: &Type, span: &TextSpan) {
    if !actual.is_assignable_to(&expected) {
        diagnostics.borrow_mut().report_type_mismatch(span, &expected, actual);
    }
}

pub(crate) fn resolve_type(diagnostics: &DiagnosticsBagCell, global_scope: &GlobalScope, type_name: &Token) -> Type {
    let ty = Type::from_token_kind(&type_name.kind);
    let ty = ty.or(global_scope.lookup_type(&type_name.span.literal));
    let ty = match ty {
        None => {
            diagnostics.borrow_mut().report_undeclared_type(&type_name);
            Type::Error
        }
        Some(ty) => ty,
    };
    ty
}


pub struct CompilationUnit {
    pub ast: Ast,
    pub diagnostics_bag: DiagnosticsBagCell,
    pub global_scope: GlobalScope,
    pub ir: IR,
}

impl CompilationUnit {
    pub fn compile(source_text: &SourceText) -> Result<CompilationUnit, DiagnosticsBagCell> {
        let mut lexer = Lexer::new(source_text);
        let mut tokens = Vec::new();
        while let Some(token) = lexer.next_token() {
            tokens.push(token);
        }
        let diagnostics_bag: DiagnosticsBagCell = Rc::new(RefCell::new(diagnostics::DiagnosticsBag::new()));
        let mut ast = Ast::new();
        let mut parser = Parser::new(
            tokens,
            Rc::clone(&diagnostics_bag),
            &mut ast,
        );
        parser.parse();
        ast.visualize();

        Self::check_diagnostics(&source_text, &diagnostics_bag)?;

        let mut global_symbol_resolver = GlobalSymbolResolver::new(Rc::clone(&diagnostics_bag), &ast);
        ast.visit(&mut global_symbol_resolver);
        let global_scope = global_symbol_resolver.global_scope;
        let mut scopes = Scopes::from_global_scope(global_scope);
        let mut resolver = Resolver::new(Rc::clone(&diagnostics_bag), &mut scopes, &mut ast);
        resolver.resolve();
        Self::check_diagnostics(&source_text, &diagnostics_bag).map_err(|_| Rc::clone(&diagnostics_bag))?;
        if let Some(path) = &source_text.path {
            Self::format(&ast, &Path::new(path.as_str())).expect("Failed to format AST");
        }
        let ir_gen = IRGen::new(
            &source_text,
        );
        let mut ir = ir_gen.gen_ir(&mut ast, &mut scopes.global_scope);
        ir.save(
            "ir.txt"
        ).expect("Failed to save IR");
        Self::run_optimizations(&diagnostics_bag, &mut ast, &mut ir);
        ir.save(
            "ir-op.txt"
        ).expect("Failed to save IR");
        Self::check_diagnostics(&source_text, &diagnostics_bag)?;

        ir.save_graphviz(
            "graph.dot"
        ).expect("Failed to output graphviz");
        Target::initialize_aarch64(&InitializationConfig::default());

        let context = inkwell::context::Context::create();
        let type_builder = LLVMTypeBuilder::new(&context);
        let mut llvm_gen = LLVMCodegen::new(
            &context,
            type_builder,
        );

        llvm_gen.gen(
            &ir
        ).expect("Failed to generate code");
        llvm_gen.save_ir().expect("Failed to output IR");
        llvm_gen.save_asm().expect("Failed to output x86");
        llvm_gen.save_executable().expect("Failed to output executable");
        Ok(CompilationUnit {
            global_scope: scopes.global_scope,
            ast,
            diagnostics_bag,
            ir,
        })
    }

    fn run_optimizations(diagnostics_bag: &DiagnosticsBagCell, ast: &mut Ast, mut ir: &mut IR) {
        let variable_metadata = ir.get_variable_metadata();
        let mut constant_folding = optimization::constant_folding::ConstantFolding::new(
            &ast,
            diagnostics_bag.clone(),
            &variable_metadata,
        );
        constant_folding.fold(&mut ir);
        // todo: maybe modify directly in constant_folding to avoid second pass
        let variable_metadata = ir.get_variable_metadata();
        let mut dead_code_elim = dead_code::DeadCodeElimination::new(
            diagnostics_bag.clone(),
            &ast,
            &variable_metadata,
        );
        dead_code_elim.remove_and_report(
            &mut ir
        );
    }


    pub fn maybe_run(&self) {
        if self.diagnostics_bag.borrow().diagnostics.len() > 0 {
            return;
        }
        self.run();
    }

    pub fn run(&self) {
        let mut eval = ASTEvaluator::new(
            &self.global_scope,
            &self.ast,
        );
        let main_function = self.global_scope.lookup_function("main");
        if let Some(function) = main_function {
            eval.visit_statement(&function.body.unwrap());
        } else {
            self.ast.visit(&mut eval);
        }
        println!("Result: {:?}", eval.last_value);
    }


    pub fn check_diagnostics(text: &text::SourceText, diagnostics_bag: &DiagnosticsBagCell) -> Result<(), DiagnosticsBagCell> {
        let diagnostics_binding = diagnostics_bag.borrow();
        if diagnostics_binding.diagnostics.len() > 0 {
            let diagnostics_printer = DiagnosticsPrinter::new(
                &text,
                &diagnostics_binding.diagnostics,
            );
            diagnostics_printer.print();
            if diagnostics_binding.has_errors() {
                return Err(Rc::clone(diagnostics_bag));
            }
        }
        Ok(())
    }

    pub fn format(ast: &Ast, save_to: &Path) -> Result<(), std::io::Error> {
        let formatter = Formatter::new(&ast);
        let formatted = formatter.format();
        // remove all color codes from the output
        let formatted = formatted.replace("\x1b[0m", "").replace("\x1b[31m", "").replace("\x1b[32m", "");
        let mut file = File::create(save_to)?;
        file.write_all(formatted.as_bytes())?;
        Ok(())
    }
}
