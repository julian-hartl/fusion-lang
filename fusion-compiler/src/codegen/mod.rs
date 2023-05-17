use crate::modules::scopes::GlobalScope;
use crate::mir::MIR;

pub mod llvm;
pub mod x86;


pub trait Codegen {
    fn gen(
        &mut self,
        mir: &MIR,
        scope: &GlobalScope,
    ) -> Result<String, std::fmt::Error>;
}