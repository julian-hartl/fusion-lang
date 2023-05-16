use std::collections::HashMap;
use std::fmt::{Display, format};

use crate::hir::{HIRBinaryOperator, Scope, ScopeCell};
use crate::mir::{Instruction, InstructionKind, Label, MIR, Place, Value, Var};
use crate::typings::Layout;

#[derive(Debug, PartialEq, Eq)]
enum X86Register {
    AL,
    AH,
    AX,
    EAX,
    RAX,
    RBX,
    RCX,
    RDX,
    RSI,
    RDI,
    RSP,
    RBP,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
}

impl Display for X86Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            X86Register::AL => write!(f, "al"),
            X86Register::AH => write!(f, "ah"),
            X86Register::AX => write!(f, "ax"),
            X86Register::EAX => write!(f, "eax"),
            X86Register::RAX => write!(f, "rax"),
            X86Register::RBX => write!(f, "rbx"),
            X86Register::RCX => write!(f, "rcx"),
            X86Register::RDX => write!(f, "rdx"),
            X86Register::RSI => write!(f, "rsi"),
            X86Register::RDI => write!(f, "rdi"),
            X86Register::RSP => write!(f, "rsp"),
            X86Register::RBP => write!(f, "rbp"),
            X86Register::R8 => write!(f, "r8"),
            X86Register::R9 => write!(f, "r9"),
            X86Register::R10 => write!(f, "r10"),
            X86Register::R11 => write!(f, "r11"),
            X86Register::R12 => write!(f, "r12"),
            X86Register::R13 => write!(f, "r13"),
            X86Register::R14 => write!(f, "r14"),
            X86Register::R15 => write!(f, "r15"),
        }
    }
}

enum X86Operand {
    Register(X86Register),
    Memory {
        base: X86Register,
        offset: i32,
        size: Option<X86Size>,
    },
    Immediate(X86Immediate),
}

impl Display for X86Operand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            X86Operand::Register(reg) => write!(f, "{}", reg),
            X86Operand::Memory { base, offset, size } => {
                let offset = *offset;
                if offset == 0 {
                    match size {
                        None => write!(f, "[{}]", base),
                        Some(size) => write!(f, "{} [{}]", size, base),
                    }
                } else {
                    let sign = if offset < 0 { "-" } else { "+" };
                    let offset = offset.abs();
                    match size {
                        None => write!(f, "[{} {} {}]", base, sign, offset),
                        Some(size) => write!(f, "{} [{} {} {}]", size, base, sign, offset),
                    }
                }
            }
            X86Operand::Immediate(imm) => write!(f, "{}", imm),
        }
    }
}

enum X86Immediate {
    QWord(i64),
    DWord(i32),
    Word(i16),
    Byte(i8),
}

impl Display for X86Immediate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            X86Immediate::QWord(imm) => write!(f, "{}", imm),
            X86Immediate::DWord(imm) => write!(f, "{}", imm),
            X86Immediate::Word(imm) => write!(f, "{}", imm),
            X86Immediate::Byte(imm) => write!(f, "{}", imm),
        }
    }
}

impl X86Immediate {
    pub fn size(&self) -> X86Size {
        match self {
            X86Immediate::QWord(_) => X86Size::QWord,
            X86Immediate::DWord(_) => X86Size::DWord,
            X86Immediate::Word(_) => X86Size::Word,
            X86Immediate::Byte(_) => X86Size::Byte,
        }
    }
}

