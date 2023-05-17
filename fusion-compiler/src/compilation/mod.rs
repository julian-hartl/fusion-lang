use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::ops::Deref;
use std::path::Path;
use std::rc::Rc;

use inkwell::targets::{InitializationConfig, Target};

use crate::ast::Ast;
use crate::ast::lexer::Lexer;
use crate::ast::parser::Parser;
// use crate::codegen::llvm::{LLVMCodegen, LLVMTypeBuilder};
use crate::diagnostics::{DiagnosticsBag, DiagnosticsBagCell};
use crate::diagnostics::printer::DiagnosticsPrinter;
use crate::formatting::Formatter;
use crate::hir::{HIR, HIRGen, Scope, ScopeCell};
use crate::mir::{MIR, MIRGen};
use crate::text;
use crate::text::SourceText;

pub struct CompilationUnit {
    pub ast: Ast,
    pub diagnostics_bag: DiagnosticsBagCell,
    pub hir: HIR,
    pub mir: MIR,
    pub scope: ScopeCell,
}

impl CompilationUnit {
    pub fn compile(source_text: &SourceText) -> Result<CompilationUnit, DiagnosticsBagCell> {
        let mut lexer = Lexer::new(source_text);
        let mut tokens = Vec::new();
        while let Some(token) = lexer.next_token() {
            tokens.push(token);
        }
        let diagnostics_bag: DiagnosticsBagCell = Rc::new(RefCell::new(DiagnosticsBag::new()));
        let mut ast = Ast::new();
        let mut parser = Parser::new(
            tokens,
            Rc::clone(&diagnostics_bag),
            &mut ast,
        );
        parser.parse();
        ast.visualize();

        Self::check_diagnostics(&source_text, &diagnostics_bag)?;
        let scope: Rc<RefCell<Scope>> = Rc::new(RefCell::new(Scope::new()));
        let hir_gen = HIRGen::new(Rc::clone(&diagnostics_bag),scope.clone());
        let hir = hir_gen.gen(&ast);
        hir.visualize(scope.clone());
        Self::check_diagnostics(&source_text, &diagnostics_bag).map_err(|_| Rc::clone(&diagnostics_bag))?;
        // if let Some(path) = &source_text.path {
        //     Self::format(&ast, &Path::new(path.as_str())).expect("Failed to format AST");
        // }
        let mir_gen = MIRGen::new(
            Rc::clone(&diagnostics_bag),
            scope.clone(),
        );
        let mir = mir_gen.construct(&hir);
        mir.output_graphviz(
            scope.borrow().deref(),
            "mir.dot",
        );
        mir.save_output(
            &scope.borrow(),
            "mir.txt",
        );
        Self::check_diagnostics(&source_text, &diagnostics_bag)?;
        Ok(CompilationUnit {
            ast,
            diagnostics_bag,
            hir,
            mir,
            scope,
        })
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
