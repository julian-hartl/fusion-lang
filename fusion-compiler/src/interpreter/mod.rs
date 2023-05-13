use std::collections::HashMap;

use crate::hir::{FunctionId, HIRBinaryOperator, HIRBinaryOperatorVisitor, HIRUnaryOperator, Scope};
use crate::mir::{BasicBlock, Body, InstructionKind, Label, MemoryPointer, MIR, Primary, TerminatorKind};
use crate::typings::{Layout, Type};

fn as_i64(value: &[u8]) -> i64 {
    i64::from_le_bytes(value.try_into().unwrap())
}

fn as_bool(value: &[u8]) -> bool {
    value[0] != 0
}

fn as_char(value: &[u8]) -> char {
    value[0] as char
}

fn from_bool(value: bool) -> Vec<u8> {
    vec![value as u8]
}

fn resolve_value_by_type(ty: &Type, value: &Vec<u8>) -> Vec<u8> {
    let value = match ty {
        Type::I64 =>
            as_i64(&value).to_le_bytes().to_vec(),

        Type::Bool => {
            from_bool(as_bool(&value))
        }
        Type::Char => {
            vec![as_char(&value) as u8]
        }
        Type::Void => {
            Vec::new()
        }
        Type::Ptr(_,_) => {
            Ptr {
                ptr: as_i64(&value) as usize,
            }.as_bytes().to_vec()
        }
        Type::Function(_) => {
            unimplemented!()
        }
        Type::Unresolved => {
            unreachable!()
        }
        Type::Error => {
            unreachable!()
        }
    };
    value
}


struct BinaryOpInterpreter {
    lhs: Vec<u8>,
    rhs: Vec<u8>,
}

impl HIRBinaryOperatorVisitor<Vec<u8>> for BinaryOpInterpreter {
    fn visit_i64_add(&self) -> Vec<u8> {
        let lhs = as_i64(&self.lhs);
        let rhs = as_i64(&self.rhs);
        let result = lhs + rhs;
        result.to_le_bytes().to_vec()
    }

    fn visit_ptr_i64_add(&self, inner_type: &Type) -> Vec<u8> {
        let lhs = as_i64(&self.lhs);
        let rhs = as_i64(&self.rhs);
        let result = lhs + rhs;
        result.to_le_bytes().to_vec()
    }

    fn visit_char_add(&self) -> Vec<u8> {
        let lhs = as_char(&self.lhs);
        let rhs = as_char(&self.rhs);
        let result = lhs as u8 + rhs as u8;
        vec![result]
    }

    fn visit_i64_subtract(&self) -> Vec<u8> {
        let lhs = as_i64(&self.lhs);
        let rhs = as_i64(&self.rhs);
        let result = lhs - rhs;
        result.to_le_bytes().to_vec()
    }

    fn visit_ptr_i64_subtract(&self, inner_type: &Type) -> Vec<u8> {
        let lhs = as_i64(&self.lhs);
        let rhs = as_i64(&self.rhs);
        let result = lhs - rhs;
        result.to_le_bytes().to_vec()
    }

    fn visit_char_subtract(&self) -> Vec<u8> {
        let lhs = as_char(&self.lhs);
        let rhs = as_char(&self.rhs);
        let result = lhs as u8 - rhs as u8;
        vec![result]
    }

    fn visit_i64_multiply(&self) -> Vec<u8> {
        todo!()
    }

    fn visit_char_multiply(&self) -> Vec<u8> {
        todo!()
    }

    fn visit_i64_divide(&self) -> Vec<u8> {
        let lhs = as_i64(&self.lhs);
        let rhs = as_i64(&self.rhs);
        let result = lhs / rhs;
        result.to_le_bytes().to_vec()
    }

    fn visit_char_divide(&self) -> Vec<u8> {
        todo!()
    }

    fn visit_equals(&self, lhs: &Type, rhs: &Type) -> Vec<u8> {
        let lhs = resolve_value_by_type(lhs, &self.lhs);
        let rhs = resolve_value_by_type(rhs, &self.rhs);
        let result = lhs == rhs;
        from_bool(result)
    }