enum X86Instruction {
    Mov(X86Operand, X86Operand),
    Add(X86Operand, X86Operand),
    Sub(X86Operand, X86Operand),
    Mul(X86Operand),
    Div(X86Operand),
    And(X86Operand, X86Operand),
    Cmp(X86Operand, X86Operand),
    Jmp(Label),
    Je(Label),
    Jne(Label),
    Jg(Label),
    Jge(Label),
    Jl(Label),
    Jle(Label),
    Call(String),
    Ret,
    Push(X86Operand),
    Pop(X86Operand),
    Syscall,
    Raw(&'static str),
    Lea(X86Operand, X86Operand),
    Cqo,
    Setl(X86Register),
    Movzx(X86Register, X86Register),
}

impl Display for X86Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            X86Instruction::Mov(op1, op2) => write!(f, "mov {}, {}", op1, op2),
            X86Instruction::Add(op1, op2) => write!(f, "add {}, {}", op1, op2),
            X86Instruction::Sub(op1, op2) => write!(f, "sub {}, {}", op1, op2),
            X86Instruction::Mul(op2) => write!(f, "mul {}", op2),
            X86Instruction::Div(op2) => write!(f, "div {}", op2),
            X86Instruction::Cmp(op1, op2) => write!(f, "cmp {}, {}", op1, op2),
            X86Instruction::Jmp(label) => write!(f, "jmp {}", label),
            X86Instruction::Je(label) => write!(f, "je {}", label),
            X86Instruction::Jne(label) => write!(f, "jne {}", label),
            X86Instruction::Jg(label) => write!(f, "jg {}", label),
            X86Instruction::Jge(label) => write!(f, "jge {}", label),
            X86Instruction::Jl(label) => write!(f, "jl {}", label),
            X86Instruction::Jle(label) => write!(f, "jle {}", label),
            X86Instruction::Call(label) => write!(f, "call {}", label),
            X86Instruction::Ret => write!(f, "ret"),
            X86Instruction::Push(op) => write!(f, "push {}", op),
            X86Instruction::Pop(op) => write!(f, "pop {}", op),
            X86Instruction::And(op1, op2) => write!(f, "and {}, {}", op1, op2),
            X86Instruction::Syscall => write!(f, "syscall"),
            X86Instruction::Raw(instruction) => write!(f, "{}", instruction),
            X86Instruction::Lea(op1, op2) => write!(f, "lea {}, {}", op1, op2),
            X86Instruction::Cqo => write!(f, "cqo"),
            X86Instruction::Setl(reg) => write!(f, "setl {}", reg),
            X86Instruction::Movzx(reg1, reg2) => write!(f, "movzx {}, {}", reg1, reg2),
        }
    }
}

enum X86Size {
    Byte,
    Word,
    DWord,
    QWord,
}

impl X86Size {
    fn from_layout(layout: &Layout) -> Option<Self> {
        match layout.size {
            1 => Some(X86Size::Byte),
            2 => Some(X86Size::Word),
            4 => Some(X86Size::DWord),
            8 => Some(X86Size::QWord),
            _ => None,
        }
    }
}

impl Display for X86Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            X86Size::Byte => write!(f, "byte"),
            X86Size::Word => write!(f, "word"),
            X86Size::DWord => write!(f, "dword"),
            X86Size::QWord => write!(f, "qword"),
        }?;
        write!(f, " ptr")
    }
}

const INITIAL_BASE_POINTER_OFFSET: u32 = 8;

pub struct X86Codegen<'a> {
    var_addr: HashMap<Var, u32>,
    mir: &'a MIR,
    scope: ScopeCell,
    asm: String,
    base_pointer_offset: u32,
}

impl<'a> X86Codegen<'a> {
    pub fn new(
        mir: &'a MIR,
        scope: ScopeCell,
    ) -> Self {
        Self {
            mir,
            asm: String::new(),
            scope,
            var_addr: HashMap::new(),
            base_pointer_offset: INITIAL_BASE_POINTER_OFFSET,
        }
    }

    pub fn gen(mut self) -> String {
        self.asm.push_str(".intel_syntax noprefix\n");
        self.asm.push_str(".global _start\n");
        self.gen_start();
        for body in self.mir.bodies.iter() {
            self.gen_body(body);
        }
        self.asm
    }

    pub fn gen_start(&mut self) {
        self.asm.push_str("_start:\n");
        self.push_instruction(X86Instruction::Mov(
            X86Operand::Register(X86Register::RBP),
            X86Operand::Register(X86Register::RSP),
        ));
        self.push_instruction(X86Instruction::And(
            X86Operand::Register(X86Register::RSP),
            X86Operand::Immediate(X86Immediate::QWord(-16)),
        ));
        self.push_instruction(X86Instruction::Sub(
            X86Operand::Register(X86Register::RSP),
            X86Operand::Immediate(X86Immediate::QWord(8)),
        ));
        self.push_instruction(X86Instruction::Call("main".to_string()));
        self.push_instruction(X86Instruction::Mov(
            X86Operand::Register(X86Register::RDI),
            X86Operand::Register(X86Register::RAX),
        ));
        self.push_instruction(X86Instruction::Mov(
            X86Operand::Register(X86Register::RAX),
            X86Operand::Immediate(X86Immediate::QWord(60)),
        ));
        self.push_instruction(X86Instruction::Syscall);
    }

