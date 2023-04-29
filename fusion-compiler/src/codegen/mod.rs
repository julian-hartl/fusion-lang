pub mod llvm;

use crate::ir::IR;

pub trait Codegen {

    fn gen(
        &mut self,
        ir: &IR,
    ) ->  Result<String, std::fmt::Error>;

}