use std::fmt::{Display, Formatter};

use crate::ast::{ASTBinaryOperatorKind, ASTNodeId, ASTUnaryOperatorKind};
use crate::compilation::symbols::class::ClassSymbol;
use crate::compilation::symbols::function::FunctionSymbol;
use crate::compilation::symbols::variable::{VariableId, VariableSymbol};
use crate::ir::terminator::TerminatorKind;
use crate::typings::{FunctionType, Type};

#[derive(Debug, Clone)]
pub enum InstructionKind {
    Binary(
        IRBinaryOperator,
        Primary,
        Primary,
    ),
    Unary(
        IRUnaryOperator,
        Primary,
    ),

    Store(VariableSymbol, Primary),
    Alloc(VariableSymbol),
    Primary(Primary),
}

impl Display for InstructionKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            InstructionKind::Binary(binary, lhs, rhs) => write!(f, "{} {} {}", lhs, binary, rhs),
            InstructionKind::Unary(unary, operand) => write!(f, "{}{}", unary, operand),
            InstructionKind::Primary(operand) => write!(f, "{}", operand),
            InstructionKind::Store(variable, value) => write!(f, "{} = {}", variable.name, value),
            InstructionKind::Alloc(variable) => {
                write!(f, "{} = alloc {}", variable.name, variable.ty)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum IRBinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
    BitAnd,
    BitOr,
    BitXor,
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
}

impl Display for IRBinaryOperator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IRBinaryOperator::Add => write!(f, "+"),
            IRBinaryOperator::Sub => write!(f, "-"),
            IRBinaryOperator::Mul => write!(f, "*"),
            IRBinaryOperator::Div => write!(f, "/"),
            IRBinaryOperator::BitAnd => write!(f, "&"),
            IRBinaryOperator::BitOr => write!(f, "|"),
            IRBinaryOperator::BitXor => write!(f, "^"),
            IRBinaryOperator::Eq => write!(f, "=="),
            IRBinaryOperator::Neq => write!(f, "!="),
            IRBinaryOperator::Lt => write!(f, "<"),
            IRBinaryOperator::Lte => write!(f, "<="),
            IRBinaryOperator::Gt => write!(f, ">"),
            IRBinaryOperator::Gte => write!(f, ">="),
        }
    }
}

impl From<&ASTBinaryOperatorKind> for IRBinaryOperator {
    fn from(kind: &ASTBinaryOperatorKind) -> Self {
        match kind {
            ASTBinaryOperatorKind::Plus => IRBinaryOperator::Add,
            ASTBinaryOperatorKind::Minus => IRBinaryOperator::Sub,
            ASTBinaryOperatorKind::Multiply => IRBinaryOperator::Mul,
            ASTBinaryOperatorKind::Divide => IRBinaryOperator::Div,
            ASTBinaryOperatorKind::BitwiseAnd => IRBinaryOperator::BitAnd,
            ASTBinaryOperatorKind::BitwiseOr => IRBinaryOperator::BitOr,
            ASTBinaryOperatorKind::BitwiseXor => IRBinaryOperator::BitXor,
            ASTBinaryOperatorKind::Equals => IRBinaryOperator::Eq,
            ASTBinaryOperatorKind::NotEquals => IRBinaryOperator::Neq,
            ASTBinaryOperatorKind::LessThan => IRBinaryOperator::Lt,
            ASTBinaryOperatorKind::LessThanOrEqual => IRBinaryOperator::Lte,
            ASTBinaryOperatorKind::GreaterThan => IRBinaryOperator::Gt,
            ASTBinaryOperatorKind::GreaterThanOrEqual => IRBinaryOperator::Gte,
            ASTBinaryOperatorKind::Power => unimplemented!(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum IRUnaryOperator {
    Neg,
    BitNot,
}

impl From<&ASTUnaryOperatorKind> for IRUnaryOperator {
    fn from(kind: &ASTUnaryOperatorKind) -> Self {
        match kind {
            ASTUnaryOperatorKind::Minus => IRUnaryOperator::Neg,
            ASTUnaryOperatorKind::BitwiseNot => IRUnaryOperator::BitNot,
        }
    }
}

impl Display for IRUnaryOperator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IRUnaryOperator::Neg => write!(f, "-"),
            IRUnaryOperator::BitNot => write!(f, "~"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub kind: InstructionKind,
    pub assign_to: Option<VariableSymbol>,
    pub node_id: ASTNodeId,
}

impl Display for Instruction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(assign_to) = &self.assign_to {
            write!(f, "{} = ", assign_to.name)?;
        }
        write!(f, "{}", self.kind)
    }
}

#[derive(Debug, Clone)]
pub struct Terminator {
    pub kind: TerminatorKind,
    pub node_id: ASTNodeId,
}

impl Display for Terminator {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            TerminatorKind::Goto(label) => write!(f, "goto {}", label.name),
            TerminatorKind::If(condition, then_label, else_label) => {
                write!(f, "if {} then {} else {}", condition, then_label.name, else_label.name)
            }
            TerminatorKind::Return(value) => {
                if let Some(value) = value {
                    write!(f, "return {}", value)
                } else {
                    write!(f, "return")
                }
            }
            TerminatorKind::Unresolved => write!(f, "unresolved"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Primary {
    Integer(i64),
    Boolean(bool),
    String(String),
    Variable(VariableSymbol),
    FuncRef(FunctionSymbol),
    New(ClassSymbol),
    Call(String, Vec<Primary>),
    MemberAccess(Box<Primary>, Member),
    MethodAccess(Box<Primary>, FunctionSymbol),
    Self_(Type),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    pub ty: Type,
    pub index: u32,
}

impl Display for Primary {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Primary::Integer(value) => write!(f, "{}", value),
            Primary::Boolean(value) => write!(f, "{}", value),
            Primary::Variable(variable) => write!(f, "{}", &variable.name),
            Primary::Call(name, args) => {
                write!(f, "{}(", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", arg)?;
                }
                write!(f, ")")
            }
            Primary::String(value) => {
                write!(f, "\"")?;
                write!(f, "{}", value)?;
                write!(f, "\"")
            }
            Primary::MemberAccess(obj, Member { ty, index: index }) => {
                write!(f, "{}[{}] @ {}", obj, index, ty)
            }
            Primary::Self_(ty) => write!(f, "self @ {}", ty),
            Primary::FuncRef(func) => {
                write!(f, "{}", func.name)
            }
            Primary::New(class) => {
                write!(f, "new {}", class.name)
            }
            Primary::MethodAccess(obj, func) => {
                write!(f, "{}.{}", obj, func.name)
            }
        }
    }
}