    pub fn gen_body(&mut self, body: &crate::mir::Body) {
        self.base_pointer_offset = 0;
        self.var_addr.clear();
        let scope = self.scope.borrow();
        let function = scope.get_function(&body.function);
        self.asm.push_str(&format!("{}:\n", function.name));
        drop(scope);
        self.push_instruction(X86Instruction::Push(
            X86Operand::Register(X86Register::RBP),
        ));
        self.push_instruction(X86Instruction::Mov(
            X86Operand::Register(X86Register::RBP),
            X86Operand::Register(X86Register::RSP),
        ));
        let locals_size = body
            .scope.locals_size;
        self.push_instruction(X86Instruction::Sub(
            X86Operand::Register(X86Register::RSP),
            X86Operand::Immediate(X86Immediate::QWord(locals_size as i64)),
        ));
        for bb in body.basic_blocks.iter() {
            self.gen_basic_block(bb);
        }
    }

    fn clear_stack_frame(&mut self) {
        self.push_instruction(X86Instruction::Mov(
            X86Operand::Register(X86Register::RSP),
            X86Operand::Register(X86Register::RBP),
        ));
        self.push_instruction(X86Instruction::Pop(
            X86Operand::Register(X86Register::RBP),
        ));
        self.push_instruction(X86Instruction::Ret);
    }

    pub fn gen_basic_block(&mut self, bb: &crate::mir::BasicBlock) {
        self.asm.push_str(&format!("{}:\n", bb.label));
        for inst in bb.instructions.iter() {
            self.gen_instruction(inst);
        }
        self.gen_term(&bb.terminator);
    }

    pub fn gen_term(&mut self, term: &crate::mir::Terminator) {
        match &term.kind {
            crate::mir::TerminatorKind::Goto(label) => {
                self.push_instruction(X86Instruction::Jmp(label.clone()));
            }
            crate::mir::TerminatorKind::Return(storage) => {
                // todo: fix this by properly handling void values
                if storage.layout.size != 0 {
                    let operand = self.gen_mem_op(storage);
                    // self.gen_value(value, storage);
                    self.push_instruction(X86Instruction::Mov(
                        X86Operand::Register(X86Register::RAX),
                        operand,
                    ));
                }
                self.clear_stack_frame();
            }
            crate::mir::TerminatorKind::If {
                condition,
                then,
                else_,
            } => {
                // todo: maybe do not pass a Value here but rather a new type called condition which can hold bool binary expressions or literal values
                let condition_operand = self.gen_mem_op(condition);
                self.push_instruction(X86Instruction::Cmp(
                    condition_operand,
                    X86Operand::Immediate(X86Immediate::QWord(0)),
                ));
                self.push_instruction(X86Instruction::Jne(*then));
                self.push_instruction(X86Instruction::Jmp(*else_));
            }
            crate::mir::TerminatorKind::Next => {}
        }
    }

    pub fn gen_instruction(&mut self, instruction: &Instruction) {
        match &instruction.kind {
            InstructionKind::Store {
                place,
                value
            } => {
                self.gen_mem_op(place);
                self.gen_value(value, place);
            }
            InstructionKind::Call { .. } => {
                unimplemented!()
            }
            InstructionKind::BinaryOp {
                operator,
                result_place,
                lhs,
                rhs
            } => {
                self.gen_mem_op(result_place);
                self.gen_binary_op(operator, result_place, lhs, rhs);
            }
            InstructionKind::UnaryOp { .. } => {
                unimplemented!()
            }
            InstructionKind::Deref { from, to } => {
                self.gen_mem_op(to);
                self.deref_value(from, to);
            }
            InstructionKind::Move { from, to } => {
                self.copy_value(from, to);
            }
        }
    }

