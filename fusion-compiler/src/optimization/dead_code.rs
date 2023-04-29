use std::collections::HashMap;
use std::ops::Deref;

use crate::ast::Ast;
use crate::compilation::symbols::variable::VariableId;
use crate::diagnostics::DiagnosticsBagCell;
use crate::ir::{IR, VariableMetadata};
use crate::ir::basic_block::{BasicBlock, Label};
use crate::ir::instruction::{Instruction, InstructionKind, Primary};
use crate::ir::terminator::TerminatorKind;

pub struct DeadCodeElimination<'a> {
    pub diagnostics_bag: DiagnosticsBagCell,
    pub ast: &'a Ast,
    pub variable_usages: &'a HashMap<VariableId, VariableMetadata>,

}

impl<'a> DeadCodeElimination<'a> {
    pub fn new(
        diagnostics_bag: DiagnosticsBagCell,
        ast: &'a Ast,
        variable_metadata: &'a HashMap<VariableId, VariableMetadata>,
    ) -> Self {
        Self {
            diagnostics_bag,
            ast,
            variable_usages: variable_metadata,
        }
    }

    pub fn remove_and_report(&mut self, ir: &'a mut IR) {
        let unvisited_blocks = self.find_unvisited_blocks(ir);
        self.report_dead_code(&unvisited_blocks, ir);
        self.report_not_returning_functions(ir);
        // self.remove_unvisited_blocks(&unvisited_blocks, ir);
    }

    /// Checks if all paths in a function return a value
    fn report_not_returning_functions(&self, ir: &'a IR) {
        for function in ir.functions.iter() {
            let start_block = ir.get_block(&function).unwrap();
            let mut next_blocks = vec![start_block];
            let mut visited_blocks = Vec::new();

            let edges = ir.get_edges();
            while let Some(block) = next_blocks.pop() {
                if visited_blocks.contains(&block.label) {
                    continue;
                }
                visited_blocks.push(block.label.clone());
                let edges = edges.get(&block.label).unwrap();
                for edge in edges {
                    if edge.condition.unwrap_or(true) {
                        let next_block = ir.get_block(&edge.to).unwrap();
                        next_blocks.push(next_block);
                    }
                }
                if edges.is_empty() {
                    match &block.terminator.kind {
                        TerminatorKind::Return(Some(_)) => {}
                        _ => {
                            let span = block.span(&self.ast).expect(format!("No span for block {:?}", block.label).as_str());
                            self.diagnostics_bag.borrow_mut().report_missing_return(&span);
                        }
                    }
                }
            }
        }
    }

    fn report_dead_code(&self, unvisited_blocks: &Vec<Label>, ir: &'a IR) {
        for block_label in unvisited_blocks {
            let block = ir.get_block(block_label).unwrap();
            let span = block.span(&self.ast);
            if let Some(span) = span {
                self.diagnostics_bag.borrow_mut().report_unreachable_code(&span);
            }
        }
    }

    fn remove_unvisited_blocks(&mut self, unvisited_blocks: &Vec<Label>, ir: &'a mut IR) {
        ir.basic_blocks.retain(|block| !unvisited_blocks.contains(&&block.label));
        self.remove_unused_statements(ir);
    }

    /// This methods traverses all paths of the CFG and returns the basic blocks that have not been visited.
    ///
    /// It does so by starting at the main entry point and walking all paths.
    fn find_unvisited_blocks(&self, ir: &'a IR) -> Vec<Label> {
        let mut unvisited_blocks = Vec::new();
        let mut visited_blocks = Vec::new();
        let mut stack = Vec::new();
        stack.push(&ir.get_entry_point().label);
        let edges = ir.get_edges();
        while let Some(block_label) = stack.pop() {
            if visited_blocks.contains(&block_label) {
                continue;
            }
            visited_blocks.push(&block_label);
            for next_block in edges.get(&block_label).expect(
                format!("No edges found for block {:?}", block_label).as_str(),
            ) {
                if next_block.condition.unwrap_or(true) {
                    stack.push(&next_block.to);
                }
            }
        }
        for block in &ir.basic_blocks {
            if !visited_blocks.contains(&&block.label) {
                if let None = block.function {
                    unvisited_blocks.push(block.label.clone());
                }
            }
        }
        unvisited_blocks
    }

    fn remove_unused_statements(&mut self, ir: &'a mut IR)
    {
        let variable_usages = self.variable_usages;
        for block in ir.basic_blocks.iter_mut() {
            block.instructions.retain(|instruction| {
                match &instruction.kind {
                    InstructionKind::Alloc(alloc) => {
                        variable_usages.get(&alloc.id).unwrap().usages > 0
                    }
                    InstructionKind::Store(store, ..) => {
                        variable_usages.get(&store.id).unwrap().usages > 0
                    }

                    _ => true
                }
            });
        }
    }
}