    fn visit_not_equals(&self, lhs: &Type, rhs: &Type) -> Vec<u8> {
        let lhs = resolve_value_by_type(lhs, &self.lhs);
        let rhs = resolve_value_by_type(rhs, &self.rhs);
        let result = lhs != rhs;
        from_bool(result)
    }

    fn visit_i64_less_than(&self) -> Vec<u8> {
        let lhs = as_i64(&self.lhs);
        let rhs = as_i64(&self.rhs);
        let result = lhs < rhs;
        from_bool(result)
    }

    fn visit_char_less_than(&self) -> Vec<u8> {
        todo!()
    }

    fn visit_i64_less_than_or_equal(&self) -> Vec<u8> {
        todo!()
    }

    fn visit_char_less_than_or_equal(&self) -> Vec<u8> {
        todo!()
    }

    fn visit_i64_greater_than(&self) -> Vec<u8> {
        todo!()
    }

    fn visit_char_greater_than(&self) -> Vec<u8> {
        todo!()
    }

    fn visit_i64_greater_than_or_equal(&self) -> Vec<u8> {
        todo!()
    }

    fn visit_char_greater_than_or_equal(&self) -> Vec<u8> {
        todo!()
    }

    fn visit_i64_bitwise_and(&self) -> Vec<u8> {
        todo!()
    }

    fn visit_char_bitwise_and(&self) -> Vec<u8> {
        todo!()
    }

    fn visit_i64_bitwise_or(&self) -> Vec<u8> {
        todo!()
    }

    fn visit_char_bitwise_or(&self) -> Vec<u8> {
        todo!()
    }

    fn visit_i64_bitwise_xor(&self) -> Vec<u8> {
        todo!()
    }

    fn visit_char_bitwise_xor(&self) -> Vec<u8> {
        todo!()
    }

    fn visit_i64_modulo(&self) -> Vec<u8> {
        let lhs = as_i64(&self.lhs);
        let rhs = as_i64(&self.rhs);
        let result = lhs % rhs;
        result.to_le_bytes().to_vec()
    }

    fn visit_char_modulo(&self) -> Vec<u8> {
        todo!()
    }
}

struct VarMeta {
    ptr: Ptr,
    size: usize,
}

impl VarMeta {
    fn new(ptr: Ptr, size: usize) -> Self {
        Self {
            ptr,
            size,
        }
    }
}


struct InterpreterStackFrame {
    meta: HashMap<usize, VarMeta>,
    base_pointer: usize,
}

struct InterpreterState {
    memory: Vec<u8>,
    stack_pointer: usize,
    stack_frames: Vec<InterpreterStackFrame>,
    heap: Heap,
}

#[derive(Debug, Clone, Copy)]
struct Ptr {
    ptr: usize,
}

impl TryFrom<&[u8]> for Ptr {
    type Error = String;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() != 8 {
            return Err(format!("Invalid pointer size: {}", value.len()));
        }
        let mut bytes = [0; 8];
        bytes.copy_from_slice(value);
        Ok(Self {
            ptr: i64::from_le_bytes(bytes) as usize
        })
    }
}

impl TryFrom<Vec<u8>> for Ptr {
    type Error = String;

    fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
        Self::try_from(&value[..])
    }
}

impl Ptr {
    fn as_raw(&self) -> usize {
        self.ptr
    }

    fn as_bytes(&self) -> [u8; 8] {
        (self.ptr as i64).to_le_bytes()
    }

    fn step(&mut self, size: usize) {
        self.ptr += size;
    }
}

impl InterpreterState {
    const STACK_SIZE: usize = 1024;
    const HEAP_SIZE: usize = 1024 * 1024;
    pub fn new() -> Self {
        Self {
            memory: vec![0; Self::STACK_SIZE + Self::HEAP_SIZE],
            stack_pointer: 0,
            stack_frames: Vec::new(),
            heap: Heap::new(
                Self::HEAP_SIZE
            ),
        }
    }