    fn deref_value(&mut self, from: &Place, to: &Place) {
        let from_mem_op = self.gen_mem_op(&from);
        let to_mem_op = self.gen_mem_op(&to);
        if to.layout.size <= Layout::POINTER_SIZE {
            self.push_instruction(X86Instruction::Mov(
                X86Operand::Register(X86Register::RAX),
                from_mem_op,
            ));
            self.push_instruction(X86Instruction::Mov(
                X86Operand::Register(X86Register::RAX),
                X86Operand::Memory {
                    base: X86Register::RAX,
                    offset: 0,
                    size: None
                }
            ));
            self.push_instruction(X86Instruction::Mov(
                to_mem_op,
                X86Operand::Register(X86Register::RAX),
            ));
        } else {
            self.push_instruction(X86Instruction::Mov(
                X86Operand::Register(X86Register::RSI),
                from_mem_op,
            ));
            self.push_instruction(X86Instruction::Lea(
                X86Operand::Register(X86Register::RDI),
                to_mem_op,
            ));
            self.push_instruction(X86Instruction::Mov(
                X86Operand::Register(X86Register::RCX),
                X86Operand::Immediate(X86Immediate::QWord(to.layout.size as i64)),
            ));
            self.push_instruction(X86Instruction::Raw("rep movsb"));
        }
    }

    fn copy_value(&mut self, from: &Place, to: &Place) {
        let from_mem_op = self.gen_mem_op(&from);
        let to_mem_op = self.gen_mem_op(&to);
        if to.layout.size <= Layout::POINTER_SIZE {
            self.push_instruction(X86Instruction::Mov(
                X86Operand::Register(X86Register::RAX),
                from_mem_op,
            ));
            self.push_instruction(X86Instruction::Mov(
                to_mem_op,
                X86Operand::Register(X86Register::RAX),
            ));
        } else {
            self.push_instruction(X86Instruction::Lea(
                X86Operand::Register(X86Register::RSI),
                from_mem_op,
            ));
            self.push_instruction(X86Instruction::Lea(
                X86Operand::Register(X86Register::RDI),
                to_mem_op,
            ));
            self.push_instruction(X86Instruction::Mov(
                X86Operand::Register(X86Register::RCX),
                X86Operand::Immediate(X86Immediate::QWord(to.layout.size as i64)),
            ));
            self.push_instruction(X86Instruction::Raw("rep movsb"));
        }
    }

    fn gen_value(&mut self, value: &Value, store_at: &Place) {
        match value {
            Value::I64(value) => {
                let store_at_mem_op = self.gen_mem_op(store_at);
                self.push_instruction(X86Instruction::Mov(
                    store_at_mem_op,
                    X86Operand::Immediate(
                        X86Immediate::QWord(*value),
                    ),
                ));
            }
            Value::Char(value) => {
                let store_at_mem_op = self.gen_mem_op(store_at);
                self.push_instruction(X86Instruction::Mov(
                    store_at_mem_op,
                    X86Operand::Immediate(
                        X86Immediate::Byte(*value as u8 as i8)
                    ),
                ));
            }
            Value::String(_) => {
                unimplemented!()
            }
            Value::Bool(value) => {
                let store_at_mem_op = self.gen_mem_op(store_at);
                self.push_instruction(X86Instruction::Mov(
                    store_at_mem_op,
                    X86Operand::Immediate(
                        X86Immediate::Byte(if *value { 1 } else { 0 })
                    ),
                ));
            }
            Value::Struct(values) => {
                let mut offset = 0;
                for value in values.iter() {
                    let store_at = Place::new(store_at.var, offset, value.layout());
                    self.gen_value(value, &store_at);
                    offset += value.layout().size;
                }
            }
            Value::Void => {}
            Value::Ptr(place) => {
                let operand = self.gen_mem_op(place);
                self.push_instruction(X86Instruction::Lea(
                    X86Operand::Register(X86Register::RAX),
                    operand,
                ));
                let store_at_mem_op = self.gen_mem_op(store_at);
                self.push_instruction(X86Instruction::Mov(
                    store_at_mem_op,
                    X86Operand::Register(X86Register::RAX),
                ));
            }
            Value::StoredAt(place) => {
                self.copy_value(place, store_at);
            }
        }
    }

