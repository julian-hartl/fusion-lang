#[macro_use]
extern crate clap;

use clap::Parser;

use crate::cli::Command;

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
mod cli;
mod modules;
mod perf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = cli::Cli::parse();
    match &cli.command {
        Command::Build(cmd) => {
            cli.build(cmd)?;
        }
        Command::Run(cmd) => {
            cli.run(cmd)?;
        }
    }

    Ok(())
}
