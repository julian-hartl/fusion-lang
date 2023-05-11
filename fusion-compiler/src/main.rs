use inkwell::targets::{InitializationConfig, Target};

use compilation::CompilationUnit;

mod ast;
mod diagnostics;
mod text;
mod compilation;
mod typings;
mod codegen;
mod formatting;
mod mir;
mod hir;
mod interpreter;

fn main() -> Result<(), ()> {
    let input = std::env::args().nth(1).expect("No input file");
    let path = std::path::Path::new(&input);
    let source_text = text::io::read_source_text(&path).map_err(|_| ())?;
    let mut compilation_unit = CompilationUnit::compile(&source_text).map_err(|_| ())?;
    // let jit = &compilation_unit.jit;
    // let exit_code = unsafe { jit.call() };
    // println!("Exit code: {}", exit_code);
    // compilation_unit.run();
    Ok(())
}