    fn gen_mem_op(&mut self, place: &Place) -> X86Operand {
        if !self.var_addr.contains_key(&place.var) {
            let size = place.layout.size;
            // self.push_instruction(X86Instruction::Sub(
            //     X86Operand::Register(X86Register::RSP),
            //     X86Operand::Immediate(X86Immediate::QWord(size as i64)),
            // ));
            self.base_pointer_offset += size;
            self.var_addr.insert(place.var, self.base_pointer_offset);
        }
        let offset = self.get_place_offset(place) + place.offset;
        X86Operand::Memory {
            base: X86Register::RBP,
            offset: -(offset as i32),
            size: X86Size::from_layout(&place.layout),
        }
    }

    fn get_place_offset(&mut self, place: &Place) -> u32 {
        let var = &place.var;
        self.var_addr[var]
    }

    fn push_instruction(&mut self, instruction: X86Instruction) {
        self.asm.push_str("    ");
        self.asm.push_str(format!("{}", instruction).as_str());
        self.asm.push_str("\n");
    }
    fn gen_binary_op(&mut self, op: &HIRBinaryOperator, store_at: &Place, lhs: &Value, rhs: &Value) {
        let lhs_op = self.gen_value_op(lhs);
        let rhs_op = self.gen_value_op(rhs);
        let store_at_op = self.gen_mem_op(store_at);
        match op {
            HIRBinaryOperator::Add => {
                self.push_instruction(X86Instruction::Mov(
                    X86Operand::Register(X86Register::RAX),
                    lhs_op,
                ));
                self.push_instruction(X86Instruction::Add(
                    X86Operand::Register(X86Register::RAX),
                    rhs_op,
                ));
                self.push_instruction(X86Instruction::Mov(
                    store_at_op,
                    X86Operand::Register(X86Register::RAX),
                ));
            }
            HIRBinaryOperator::Subtract => {
                self.push_instruction(X86Instruction::Mov(
                    X86Operand::Register(X86Register::RAX),
                    lhs_op,
                ));
                self.push_instruction(X86Instruction::Sub(
                    X86Operand::Register(X86Register::RAX),
                    rhs_op,
                ));
                self.push_instruction(X86Instruction::Mov(
                    store_at_op,
                    X86Operand::Register(X86Register::RAX),
                ));
            }
            HIRBinaryOperator::Multiply => {
                self.push_instruction(X86Instruction::Mov(
                    X86Operand::Register(X86Register::RAX),
                    lhs_op,
                ));
                self.push_instruction(X86Instruction::Mul(
                    rhs_op,
                ));
                self.push_instruction(X86Instruction::Mov(
                    store_at_op,
                    X86Operand::Register(X86Register::RAX),
                ));
            }
            HIRBinaryOperator::Divide => {
                self.push_instruction(X86Instruction::Mov(
                    X86Operand::Register(X86Register::RAX),
                    lhs_op,
                ));
                self.push_instruction(X86Instruction::Cqo);
                self.push_instruction(X86Instruction::Div(
                    rhs_op,
                ));
                self.push_instruction(X86Instruction::Mov(
                    store_at_op,
                    X86Operand::Register(X86Register::RAX),
                ));
            }
            HIRBinaryOperator::LessThan => {
                self.push_instruction(X86Instruction::Mov(
                    X86Operand::Register(X86Register::RAX),
                    lhs_op,
                ));
                self.push_instruction(X86Instruction::Cmp(
                    X86Operand::Register(X86Register::RAX),
                    rhs_op,
                ));
                self.push_instruction(X86Instruction::Setl(
                    X86Register::AL,
                ));
                self.push_instruction(X86Instruction::Mov(
                    store_at_op,
                    X86Operand::Register(X86Register::AL),
                ));
            }
            _ => unimplemented!(),
        }
    }

    fn gen_value_op(&mut self, value: &Value) -> X86Operand {
        match value {
            Value::I64(value) => {
                X86Operand::Immediate(X86Immediate::QWord(*value))
            }
            Value::Char(value) => {
                X86Operand::Immediate(X86Immediate::Byte(*value as u8 as i8))
            }
            Value::String(_) => {
                unimplemented!()
            }
            Value::Bool(value) => {
                X86Operand::Immediate(X86Immediate::Byte(if *value { 1 } else { 0 }))
            }
            Value::Struct(values) => {
                unimplemented!()
            }
            Value::Void => {
                unimplemented!()
            }
            Value::Ptr(place) => {
                unimplemented!()
            }
            Value::StoredAt(place) => {
                self.gen_mem_op(place)
            }
        }
    }
}