    pub fn push(&mut self, value: &[u8]) -> Ptr {
        self.memory[self.stack_pointer..self.stack_pointer + value.len()].copy_from_slice(value);
        self.stack_pointer += value.len();
        Ptr {
            ptr: self.stack_pointer - value.len()
        }
    }

    pub fn pop(&mut self) -> u8 {
        self.stack_pointer -= 1;
        self.memory[self.stack_pointer]
    }

    pub fn get(&self, ptr: &Ptr, size: usize) -> &[u8] {
        &self.memory[ptr.as_raw()..ptr.as_raw() + size]
    }

    pub fn store(&mut self, ptr: &Ptr, value: &[u8]) {
        for (i, byte) in value.iter().enumerate() {
            self.memory[ptr.as_raw() + i] = *byte;
        }
    }

    pub fn get_var(&self, var: usize) -> &[u8] {
        let meta = self.get_var_meta(var);
        self.get(&meta.ptr, meta.size)
    }

    pub fn get_var_meta(&self, var: usize) -> &VarMeta {
        self.current_frame().meta.get(&var).expect(format!("Variable {} not found", var).as_str())
    }

    pub fn get_var_address(&self, var: usize) -> &Ptr {
        &self.get_var_meta(var).ptr
    }

    pub fn save_var(&mut self, var: usize, size: usize, value: &[u8]) {
        assert!(size <= Layout::POINTER_SIZE);
        match self.current_frame().meta.get(&var).map(|meta| meta.ptr) {
            Some(address) => {
                self.store(&address, value);
            }
            None => {
                let address = self.push(value);
                self.current_frame_mut().meta.insert(var, VarMeta::new(address, size));
            }
        }
    }

    pub fn get_string(&self, ptr: &Ptr) -> String {
        let mut result = String::new();
        let mut ptr = ptr.clone();
        loop {
            let byte = self.get(&ptr, 1)[0];
            if byte == 0 {
                break;
            }
            result.push(byte as char);
            ptr.step(1);
        }
        result
    }

    pub fn push_str(&mut self, string: &str) -> Ptr {
        let mut bytes = string.as_bytes().to_vec();
        bytes.push(0);
        let ptr = self.push(&bytes);
        ptr
    }

    pub fn push_char(&mut self, value: char) -> Ptr {
        self.push(&[value as u8])
    }

    pub fn push_bool(&mut self, value: bool) -> Ptr {
        self.push(&[value as u8])
    }

    pub fn push_i64(&mut self, value: i64) -> Ptr {
        self.push(&value.to_le_bytes())
    }

    pub fn push_stack_frame(&mut self) {
        self.stack_frames.push(InterpreterStackFrame {
            meta: HashMap::new(),
            base_pointer: self.stack_pointer,
        });
    }

    pub fn pop_stack_frame(&mut self) {
        let frame = self.stack_frames.pop().unwrap();
        while self.stack_pointer > frame.base_pointer {
            self.pop();
        }
    }

    fn current_frame(&self) -> &InterpreterStackFrame {
        self.stack_frames.last().unwrap()
    }

    fn current_frame_mut(&mut self) -> &mut InterpreterStackFrame {
        self.stack_frames.last_mut().unwrap()
    }

    pub fn malloc(&mut self, size: usize) -> Ptr {
        let ptr = self.heap.allocate(size).expect("Out of memory");
        Ptr {
            ptr: ptr.as_raw() + Self::STACK_SIZE
        }
    }

    pub fn free(&mut self, ptr: &Ptr) {
        self.heap.free(&Ptr {
            ptr: ptr.as_raw() - Self::STACK_SIZE
        }).expect("Tried to free unallocated memory");
    }
}

struct Heap {
    size: usize,
    allocated_blocks: Vec<HeapBlock>,
}

struct HeapBlock {
    start: usize,
    size: usize,
}

impl Heap {
    pub fn new(size: usize) -> Self {
        Self {
            size,
            allocated_blocks: Vec::new(),
        }
    }

