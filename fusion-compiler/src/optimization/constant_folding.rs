use std::collections::HashMap;

use crate::ast::Ast;
use crate::compilation::symbols::variable::VariableId;
use crate::diagnostics::DiagnosticsBagCell;
use crate::ir::{IR, VariableMetadata};
use crate::ir::basic_block::BasicBlock;
use crate::ir::instruction::{InstructionKind, IRBinaryOperator, IRUnaryOperator, Primary};
use crate::ir::terminator::TerminatorKind;

pub struct ConstantFolding<'a> {
    pub ast: &'a Ast,
    pub diagnostics_bag: DiagnosticsBagCell,
    pub variable_metadata: &'a HashMap<VariableId, VariableMetadata>,
}

impl<'a> ConstantFolding<'a> {
    pub fn new(
        ast: &'a Ast,
        diagnostics_bag: DiagnosticsBagCell,
        variable_usages: &'a HashMap<VariableId, VariableMetadata>,
    ) -> Self {
        Self {
            ast,
            diagnostics_bag,
            variable_metadata: variable_usages,
        }
    }

    pub fn fold(&mut self, ir: &'a mut IR) {
        let mut values = HashMap::new();
        for block in ir.basic_blocks.iter_mut() {
            self.fold_block(block, &mut values);
        }
    }

    pub fn fold_block(&self, block: &mut BasicBlock, values: &mut HashMap<VariableId, i64>) {
        for instruction in block.instructions.iter_mut() {
            let const_value = match &instruction.kind {
                InstructionKind::Binary(op, lhs, rhs) => {
                    let lhs_value = self.get_const_value(lhs, values);
                    let rhs_value = self.get_const_value(rhs, values);
                    if let (Some(lhs_value), Some(rhs_value)) = (lhs_value, rhs_value) {
                        Some(self.eval_binary(lhs_value, rhs_value, op))
                    } else if let Some(lhs) = lhs_value {
                        instruction.kind = InstructionKind::Binary(op.clone(), Primary::Integer(lhs), rhs.clone());
                        None
                    } else if let Some(rhs) = rhs_value {
                        instruction.kind = InstructionKind::Binary(op.clone(), lhs.clone(), Primary::Integer(rhs));
                        None
                    } else {
                        None
                    }
                }
                InstructionKind::Unary(op, operand) => {
                    let operand_value = self.get_const_value(operand, values);
                    operand_value.map(|value| {
                        self.eval_unary(value, op)
                    })
                }
                InstructionKind::Store(var, value) => {
                    let value = self.get_const_value(value, values);
                    if let Some(value) = value {
                        values.insert(var.id, value);
                        instruction.kind = InstructionKind::Store(var.clone(), Primary::Integer(value));
                    }
                    None
                }
                InstructionKind::Alloc(var) => {
                    values.insert(var.id, 0);
                    None
                }
                InstructionKind::Primary(primary) => {
                    self.get_const_value(primary, values)
                }
            };
            if let (Some(var), Some(value)) = (instruction.assign_to.as_ref(), const_value) {
                values.insert(var.id, value);
            }
            if let Some(value) = const_value {
                instruction.kind = InstructionKind::Primary(Primary::Integer(value));
            }
        }

        match &block.terminator.kind {
            TerminatorKind::Goto(_) => {}
            TerminatorKind::If(cond, _, else_label) => {
                let cond_value = self.get_const_value(cond, values);
                if let Some(value) = cond_value {
                    if value == 0 {
                        block.terminator.kind = TerminatorKind::Goto(else_label.clone());
                    }
                }
            }
            TerminatorKind::Return(value) => {
                let value = value.as_ref().map(|value| {
                    self.get_const_value(&value, values)
                }).flatten();
                if let Some(value) = value {
                    block.terminator.kind = TerminatorKind::Return(Some(Primary::Integer(value)));
                }
            }
            TerminatorKind::Unresolved => {}
        }
    }
    fn eval_unary(&self, value: i64, op: &IRUnaryOperator) -> i64 {
        match op {
            IRUnaryOperator::Neg => -value,
            IRUnaryOperator::BitNot => !value,
        }
    }

    fn eval_binary(&self, lhs: i64, rhs: i64, op: &IRBinaryOperator) -> i64 {
        match op {
            IRBinaryOperator::Add => lhs + rhs,
            IRBinaryOperator::Sub => lhs - rhs,
            IRBinaryOperator::Mul => lhs * rhs,
            IRBinaryOperator::Div => lhs / rhs,
            IRBinaryOperator::BitAnd => lhs & rhs,
            IRBinaryOperator::BitOr => lhs | rhs,
            IRBinaryOperator::BitXor => lhs ^ rhs,
            IRBinaryOperator::Eq => self.bool_to_int(lhs == rhs),
            IRBinaryOperator::Neq => self.bool_to_int(lhs != rhs),
            IRBinaryOperator::Lt => self.bool_to_int(lhs < rhs),
            IRBinaryOperator::Gt => self.bool_to_int(lhs > rhs),
            IRBinaryOperator::Lte => self.bool_to_int(lhs <= rhs),
            IRBinaryOperator::Gte => self.bool_to_int(lhs >= rhs),
        }
    }

    fn bool_to_int(&self, value: bool) -> i64 {
        if value {
            1
        } else {
            0
        }
    }

    fn get_const_value(&self, primary: &Primary, values: &HashMap<VariableId, i64>) -> Option<i64> {
        match primary {
            Primary::Integer(value) => Some(*value),
            Primary::Boolean(value) => {
                if *value {
                    Some(1)
                } else {
                    Some(0)
                }
            }
            Primary::Variable(var) => {
                // todo: problem here is that we do not optimize variables that are assigned again but the value that was assigned before is not used
                if !self.variable_metadata.get(&var.id).map(|m|m.has_been_reassigned()).unwrap_or(true) {
                    values.get(&var.id).cloned()
                }
                else {
                    None
                }
            }
            Primary::Call(func, args) => None,
            Primary::String(_) => None,
            Primary::MemberAccess(_, _) => None,
            Primary::Self_(_) => None,
            Primary::FuncRef(_) => None,
            Primary::New(_) => None,
            Primary::MethodAccess(_, _) => None,
        }
    }
}