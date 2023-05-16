use crate::hir::Scope;
use crate::mir::MIR;

pub mod llvm;
pub mod x86;


pub trait Codegen {
    fn gen(
        &mut self,
        mir: &MIR,
        scope: &Scope,
    ) -> Result<String, std::fmt::Error>;
}