    pub fn allocate(&mut self, size: usize) -> Option<Ptr> {
        let mut start = 0;
        for block in &self.allocated_blocks {
            if block.start - start >= size {
                let ptr = Ptr {
                    ptr: start,
                };
                self.allocated_blocks.push(HeapBlock {
                    start,
                    size,
                });
                return Some(ptr);
            }
            start = block.start + block.size;
        }
        if self.size - start >= size {
            let ptr = Ptr {
                ptr: start,
            };
            self.allocated_blocks.push(HeapBlock {
                start,
                size,
            });
            return Some(ptr);
        }
        None
    }

    pub fn free(&mut self, ptr: &Ptr) -> Option<()> {
        let index = self.allocated_blocks.iter().position(|block| block.start == ptr.as_raw())?;
        self.allocated_blocks.remove(index);
        Some(())
    }
}

pub struct Interpreter<'a> {
    mir: &'a MIR,
    scope: &'a Scope,
    state: InterpreterState,
}

impl<'a> Interpreter<'a> {
    pub fn new(mir: &'a MIR, scope: &'a Scope) -> Self {
        Self {
            mir,
            state: InterpreterState::new(),
            scope,
        }
    }

    pub fn print_stats(&self) {
        // for (address, value) in self.state.memory[InterpreterState::STACK_SIZE..InterpreterState::STACK_SIZE + 20].iter().enumerate() {
        //     println!("{}: {}", address + InterpreterState::STACK_SIZE, value);
        // }
        println!();
        println!("Stack pointer: {}", self.state.stack_pointer);
        println!("Stack frames: {}", self.state.stack_frames.len());
    }

    pub fn interpret(&mut self) {
        let main = self.mir.bodies.iter()
            .find(|body| self.scope.get_function(&body.function).name == "main")
            .expect("Failed to find main function");
        self.state.push_stack_frame();
        self.interpret_body(&main);
        self.print_stats();
    }

