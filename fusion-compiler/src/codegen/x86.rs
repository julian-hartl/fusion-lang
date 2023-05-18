use std::collections::HashMap;
use std::fmt::{Display, format};

use crate::hir::{FunctionId, HIRBinaryOperator};
use crate::mir::{GlobalLabel, GlobalPlace, GlobalValue, Instruction, InstructionKind, Label, LocalPlace, MIR, Place, Value, Var};
use crate::modules::scopes::{GlobalScope, GlobalScopeCell};
use crate::modules::symbols::{Function, QualifiedName};
use crate::typings::Layout;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone)]
enum X86Operand {
    Register(X86Register),
    Memory {
        base: X86Register,
        offset: i32,
        size: Option<X86Size>,
    },
    Immediate(X86Immediate),
    Global {
        label: GlobalLabel,
    },
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
            X86Operand::Global {
                label
            } => write!(f, "{}", label),
        }
    }
}

#[derive(Debug, Clone, Copy)]
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
    Sete(X86Register),
    Setne(X86Register),
    Setle(X86Register),
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
            X86Instruction::Setne(reg) => write!(f, "setne {}", reg),
            X86Instruction::Sete(reg) => write!(f, "sete {}", reg),
            X86Instruction::Setle(reg) => write!(f, "setle {}", reg),
            X86Instruction::Movzx(reg1, reg2) => write!(f, "movzx {}, {}", reg1, reg2),
        }
    }
}

#[derive(Debug, Clone, Copy)]
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
    var_addr: HashMap<Var, i32>,
    mir: &'a MIR,
    scope: GlobalScopeCell,
    asm: String,
    base_pointer_offset: u32,
}

impl<'a> X86Codegen<'a> {
    pub fn new(
        mir: &'a MIR,
        scope: GlobalScopeCell,
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
        self.gen_globals();
        self.asm.push_str(".text\n");
        self.gen_start();
        for body in self.mir.bodies.iter() {
            self.gen_body(body);
        }
        self.asm
    }

