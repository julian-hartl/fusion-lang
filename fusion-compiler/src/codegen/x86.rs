use std::collections::HashMap;
use std::fmt::{Display, format};

use crate::hir::{Scope, ScopeCell};
use crate::mir::{Instruction, InstructionKind, MIR, Place, Value, Var};


pub struct X86Codegen<'a> {
    base_pointer_offset: u32,
    var_addr: HashMap<Var, u32>,
    mir: &'a MIR,
    scope: ScopeCell,
    asm: String,
}

enum X86Size {
    Byte,
    Word,
    DWord,
    QWord,
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

impl<'a> X86Codegen<'a> {
    pub fn new(
        mir: &'a MIR,
        scope: ScopeCell,
    ) -> Self {
        Self {
            mir,
            base_pointer_offset: 0,
            asm: String::new(),
            scope,
            var_addr: HashMap::new(),
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
        self.push_instruction("mov rbp, rsp");
        self.push_instruction("and rsp, -16");
        self.push_instruction("sub rsp, 8");
        self.push_instruction("call main");
        self.push_instruction("mov rdi, 0");
        self.push_instruction("mov rax, 60");
        self.push_instruction("syscall");
    }

    pub fn gen_body(&mut self, body: &crate::mir::Body) {
        self.var_addr.clear();
        self.base_pointer_offset = INITIAL_BASE_POINTER_OFFSET;
        let scope = self.scope.borrow();
        let function = scope.get_function(&body.function);
        self.asm.push_str(&format!("{}:\n", function.name));
        drop(scope);
        self.push_instruction("push rbp");
        self.mov("rbp", "rsp", None);
        for bb in body.basic_blocks.iter() {
            self.gen_basic_block(bb);
        }
    }

    fn clear_stack_frame(&mut self) {
        self.mov("rsp", "rbp", None);
        self.push_instruction("pop rbp");
        self.push_instruction("ret");
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
                self.asm.push_str(&format!("jmp {}\n", label));
            }
            crate::mir::TerminatorKind::Return(value) => {
                // todo: maybe we should pass the place of the return value to the return terminator so we can gen it here
                self.clear_stack_frame();
            }
            crate::mir::TerminatorKind::If {
                condition,
                then,
                else_,
            } => {
                // todo: maybe do not pass a Value here but rather a new type called condition which can hold bool binary expressions or literal values
                // let condition = self.gen_value(condition);
                self.push_instruction(&format!("cmp 0, 0"));
                self.push_instruction(&format!("je {}", else_));
                self.push_instruction(&format!("jmp {}", then));
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
                let value_size = value.layout().size;
                self.maybe_gen_new_place(place);
                self.gen_value(value, place);
                self.base_pointer_offset += value_size;
            }
            InstructionKind::Call { .. } => {}
            InstructionKind::BinaryOp { .. } => {}
            InstructionKind::UnaryOp { .. } => {}
            InstructionKind::Move { from, to } => {
                let from_place = self.gen_existing_place(from);
                let to_place = self.maybe_gen_new_place(to);
                let size = to.layout.size;
                // todo: only do this if the size is bigger than 8 bytes
                self.push_instruction(&format!("lea rsi, {}", from_place));
                self.push_instruction(&format!("lea rdi, {}", to_place));
                self.mov("rcx", &size.to_string(), None);
                self.push_instruction("rep movsb");
            }
        }
    }

    fn gen_value(&mut self, value: &Value, store_at: &Place) {
        let store_at_place = self.gen_existing_place(store_at);
        match value {
            Value::I64(value) => {
                self.mov(store_at_place.as_str(), &format!("{}", value), Some(X86Size::QWord));
            }
            Value::Char(value) => {
                self.mov(store_at_place.as_str(), &format!("{}",*value as u8), Some(X86Size::Byte));
            }
            Value::String(_) => {
                unimplemented!()
            }
            Value::Bool(value) => {
                self.mov(store_at_place.as_str(), &format!("{}",*value as u8), Some(X86Size::Byte));
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
            Value::Place(place) => {
                let place = self.gen_existing_place(place);
                self.push_instruction(&format!("lea rax, {}", place));
                self.mov(place.as_str(), "rax", None);
            }
        }
    }

    fn add(&mut self, destination: &str, source: i64) {
        self.push_instruction(&format!("add {}, {}", destination, source));
    }

    fn mov(&mut self, destination: &str, source: &str, size: Option<X86Size>) {
        match size {
            Some(size) => {
                self.push_instruction(&format!("mov {} {}, {}", size, destination, source));
            }
            None => {
                self.push_instruction(&format!("mov {}, {}", destination, source));
            }
        }
    }

    fn maybe_gen_new_place(&mut self, place: &Place) -> String {
        if self.var_addr.contains_key(&place.var) {
            self.gen_existing_place(place)
        } else {
            self.gen_new_place(place)
        }
    }

    fn gen_new_place(&mut self, place: &Place) -> String {
        let var = &place.var;
        let offset = self.base_pointer_offset;
        self.var_addr.insert(var.clone(), offset);
        self.push_instruction(&format!("sub rsp, {}", place.layout.size));
        self.gen_existing_place(place)
    }

    fn gen_existing_place(&mut self, place: &Place) -> String {
        let offset = self.get_place_offset(place);
        format!("[rbp-{}]", offset)
    }

    fn get_place_offset(&mut self, place: &Place) -> u32 {
        let var = &place.var;
        self.var_addr[var]
    }

    fn push_instruction(&mut self, instruction: &str) {
        self.asm.push_str("    ");
        self.asm.push_str(instruction);
        self.asm.push_str("\n");
    }
}