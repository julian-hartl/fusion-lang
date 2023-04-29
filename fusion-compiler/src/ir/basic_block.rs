use crate::ast::Ast;
use crate::compilation::symbols::function::FunctionSymbol;
use crate::ir::instruction::{Instruction, Terminator};
use crate::text::span::TextSpan;

pub struct BasicBlock {
    pub label: Label,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
    pub function: Option<FunctionSymbol>,
}

impl BasicBlock {
    pub fn span(&self, ast: &Ast) -> Option<TextSpan> {
        let nodes = self.instructions.iter().map(|i| ast.compute_node(&i.node_id)).collect::<Vec<_>>();
        let mut spans = nodes.iter().map(|n| n.as_ref().map(|n| &n.span)).collect::<Vec<_>>();
        let term_node = ast.compute_node(&self.terminator.node_id);
        if let Some(term_node) = term_node.as_ref() {
            spans.push(Some(&term_node.span));
        }
        let spans: Vec<&TextSpan> = spans.into_iter().filter_map(|s| s).collect();
        if spans.is_empty() {
            None
        } else {
            Some(TextSpan::merge(spans))
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Label {
    pub name: String,
}

impl Label {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}