    pub fn gen_globals(&mut self) {
        self.asm.push_str(".data\n");
        for (label, value) in &self.mir.globals {
            self.asm.push_str(&format!("{}:\n", label));
            self.asm.push_str("    ");
            match value {
                GlobalValue::String(s) => {
                    self.asm.push_str(&format!(".asciz \"{}\"\n", s));
                }
            }
        }
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
        self.push_instruction(X86Instruction::Call("root__main".to_string()));
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
        self.asm.push_str(&format!("{}:\n", self.format_qualified_name(&function.name)));
        drop(scope);
        let mut offset = -16;
        for param in &body.parameters {
            self.var_addr.insert(param.var, offset);
            offset -= param.layout.size as i32;
        }
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

    fn format_qualified_name(&self, name: &QualifiedName) -> String {
        name.name.replace(":", "_")
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
                    let operand = self.gen_local_mem_op(&storage);
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
                self.gen_local_mem_op(place);
                self.gen_value(value, place);
            }
            InstructionKind::Call {
                return_value_place,
                args,
                function_id
            } => {
                let arg_size = self.layout_function_call_args(args);
                let scope = self.scope.borrow();
                let function = scope.get_function(function_id);
                let function_name = self.format_qualified_name(&function.name);
                drop(scope);
                self.push_instruction(X86Instruction::Call(function_name));
                self.push_instruction(X86Instruction::Add(
                    X86Operand::Register(X86Register::RSP),
                    X86Operand::Immediate(X86Immediate::QWord(arg_size as i64)),
                ));
                // todo: for now we assume that each function returns its value in rax
                let return_value_operand = self.gen_local_mem_op(return_value_place);
                self.push_instruction(X86Instruction::Mov(
                    return_value_operand,
                    X86Operand::Register(X86Register::RAX),
                ));
            }
            InstructionKind::BinaryOp {
                operator,
                result_place,
                lhs,
                rhs
            } => {
                self.gen_local_mem_op(result_place);
                self.gen_binary_op(operator, result_place, lhs, rhs);
            }
            InstructionKind::UnaryOp { .. } => {
                unimplemented!()
            }
            InstructionKind::Deref { from, to } => {
                self.gen_local_mem_op(to);
                self.deref_value(from, to);
            }
            InstructionKind::Move { from, to } => {
                self.copy_value(from, to);
            }
            InstructionKind::Index { base, index, result_place: place } => {
                self.gen_local_mem_op(place);
                self.gen_index(base, index, place);
            }
        }
    }

    fn gen_index(&mut self, base: &Place, index: &Place, place: &LocalPlace) {
        let base_mem_op = self.gen_mem_op(&base);
        let index_mem_op = self.gen_mem_op(&index);
        let place_mem_op = self.gen_local_mem_op(&place);
        self.push_instruction(X86Instruction::Mov(
            X86Operand::Register(X86Register::RAX),
            base_mem_op,
        ));
        self.push_instruction(X86Instruction::Mov(
            X86Operand::Register(X86Register::RDX),
            index_mem_op,
        ));
        self.push_instruction(X86Instruction::Add(
            X86Operand::Register(X86Register::RAX),
            X86Operand::Register(X86Register::RDX),
        ));
        self.push_instruction(X86Instruction::Mov(
            X86Operand::Register(X86Register::AL),
            X86Operand::Memory {
                base: X86Register::RAX,
                offset: 0,
                size: None,
            },
        ));
        let register = self.get_matching_register(&place_mem_op);
        self.push_instruction(X86Instruction::Mov(
            place_mem_op,
            X86Operand::Register(register),
        ));
    }

    fn deref_value(&mut self, from: &Place, to: &LocalPlace) {
        let from_mem_op = self.gen_mem_op(&from);
        let to_mem_op = self.gen_local_mem_op(&to);
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
                    size: None,
                },
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

    fn copy_value(&mut self, from: &Place, to: &LocalPlace) {
        let from_mem_op = self.gen_mem_op(&from);
        let to_mem_op = self.gen_local_mem_op(&to);
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

    fn gen_value(&mut self, value: &Value, store_at: &LocalPlace) {
        match value {
            Value::I64(value) => {
                let store_at_mem_op = self.gen_local_mem_op(store_at);
                self.push_instruction(X86Instruction::Mov(
                    store_at_mem_op,
                    X86Operand::Immediate(
                        X86Immediate::QWord(*value),
                    ),
                ));
            }
            Value::Char(value) => {
                let store_at_mem_op = self.gen_local_mem_op(store_at);
                self.push_instruction(X86Instruction::Mov(
                    store_at_mem_op,
                    X86Operand::Immediate(
                        X86Immediate::Byte(*value as u8 as i8)
                    ),
                ));
            }
            Value::Bool(value) => {
                let store_at_mem_op = self.gen_local_mem_op(store_at);
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
                    let store_at = LocalPlace::new(store_at.var, offset, value.layout());
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
                let store_at_mem_op = self.gen_local_mem_op(store_at);
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

    fn layout_function_call_args(&mut self, args: &Vec<LocalPlace>) -> u32 {
        let mut arg_size = 0;
        // This will layout the arguments in reversed order on the stack
        for arg in args.iter().rev() {
            let op = self.gen_local_mem_op(arg);
            self.push_instruction(X86Instruction::Push(op));
            arg_size += arg.layout.size;
        }
        arg_size
    }

    fn gen_mem_op(&mut self, place: &Place) -> X86Operand {
        match place {
            Place::Local(place) => self.gen_local_mem_op(place),
            Place::Global(place) => self.gen_global_mem_op(place),
        }
    }

    fn gen_global_mem_op(&mut self, place: &GlobalPlace) -> X86Operand {
        X86Operand::Global {
            label: place.label.clone(),
        }
    }

    fn gen_local_mem_op(&mut self, place: &LocalPlace) -> X86Operand {
        if !self.var_addr.contains_key(&place.var) {
            let size = place.layout.size;
            // self.push_instruction(X86Instruction::Sub(
            //     X86Operand::Register(X86Register::RSP),
            //     X86Operand::Immediate(X86Immediate::QWord(size as i64)),
            // ));
            self.base_pointer_offset += size;
            self.var_addr.insert(place.var, self.base_pointer_offset as i32);
        }
        let offset = self.get_place_offset(place) + place.offset as i32;
        X86Operand::Memory {
            base: X86Register::RBP,
            offset: -(offset as i32),
            size: X86Size::from_layout(&place.layout),
        }
    }

    fn get_place_offset(&mut self, place: &LocalPlace) -> i32 {
        let var = &place.var;
        self.var_addr[var]
    }

    fn push_instruction(&mut self, instruction: X86Instruction) {
        self.asm.push_str("    ");
        self.asm.push_str(format!("{}", instruction).as_str());
        self.asm.push_str("\n");
    }
    fn gen_binary_op(&mut self, op: &HIRBinaryOperator, store_at: &LocalPlace, lhs: &Value, rhs: &Value) {
        let lhs_op = self.gen_value_op(lhs);
        let rhs_op = self.gen_value_op(rhs);
        let store_at_op = self.gen_local_mem_op(store_at);
        match op {
            HIRBinaryOperator::Add
            | HIRBinaryOperator::Subtract
            | HIRBinaryOperator::Multiply
            | HIRBinaryOperator::Divide
            => {
                self.gen_arithmetic_op(op, lhs_op, rhs_op, store_at_op);
            }
            HIRBinaryOperator::Equals
            | HIRBinaryOperator::NotEquals
            | HIRBinaryOperator::LessThan
            | HIRBinaryOperator::LessThanOrEqual
            | HIRBinaryOperator::GreaterThan
            | HIRBinaryOperator::GreaterThanOrEqual
            => {
                self.gen_comp_op(op, store_at, lhs, rhs);
            }
            _ => unimplemented!("Binary operator {:?} not implemented", op),
        }
    }

    fn gen_comp_op(&mut self, op: &HIRBinaryOperator, store_at: &LocalPlace, lhs: &Value, rhs: &Value) {
        let lhs_op = self.gen_value_op(lhs);
        let rhs_op = self.gen_value_op(rhs);
        let store_at_op = self.gen_local_mem_op(store_at);
        let source = X86Operand::Register(self.get_matching_register(&lhs_op));
        self.push_instruction(X86Instruction::Mov(
            source.clone(),
            lhs_op,
        ));
        self.push_instruction(X86Instruction::Cmp(
            source,
            rhs_op,
        ));
        let comp_instruction =
            match op {
                HIRBinaryOperator::Equals => {
                    X86Instruction::Sete(
                        X86Register::AL,
                    )
                }
                HIRBinaryOperator::NotEquals => {
                    X86Instruction::Setne(
                        X86Register::AL,
                    )
                }
                HIRBinaryOperator::LessThan => {
                    X86Instruction::Setl(
                        X86Register::AL,
                    )
                }
                HIRBinaryOperator::LessThanOrEqual => {
                    X86Instruction::Setle(
                        X86Register::AL,
                    )
                }
                _ => unimplemented!("Comp operator {:?} not implemented", op),
            };
        self.push_instruction(comp_instruction);
        self.push_instruction(X86Instruction::Mov(
            store_at_op,
            X86Operand::Register(X86Register::AL),
        ));
    }

    fn get_matching_register(&self, op: &X86Operand) -> X86Register {
        match op {
            X86Operand::Memory { size, .. } => {
                match size {
                    None => {
                        X86Register::RAX
                    }
                    Some(size) => {
                        match size {
                            X86Size::Byte => X86Register::AL,
                            X86Size::Word => X86Register::AX,
                            X86Size::DWord => X86Register::EAX,
                            X86Size::QWord => X86Register::RAX,
                        }
                    }
                }
            }
            _ => unimplemented!("Operand {:?} not implemented", op),
        }
    }

    fn gen_arithmetic_op(&mut self, op: &HIRBinaryOperator, lhs_op: X86Operand, rhs_op: X86Operand, store_at_op: X86Operand) {
        let dest = X86Operand::Register(X86Register::RAX);
        self.push_instruction(X86Instruction::Mov(
            dest.clone(),
            lhs_op,
        ));
        let arithmetic_instruction = match op {
            HIRBinaryOperator::Add => X86Instruction::Add(
                dest.clone(),
                rhs_op,
            ),
            HIRBinaryOperator::Subtract => X86Instruction::Sub(
                dest.clone(),
                rhs_op,
            ),
            HIRBinaryOperator::Multiply => X86Instruction::Mul(
                rhs_op,
            ),
            HIRBinaryOperator::Divide => {
                self.push_instruction(X86Instruction::Cqo);
                X86Instruction::Div(
                    rhs_op,
                )
            }
            _ => unimplemented!("Binary operator {:?} not implemented", op),
        };
        self.push_instruction(arithmetic_instruction);
        self.push_instruction(X86Instruction::Mov(
            store_at_op,
            dest,
        ));
    }

    fn gen_value_op(&mut self, value: &Value) -> X86Operand {
        match value {
            Value::I64(value) => {
                X86Operand::Immediate(X86Immediate::QWord(*value))
            }
            Value::Char(value) => {
                X86Operand::Immediate(X86Immediate::Byte(*value as u8 as i8))
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