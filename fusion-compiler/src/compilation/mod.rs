use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::hash::Hash;
use std::io::Write;
use std::ops::Deref;
use std::path::Path;
use std::rc::Rc;

use crate::ast::Ast;
use crate::ast::lexer::Lexer;
use crate::ast::parser::Parser;
use crate::diagnostics::{DiagnosticsBag, DiagnosticsBagCell};
use crate::diagnostics::printer::DiagnosticsPrinter;
use crate::formatting::Formatter;
use crate::hir::{HIR, HIRGen};
use crate::mir::{MIR, MIRGen};
use crate::modules::scopes::{GlobalScope, GlobalScopeCell};
use crate::modules::symbols::ModuleId;
use crate::text;
use crate::text::SourceText;

pub struct SourceTree {
    pub asts: HashMap<ModuleId, Ast>,
    diagnostics_bag: DiagnosticsBagCell,
    global_scope: GlobalScopeCell,
}

impl SourceTree {
    pub fn new(
        diagnostics_bag: DiagnosticsBagCell,
        global_scope: GlobalScopeCell,
    ) -> Self {
        Self {
            asts: HashMap::new(),
            diagnostics_bag,
            global_scope,
        }
    }

    fn parse_ast(&mut self, path: &Path, id: ModuleId) -> fusion_compiler::Result<SourceText> {
        let text = std::fs::read_to_string(path).map_err(|_| ())?;
        let source_text = SourceText::new(text);
        let mut lexer = Lexer::new(&source_text);
        let mut tokens = Vec::new();
        while let Some(token) = lexer.next_token() {
            tokens.push(token);
        }
        let mut root_ast = Ast::new();
        let mut parser = Parser::new(
            tokens,
            Rc::clone(&self.diagnostics_bag),
            &mut root_ast,
        );
        parser.parse();
        Self::print_diagnostics(&source_text,&self.diagnostics_bag);
        let module_decls = parser.get_encountered_module_declarations().clone();
        drop(parser);

        self.asts.insert(id, root_ast);
        for mod_id in module_decls {
            let mod_name = &mod_id.span.literal;
            let mut mod_path = path.parent().unwrap().join(mod_name);
            if mod_path.is_dir() {
                // fallback to mod.fs
                mod_path = mod_path.join("mod.fs");
            }
            else {
                mod_path = mod_path.with_extension("fs");
            }
            let mut scope = self.global_scope.borrow_mut();
            let decl_module_result = scope.declare_module(mod_name.clone());
            drop(scope);
            match decl_module_result {
                Ok(id) => {
                    match self.parse_ast(&mod_path, id) {
                        Ok(_) => {}
                        Err(_) => {
                            self.diagnostics_bag.borrow_mut().report_could_not_open_module(&mod_id.span);
                        }
                    }
                }
                Err(_) => {
                    self.diagnostics_bag.borrow_mut().report_module_already_declared(&mod_id.span);
                }
            }
        }
        Ok(source_text)
    }

    pub fn print_diagnostics(source_text: &SourceText, diagnostics_bag: &DiagnosticsBagCell) {
        let diagnostics_bag = diagnostics_bag.borrow();
        let printer = DiagnosticsPrinter::new(source_text, &diagnostics_bag.diagnostics);
        printer.print();
    }

}

pub struct CompilationUnit {
    pub source_tree: SourceTree,
    pub diagnostics_bag: DiagnosticsBagCell,
    pub hir: HIR,
    pub mir: MIR,
    pub scope: GlobalScopeCell,
}

impl CompilationUnit {
    pub fn compile(input_file: &Path) -> Result<CompilationUnit, DiagnosticsBagCell> {
        let scope: Rc<RefCell<GlobalScope>> = Rc::new(RefCell::new(GlobalScope::new()));
        let diagnostics_bag: DiagnosticsBagCell = Rc::new(RefCell::new(DiagnosticsBag::new()));
        let mut source_tree = SourceTree::new(diagnostics_bag.clone(), scope.clone());
        let id = scope.borrow().current_module().id;
        let source_text = source_tree.parse_ast(input_file, id).expect("Could not find root module");

        Self::check_diagnostics(&source_text, &diagnostics_bag)?;
        let hir_gen = HIRGen::new(Rc::clone(&diagnostics_bag), scope.clone());
        let hir = hir_gen.gen(&source_tree);
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
            source_tree,
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
