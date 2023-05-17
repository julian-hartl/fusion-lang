use std::ffi::OsStr;
use clap::Parser;
use std::fs::File;
use std::io::{Error, ErrorKind, Write};
use std::path::{Path, PathBuf};
use crate::compilation::CompilationUnit;
use crate::text;


#[derive(Parser)]
#[clap(version = "1.0", author = "Julian Hartl")]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    #[clap(name = "build")]
    Build(BuildCommand),
    #[clap(name = "run")]
    Run(BuildCommand),
}

#[derive(Args)]
pub struct BuildCommand {
    /// Input file
    #[clap()]
    pub input: PathBuf,

    /// Output file
    #[clap(short = 'o', long = "output", default_value = "out")]
    pub output: PathBuf,
}

impl Cli {

    pub fn run(&self, build: &BuildCommand) -> Result<(), Box<dyn std::error::Error>> {
        self.build(build)?;
        self.execute(build)?;
        Ok(())
    }

    pub fn build(&self, build: &BuildCommand) -> Result<(), Box<dyn std::error::Error>> {
        self.gen_asm(&build)?;
        self.gen_object(&build)?;
        self.link(&build)?;
        Ok(())
    }

    fn gen_object(&self, build: &BuildCommand) -> Result<(), Box<dyn std::error::Error>> {
        // Create a temporary path for the output object file
        let output_file = Self::get_object_file(build);
        let input_files = [Self::get_asm_file(build), PathBuf::from("std/linux/syscalls.s")];

        // Execute the 'as' command to create the object file
        let output = std::process::Command::new("as")
            .arg("-o").arg(&output_file)
            .args(&input_files)
            .output()
            .map_err(|e| format!("Failed to execute 'as' command: {}", e))?;

        // Check if the 'as' command succeeded
        if !output.status.success() {
            let error_message = String::from_utf8_lossy(&output.stderr).into_owned();
            return Err(format!("Failed to create object file: {}", error_message).into());
        }
        Ok(())
    }

    fn link(&self, build: &BuildCommand) -> Result<(), Box<dyn std::error::Error>> {
        let object_file = Self::get_object_file(build);
        let output_file = &build.output;

        Self::link_object_files(&[object_file], output_file).map_err(|e| format!("Failed to link object files: {}", e))?;
        Ok(())
    }


    fn link_object_files(object_files: &[PathBuf], output_file: &Path) -> Result<(), String> {
        // Execute the 'ld' command to link the object files
        let mut command = std::process::Command::new("ld");
        command.arg("-o").arg(output_file);

        for object_file in object_files {
            command.arg(object_file);
        }

        let output = command.output().map_err(|e| format!("Failed to execute 'ld' command: {}", e))?;

        // Check if the 'ld' command succeeded
        if !output.status.success() {
            let error_message = String::from_utf8_lossy(&output.stderr).into_owned();
            return Err(format!("Failed to link object files: {}", error_message));
        }

        Ok(())
    }

    fn execute(&self, build: &BuildCommand) -> Result<(), Box<dyn std::error::Error>> {
        let output_file = &build.output;
        let output = std::process::Command::new(output_file)
            .output()
            .map_err(|e| format!("Failed to execute output file: {}", e))?;

        // Check if the output file succeeded
        if !output.status.success() {
            let error_message = String::from_utf8_lossy(&output.stderr).into_owned();
            return Err(format!("Failed to execute output file: {}", error_message).into());
        }

        // Print the output of the output file
        let output_text = String::from_utf8_lossy(&output.stdout);
        println!("{}", output_text);

        Ok(())
    }

    fn get_object_file(build: &BuildCommand) -> PathBuf {
        build.output.with_extension("o")
    }

    fn get_asm_file(build: &BuildCommand) -> PathBuf {
        build.output.with_extension("s")
    }

    fn gen_asm(&self, build: &BuildCommand) -> Result<(), Box<dyn std::error::Error>> {
        let source_text = text::io::read_source_text(&build.input)?;
        let compilation_unit = CompilationUnit::compile(&source_text).map_err(|_| Box::new(Error::new(ErrorKind::Other, "Compilation failed")))?;

        let x86_gen = crate::codegen::x86::X86Codegen::new(
            &compilation_unit.mir,
            compilation_unit.scope.clone()
        );

        let asm = x86_gen.gen();
        let mut file = File::create(&Self::get_asm_file(build))?;
        file.write_all(asm.as_bytes())?;
        Ok(())
    }
}

