use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::hash::Hash;
use std::io::Write;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::Result;

use crate::ast::Ast;
use crate::ast::lexer::Lexer;
use crate::ast::parser::{Parser, ParseResult};
use crate::diagnostics::{DiagnosticsBag, DiagnosticsBagCell};
use crate::diagnostics::printer::DiagnosticsPrinter;
use crate::formatting::Formatter;
use crate::hir::{HIR, HIRGen};
use crate::mir::{MIR, MIRGen};
use crate::modules::scopes::{GlobalScope, GlobalScopeCell};
use crate::modules::symbols::ModuleIdx;
use crate::perf::PerfMeasurement;
use crate::text::SourceText;

pub trait Parseable {
    type Error: std::error::Error + 'static + Send + Sync;
    fn get_content(&self) -> Result<String, Self::Error>;
    fn join(&self, path: &str) -> Self;
    fn describes_module(&self) -> bool;
    fn with_extension(&self, ext: &str) -> Self;
}

pub struct SourceTree {
    pub asts: HashMap<ModuleIdx, (Ast, SourceText)>,
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

    fn parse_ast<P>(&mut self, parseable: &P, id: ModuleIdx) -> Result<()> where P: Parseable {
        self.global_scope.borrow_mut().set_current_module(id);
        self.diagnostics_bag.borrow_mut().set_current_module(id);
        let text = parseable.get_content()?;
        let source_text = SourceText::new(text);
        let mut lexer = Lexer::new(&source_text);
        let mut token_stream = lexer.token_stream();
        let parser = Parser::new(
            token_stream,
            Rc::clone(&self.diagnostics_bag),
            Rc::clone(&self.global_scope),
        );
        let ParseResult { ast: root_ast, module_declarations: module_decls } = parser.parse();

        self.asts.insert(id, (root_ast, source_text));
        for mod_id in module_decls {
            self.global_scope.borrow_mut().set_current_module(id);
            self.diagnostics_bag.borrow_mut().set_current_module(id);
            let mod_name = &mod_id.span.literal;
            let mut mod_path = parseable.join(mod_name);
            if mod_path.describes_module() {
                // fallback to mod.fs
                mod_path = mod_path.join("mod.fs");
            } else {
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
        Ok(())
    }

    pub fn check_diagnostics(
        &self,
    ) -> Result<(), DiagnosticsBag> {
        let diagnostics_bag = self.diagnostics_bag.borrow();
        if diagnostics_bag.diagnostics.len() > 0 {
            self.print_diagnostics();
            Err(diagnostics_bag.clone())
        } else {
            Ok(())
        }
    }

    pub fn print_diagnostics(&self) {
        let diagnostics_bag = self.diagnostics_bag.borrow();
        let source_texts: HashMap<ModuleIdx, &SourceText> = self.asts.iter().map(|(id, (ast, source_text))| {
            (*id, source_text)
        }).collect();
        let printer = DiagnosticsPrinter::new(source_texts, &diagnostics_bag.diagnostics, self.global_scope.clone());
        printer.print();
    }

    pub fn visit<V>(&self, v: &mut V) where V: SourceTreeVisitor {
        for (id, (ast, _e)) in self.asts.iter() {
            v.visit_module(id, ast);
        }
    }
}

pub struct CompilationUnit {
    pub source_tree: SourceTree,
    pub diagnostics_bag: DiagnosticsBagCell,
    pub hir: HIR,
    pub mir: MIR,
    pub scope: GlobalScopeCell,
}

impl Parseable for PathBuf {
    type Error = std::io::Error;

    fn get_content(&self) -> Result<String, Self::Error> {
        std::fs::read_to_string(self)
    }

    fn join(&self, path: &str) -> Self {
        self.join(path)
    }

    fn describes_module(&self) -> bool {
        self.is_dir()
    }

    fn with_extension(&self, ext: &str) -> Self {
        self.with_extension(ext)
    }
}

impl CompilationUnit {
    pub fn compile<P>(input_file: &P) -> Result<CompilationUnit, DiagnosticsBag> where P: Parseable {
        let mut compilation_measurement = PerfMeasurement::new(String::from("Compilation"));
        compilation_measurement.start();
        let scope: Rc<RefCell<GlobalScope>> = Rc::new(RefCell::new(GlobalScope::new()));
        let scope_ref = scope.borrow();
        let root_module_id = scope_ref.root_module;
        let diagnostics_bag: DiagnosticsBagCell = Rc::new(RefCell::new(DiagnosticsBag::new(root_module_id)));
        let mut source_tree = SourceTree::new(diagnostics_bag.clone(), scope.clone());
        let modules = scope_ref.external_modules.clone();
        drop(scope_ref);
        for external_module in modules {
            let scope_ref = scope.borrow();
            let module = scope_ref.get_module(&external_module);
            let path = GlobalScope::get_external_modules_path().join(module.name.as_str()).join("lib.fs");
            drop(scope_ref);
            source_tree.parse_ast(&path, external_module).expect("Could not find external module");
        }
        source_tree.parse_ast(input_file, root_module_id).expect("Could not find root module");
        source_tree.check_diagnostics()?;
        let hir_gen = HIRGen::new(Rc::clone(&diagnostics_bag), scope.clone());
        let hir = hir_gen.gen(&source_tree);
        let scope_ref = scope.borrow();
        let infinite_size_check_result = scope_ref.check_structs_for_infinite_size();
        if let Err(struct_id) = infinite_size_check_result {
            let s = &scope_ref.get_struct(&struct_id);
            let decl_token = &s.decl_token;
            let decl_in_module = &s.decl_in_module;
            diagnostics_bag.borrow_mut().set_current_module(*decl_in_module);
            diagnostics_bag.borrow_mut().report_struct_has_infinite_size(decl_token);
        }
        drop(scope_ref);
        hir.visualize(scope.clone());
        source_tree.check_diagnostics()?;
        let mir_gen = MIRGen::new(
            Rc::clone(&diagnostics_bag),
            scope.clone(),
        );
        let mir = mir_gen.construct(&hir);
        compilation_measurement.end();
        println!("{}", compilation_measurement);
        mir.output_graphviz(
            scope.borrow().deref(),
            "mir.dot",
        );
        mir.save_output(
            &scope.borrow(),
            "mir.txt",
        );
        source_tree.check_diagnostics()?;
        Ok(CompilationUnit {
            source_tree,
            diagnostics_bag,
            hir,
            mir,
            scope,
        })
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

pub trait SourceTreeVisitor {
    fn visit_module(&mut self, module_id: &ModuleIdx, ast: &Ast);
}