    fn interpret_body(&mut self, main: &Body) -> Vec<u8> {
        if main.basic_blocks.is_empty() {
            panic!("Function {} has no basic blocks", main.function.index);
        }
        let mut current_block = &main.basic_blocks[0];
        loop {
            for instruction in current_block.instructions.iter() {
                let (return_value, ty) = match &instruction.kind {
                    InstructionKind::Store(ptr, primary) => {
                        let value = self.interpret_primary(primary);
                        let ty = match ptr {
                            MemoryPointer::Variable(variable, ty) => {
                                self.state.save_var(*variable, ty.layout().size, &value);
                                ty.clone()
                            }
                            MemoryPointer::Primary(raw_ptr, ty) => {
                                let raw_ptr: Ptr = self.interpret_primary(raw_ptr).try_into().unwrap();
                                self.state.store(&raw_ptr, &value);
                                ty.clone()
                            }
                        };
                        (value, ty)
                    }
                    InstructionKind::Call(function_id, args) => {
                        let function_body = self.mir.bodies.iter()
                            .find(|body| body.function == *function_id)
                            .expect("Failed to find function");
                        let function = self.scope.get_function(&function_body.function);
                        let arg_values = args.iter()
                            .map(|arg| self.interpret_primary(arg))
                            .collect::<Vec<_>>();
                        self.state.push_stack_frame();
                        for (index, (parameter_id, value)) in function.parameters.iter().zip(arg_values.iter()).enumerate() {
                            let parameter = self.scope.get_variable(parameter_id);
                            let argument_size = parameter.ty.layout().size;
                            self.state.save_var(index, argument_size, &value);
                        }
                        let function_name = &self.scope.get_function(function_id).name;
                        let return_value = match function_name.as_str() {
                            "printf" => {
                                let ptr: Ptr = self.state.get_var(0).try_into().unwrap();
                                let str = self.state.get_string(&ptr);
                                print!("{}", str);
                                Vec::new()
                            }
                            "malloc" => {
                                let size = as_i64(self.state.get_var(0)) as usize;
                                let ptr = self.state.malloc(size);
                                ptr.as_bytes().to_vec()
                            }
                            "free" => {
                                let ptr: Ptr = self.state.get_var(0).try_into().unwrap();
                                self.state.free(&ptr);
                                Vec::new()
                            }
                            _ => {
                                self.interpret_body(function_body)
                            }
                        };
                        self.state.pop_stack_frame();
                        (return_value, function.return_type.clone())
                    }
                    InstructionKind::Primary(primary) => {
                        let value = self.interpret_primary(primary);
                        (value, primary.ty())
                    }
                    InstructionKind::BinaryOp(op, lhs, rhs, ty) => {
                        let lhs_value = self.interpret_primary(&lhs.0);
                        let rhs_value = self.interpret_primary(&rhs.0);
                        let bin_op_interpreter = BinaryOpInterpreter {
                            lhs: lhs_value,
                            rhs: rhs_value,
                        };
                        let value = op.visit(&bin_op_interpreter, &lhs.1, &rhs.1).expect("Failed to interpret binary operation");
                        (value, ty.clone())
                    }
                    InstructionKind::UnaryOp(op, operand, ty) => {
                        let operand = self.interpret_primary(operand);
                        let operand = as_i64(&operand);
                        let value = match op {
                            HIRUnaryOperator::Negate => {
                                -operand
                            }
                            HIRUnaryOperator::BitwiseNot => {
                                !operand
                            }
                        }.to_le_bytes().to_vec();
                        (value, ty.clone())
                    }
                    InstructionKind::Load(ptr) => {
                        match ptr {
                            MemoryPointer::Variable(variable, ty) => {
                                (self.state.get_var(*variable).to_vec(), ty.clone())
                            }
                            MemoryPointer::Primary(raw_ptr, ty) => {
                                let raw_ptr: Ptr = self.interpret_primary(raw_ptr).try_into().unwrap();
                                (self.state.get(&raw_ptr, ty.layout().size).to_vec(), ty.clone())
                            }
                        }
                    }
                    InstructionKind::GetAddress(expr) => {
                        let value = match expr {
                            Primary::Variable(variable, _) => {
                                self.state.get_var_address(*variable).as_bytes()
                            }
                            Primary::Bool(value) => {
                                self.state.push_bool(*value).as_bytes()
                            }
                            Primary::I64(value) => {
                                self.state.push_i64(*value).as_bytes()
                            }
                            Primary::Str(value) => {
                                self.state.push_str(value).as_bytes()
                            }
                            Primary::Void => {
                                panic!("Cannot get address of void");
                            }
                            Primary::Char(value) => {
                                self.state.push_char(*value).as_bytes()
                            }
                        }.to_vec();
                        (value, expr.ty())
                    }
                    InstructionKind::Cast(expr, ty) => {
                        let value = self.interpret_primary(expr);
                        let value = resolve_value_by_type(ty, &value);
                        (value, ty.clone())
                    }
                };
                if let Some(assign_to) = instruction.assign_to {
                    self.state.save_var(assign_to, ty.layout().size, &return_value);
                }
            }
            current_block = match &current_block.terminator.kind {
                TerminatorKind::Goto(label) => {
                    main.find_basic_block(label)
                }
                TerminatorKind::If(cond, then, else_) => {
                    let cond = self.interpret_primary(cond);
                    if cond[0] == 1 {
                        main.find_basic_block(then)
                    } else {
                        main.find_basic_block(else_)
                    }
                }
                TerminatorKind::Return(primary) => {
                    return self.interpret_primary(primary);
                }
                TerminatorKind::Next => {
                    main.find_basic_block(&Label::new(current_block.label.index + 1))
                }
            }.unwrap();
        }
    }


    fn interpret_primary(&mut self, primary: &Primary) -> Vec<u8> {
        match primary {
            Primary::I64(value) => {
                value.to_le_bytes().to_vec()
            }
            Primary::Str(name) => {
                self.state.push_str(name).as_bytes().to_vec()
            }
            Primary::Variable(variable, _) => {
                self.state.get_var(*variable).to_vec()
            }
            Primary::Bool(value) => {
                vec![*value as u8]
            }
            Primary::Void => {
                vec![]
            }
            Primary::Char(char) => {
                vec![*char as u8]
            }
        }
    }
}