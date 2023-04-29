use crate::ast::ASTNodeId;
use crate::ir::basic_block::Label;
use crate::ir::instruction::{Primary, Terminator};

impl Terminator {
    pub fn new(kind: TerminatorKind, node_id: ASTNodeId) -> Self {
        Self { kind, node_id }
    }

    pub fn unresolved() -> Self {
        Self::new(TerminatorKind::Unresolved, ASTNodeId::Unknown)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TerminatorKind {
    Goto(Label),
    If(Primary, Label, Label),
    Return(Option<Primary>),
    Unresolved,
}
