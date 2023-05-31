use std::any::Any;
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt::Display;
use std::ops::Deref;

use petgraph::Graph;
use petgraph::graph::{Node, NodeIndex};

use fusion_compiler::{idx, IdxVec};
use fusion_compiler::Idx;

use crate::hir::{FunctionIdx, HIRBinaryOperator};
use crate::mir::{BasicBlock, BasicBlockIdx, BinOp, Body, BodyScope, ConstantValue, GlobalIdx, GlobalValue, Instruction, InstructionKind, Local, LocalIdx, MIR, MIRType, Operand, Place, Projection, Rvalue, Scalar, TerminatorKind, UnOp};
use crate::modules::scopes::GlobalScopeCell;
use crate::modules::symbols::QualifiedName;
use crate::typings::Layout;

idx!(StackFrameBlockIdx);

#[derive(Debug, Eq, PartialEq, Clone)]
struct StackFrameBlock {
    start: u32,
    end: u32,
    idx: StackFrameBlockIdx,
}

impl PartialOrd<Self> for StackFrameBlock {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.start.partial_cmp(&other.start)
    }
}

impl Ord for StackFrameBlock {
    fn cmp(&self, other: &Self) -> Ordering {
        self.start.cmp(&other.start)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
struct StackOffset(u32);


impl StackOffset {
    pub fn to_rbp_offset(&self) -> i32 {
        -(self.0 as i32)
    }

    pub fn add_offset(&mut self, offset: u32) {
        self.0 += offset;
    }
}

#[derive(Debug, Clone)]
struct StackFrame {
    blocks: BTreeSet<StackFrameBlock>,
    current_index: usize,
    stack_pointer: u32,
}

impl StackFrame {
    pub fn new() -> Self {
        StackFrame {
            blocks: BTreeSet::new(),
            current_index: 0,
            stack_pointer: 0,
        }
    }

    pub fn allocate(&mut self, size: u32) -> StackFrameBlockIdx {
        match self.find_free_range(size) {
            None => {
                self.push_block(size)
            }
            Some((start, end)) => {
                let idx = self.next_idx();
                let block = StackFrameBlock { start, end, idx };
                self.blocks.insert(block);
                idx
            }
        }
    }

    pub fn push_block(&mut self, size: u32) -> StackFrameBlockIdx {
        let start = self.get_stack_pointer();
        let end = start + size;
        let idx = self.next_idx();
        self.blocks.insert(StackFrameBlock { start, end, idx });
        self.stack_pointer = end;
        idx
    }

    pub fn pop_block(&mut self) -> Option<StackFrameBlockIdx> {
        let block = self.blocks.iter().last()?;
        let idx = block.idx;
        self.stack_pointer = block.start;
        self.free_block(idx);
        Some(idx)
    }

    pub fn decrease_stack_by(&mut self, size: u32) {
        self.stack_pointer -= size;
        self.blocks.retain(|block| block.end <= self.stack_pointer);
        assert_eq!(self.blocks.iter().last().unwrap().end, self.stack_pointer);
    }

    pub fn get_stack_pointer(&self) -> u32 {
        self.stack_pointer
    }

    pub fn free_block(&mut self, idx: StackFrameBlockIdx) {
        self.blocks.retain(|block| block.idx != idx);
    }

    /// Checks if the stack pointer does not match the actual top of the stack and returns the difference.
    pub fn check_difference(&self) -> Option<u32> {
        let topmost_block_pointer = self.blocks.iter().last()?.end;
        assert!(self.stack_pointer >= topmost_block_pointer, "Stack pointer {} is below topmost block pointer {}", self.stack_pointer, topmost_block_pointer);
        let diff = self.stack_pointer - topmost_block_pointer;
        if diff == 0 {
            None
        } else {
            Some(diff)
        }
    }

    pub fn get_block_offset(&self, idx: StackFrameBlockIdx) -> Option<StackOffset> {
        let block = self.blocks.iter().find(|block| block.idx == idx)?;
        Some(StackOffset(block.start))
    }

    fn find_free_range(&self, size: u32) -> Option<(u32, u32)> {
        for (block, next) in self.blocks.iter().zip(self.blocks.iter().skip(1)) {
            if next.start - block.end >= size {
                return Some((block.end, size));
            }
        }
        None
    }

    fn next_idx(&mut self) -> StackFrameBlockIdx {
        let idx = self.current_index;
        self.current_index += 1;
        StackFrameBlockIdx::new(idx)
    }
}

type InterferenceGraph = Graph<LocalIdx, ()>;

const GENERAL_PURPOSE_REGISTER_COUNT: usize = 14;

static GENERAL_PURPOSE_REGS_64_BIT: &'static [X86Register; GENERAL_PURPOSE_REGISTER_COUNT] = &[
    X86Register::RAX,
    X86Register::RBX,
    X86Register::RCX,
    X86Register::RDX,
    X86Register::RSI,
    X86Register::RDI,
    X86Register::R8,
    X86Register::R9,
    X86Register::R10,
    X86Register::R11,
    X86Register::R12,
    X86Register::R13,
    X86Register::R14,
    X86Register::R15,
];

static GENERAL_PURPOSE_REGS_32_BIT: &'static [X86Register; GENERAL_PURPOSE_REGISTER_COUNT] = &[
    X86Register::EAX,
    X86Register::EBX,
    X86Register::ECX,
    X86Register::EDX,
    X86Register::ESI,
    X86Register::EDI,
    X86Register::R8D,
    X86Register::R9D,
    X86Register::R10D,
    X86Register::R11D,
    X86Register::R12D,
    X86Register::R13D,
    X86Register::R14D,
    X86Register::R15D,
];

static GENERAL_PURPOSE_REGS_16_BIT: &'static [X86Register; GENERAL_PURPOSE_REGISTER_COUNT] = &[
    X86Register::AX,
    X86Register::BX,
    X86Register::CX,
    X86Register::DX,
    X86Register::SI,
    X86Register::DI,
    X86Register::R8W,
    X86Register::R9W,
    X86Register::R10W,
    X86Register::R11W,
    X86Register::R12W,
    X86Register::R13W,
    X86Register::R14W,
    X86Register::R15W,
];

static GENERAL_PURPOSE_REGS_8_BIT: &'static [X86Register; GENERAL_PURPOSE_REGISTER_COUNT] = &[
    X86Register::AL,
    X86Register::BL,
    X86Register::CL,
    X86Register::DL,
    X86Register::SIL,
    X86Register::DIL,
    X86Register::R8B,
    X86Register::R9B,
    X86Register::R10B,
    X86Register::R11B,
    X86Register::R12B,
    X86Register::R13B,
    X86Register::R14B,
    X86Register::R15B,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RegisterColor {
    color: u8,
}

impl RegisterColor {
    pub fn new(color: u8) -> Self {
        RegisterColor { color }
    }

    pub fn from_register(register: X86Register) -> Self {
        for (i, reg) in GENERAL_PURPOSE_REGS_64_BIT.iter().enumerate() {
            if *reg == register {
                return RegisterColor::new(i as u8);
            }
        }

        for (i, reg) in GENERAL_PURPOSE_REGS_32_BIT.iter().enumerate() {
            if *reg == register {
                return RegisterColor::new(i as u8);
            }
        }

        for (i, reg) in GENERAL_PURPOSE_REGS_16_BIT.iter().enumerate() {
            if *reg == register {
                return RegisterColor::new(i as u8);
            }
        }

        for (i, reg) in GENERAL_PURPOSE_REGS_8_BIT.iter().enumerate() {
            if *reg == register {
                return RegisterColor::new(i as u8);
            }
        }

        panic!("Register {:?} is not a general purpose register", register);
    }

    pub fn into_register(self, size: X86Size) -> X86Register {
        match size {
            X86Size::QWord => GENERAL_PURPOSE_REGS_64_BIT[self.color as usize],
            X86Size::DWord => GENERAL_PURPOSE_REGS_32_BIT[self.color as usize],
            X86Size::Word => GENERAL_PURPOSE_REGS_16_BIT[self.color as usize],
            X86Size::Byte => GENERAL_PURPOSE_REGS_8_BIT[self.color as usize],
        }
    }
}

struct MemoryLocationAllocator {
    locals: HashMap<LocalIdx, PlaceLocation>,
    interference_graph: InterferenceGraph,
    color_map: HashMap<NodeIndex, RegisterColor>,
    parameters: Vec<LocalIdx>,
    stack: StackFrame,
    alive_locals: HashSet<LocalIdx>,
    temp_registers: HashSet<X86Register>,
}

impl MemoryLocationAllocator {
    pub fn new(
        parameters: Vec<LocalIdx>,
    ) -> Self {
        MemoryLocationAllocator {
            temp_registers: HashSet::new(),
            parameters,
            locals: HashMap::new(),
            interference_graph: InterferenceGraph::new(),
            color_map: HashMap::new(),
            stack: StackFrame::new(),
            alive_locals: HashSet::new(),
        }
    }

    pub fn add_local(&mut self, place: LocalIdx) {
        let index = self.interference_graph.add_node(place);
        if place == Place::RETURN_PLACE {
            self.color_map.insert(index, RegisterColor::from_register(X86Register::RAX));
        }
    }

    pub fn add_interference(&mut self, a: &LocalIdx, b: &LocalIdx) {
        let a = self.find_local(a);
        let b = self.find_local(b);
        self.interference_graph.add_edge(a, b, ());
        self.interference_graph.add_edge(b, a, ());
    }

    pub fn get_location(&self, var: &LocalIdx) -> &PlaceLocation {
        self.locals.get(var).unwrap()
    }

    pub fn allocate(&mut self, body_scope: &BodyScope) {
        self.color_parameters();
        self.color_graph(body_scope);
        self.allocate_locations(body_scope);
    }

    fn color_parameters(&mut self) {
        let parameter_regs = [
            X86Register::RDI,
            X86Register::RSI,
            X86Register::RDX,
            X86Register::RCX,
            X86Register::R8,
            X86Register::R9,
        ];
        for (i, place) in self.parameters.clone().into_iter().enumerate() {
            // todo: check if parameters should be colored (e.g. if they are structs)
            self.color_map.insert(self.find_local(&place), RegisterColor::from_register(parameter_regs[i]));
        }
    }

    fn color_graph(&mut self, body_scope: &BodyScope) {
        let mut stack = vec![];

        while let Some(node) = self.find_node_to_remove(&stack) {
            stack.push(node);
        }

        while let Some(node) = stack.pop() {
            let local = &self.interference_graph[node];
            let local_type = &body_scope.locals.get(*local).ty;
            // Don't color structs because they are not allocated in registers
            if let MIRType::Struct(_) = local_type {
                continue;
            }
            let neighbors = self.interference_graph.neighbors(node).collect::<Vec<_>>();
            let used_colors = neighbors.iter().filter_map(|n| self.color_map.get(n).map(|c| c.color)).collect::<Vec<_>>();

            for color in 0..self.k() {
                if !used_colors.contains(&color) {
                    self.color_map.insert(node, RegisterColor::new(color));
                    break;
                }
            }
        }
    }

    fn allocate_locations(&mut self, body_scope: &BodyScope) {
        for node in self.interference_graph.node_indices() {
            let local_idx = self.interference_graph[node].clone();
            let local_layout = body_scope.locals.get(local_idx).ty.layout();
            match self.color_map.get(&node) {
                Some(color) => {
                    let size = X86Size::from(&local_layout);
                    let register = color.into_register(size);
                    self.use_mapped_register(local_idx, register);
                }
                None => {}
            }
        }
    }

    fn get_free_register(&self, size: X86Size) -> Option<X86Register> {
        let registers = X86Register::get_register_list(size);
        for register in registers {
            if self.is_register_free(*register) {
                return Some(*register);
            }
        }
        None
    }


    pub fn mark_local_as_alive(&mut self, idx: LocalIdx, local_layout: Layout) {
        self.alive_locals.insert(idx);
        match self.locals.get(&idx) {
            Some(PlaceLocation::Stack(_)) => {}
            Some(PlaceLocation::Register(_)) => {}
            None => {
                self.allocate_on_stack(idx, local_layout);
            }
        }
    }

    fn allocate_on_stack(&mut self, idx: LocalIdx, local_layout: Layout) {
        let block = self.stack.allocate(local_layout.size);
        self.locals.insert(idx, PlaceLocation::Stack(block));
    }

    pub fn get_block_offset(&self, block: StackFrameBlockIdx) -> StackOffset {
        self.stack.get_block_offset(block).expect(format!("Block {:?} not found", block).as_str())
    }

    pub fn mark_variable_as_dead(&mut self, local: LocalIdx) {
        self.alive_locals.remove(&local);
        match self.get_location(&local) {
            PlaceLocation::Stack(block) => {
                self.stack.free_block(*block);
            }
            PlaceLocation::Register(_) => {}
        }
    }

    fn is_register_free(&self, register: X86Register) -> bool {
        let index = register.index();
        let registers_to_consider = [
            &GENERAL_PURPOSE_REGS_8_BIT[index],
            &GENERAL_PURPOSE_REGS_16_BIT[index],
            &GENERAL_PURPOSE_REGS_32_BIT[index],
            &GENERAL_PURPOSE_REGS_64_BIT[index],
        ];
        registers_to_consider.iter().all(|r| !self.temp_registers.contains(r) && !self.is_register_used(**r))
    }

    fn is_register_used(&self, register: X86Register) -> bool {
        self.alive_locals.iter().any(|var| {
            if let Some(PlaceLocation::Register(r)) = self.locals.get(var) {
                *r == register
            } else {
                false
            }
        })
    }

    fn use_mapped_register(&mut self, local: LocalIdx, register: X86Register) {
        self.locals.insert(local, PlaceLocation::Register(register));
    }


    fn find_local(&self, local: &LocalIdx) -> NodeIndex {
        self.interference_graph.node_indices().find(|i| self.interference_graph[*i] == *local).expect(format!("Local {:?} not found in interference graph", local).as_str())
    }

    fn k(&self) -> u8 {
        GENERAL_PURPOSE_REGISTER_COUNT as u8
    }

    fn find_node_to_remove(&self, removed_nodes: &Vec<NodeIndex>) -> Option<NodeIndex> {
        self.interference_graph.node_indices()
            .find(|i| !removed_nodes.contains(i) && self.interference_graph.edges(*i).count() < self.k() as usize && i.index() > self.parameters.len())
    }
}


#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[repr(u8)]
enum X86Register {
    // 8-bit
    AL,
    AH,
    BL,
    BH,
    CL,
    CH,
    DL,
    DH,
    SIL,
    DIL,
    BPL,
    SPL,
    R8B,
    R9B,
    R10B,
    R11B,
    R12B,
    R13B,
    R14B,
    R15B,

    // 16-bit
    AX,
    BX,
    CX,
    DX,
    SI,
    DI,
    BP,
    SP,
    R8W,
    R9W,
    R10W,
    R11W,
    R12W,
    R13W,
    R14W,
    R15W,

    // 32-bit
    EAX,
    EBX,
    ECX,
    EDX,
    ESI,
    EDI,
    EBP,
    ESP,
    R8D,
    R9D,
    R10D,
    R11D,
    R12D,
    R13D,
    R14D,
    R15D,

    // 64-bit
    RAX,
    RBX,
    RCX,
    RDX,
    RSI,
    RDI,
    RBP,
    RSP,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
}

impl X86Register {
    pub fn size(&self) -> X86Size {
        match self {
            X86Register::AL | X86Register::AH | X86Register::BL | X86Register::BH
            | X86Register::CL | X86Register::CH | X86Register::DL | X86Register::DH
            | X86Register::SIL | X86Register::DIL | X86Register::BPL | X86Register::SPL
            | X86Register::R8B | X86Register::R9B | X86Register::R10B | X86Register::R11B
            | X86Register::R12B | X86Register::R13B | X86Register::R14B | X86Register::R15B => X86Size::Byte,
            X86Register::AX | X86Register::BX | X86Register::CX | X86Register::DX
            | X86Register::SI | X86Register::DI | X86Register::BP | X86Register::SP
            | X86Register::R8W | X86Register::R9W | X86Register::R10W | X86Register::R11W
            | X86Register::R12W | X86Register::R13W | X86Register::R14W | X86Register::R15W => X86Size::Word,
            X86Register::EAX | X86Register::EBX | X86Register::ECX | X86Register::EDX
            | X86Register::ESI | X86Register::EDI | X86Register::EBP | X86Register::ESP
            | X86Register::R8D | X86Register::R9D | X86Register::R10D | X86Register::R11D
            | X86Register::R12D | X86Register::R13D | X86Register::R14D | X86Register::R15D => X86Size::DWord,
            X86Register::RAX | X86Register::RBX | X86Register::RCX | X86Register::RDX
            | X86Register::RSI | X86Register::RDI | X86Register::RBP | X86Register::RSP
            | X86Register::R8 | X86Register::R9 | X86Register::R10 | X86Register::R11
            | X86Register::R12 | X86Register::R13 | X86Register::R14 | X86Register::R15 => X86Size::QWord,
        }
    }

    pub fn get_register_list(size: X86Size) -> &'static [X86Register; GENERAL_PURPOSE_REGISTER_COUNT] {
        match size {
            X86Size::QWord => GENERAL_PURPOSE_REGS_64_BIT,
            X86Size::DWord => GENERAL_PURPOSE_REGS_32_BIT,
            X86Size::Word => GENERAL_PURPOSE_REGS_16_BIT,
            X86Size::Byte => GENERAL_PURPOSE_REGS_8_BIT,
        }
    }

    pub fn index(&self) -> usize {
        Self::get_register_list(self.size()).iter()
            .enumerate().find(|(_, r)| *r == self).map(|(i, _)| i).expect("Register not found in register list")
    }

    pub fn resize(self, size: &X86Size) -> X86Register {
        let index = self.index();
        match size {
            X86Size::Byte =>
                GENERAL_PURPOSE_REGS_8_BIT[index],
            X86Size::Word =>
                GENERAL_PURPOSE_REGS_16_BIT[index],
            X86Size::DWord =>
                GENERAL_PURPOSE_REGS_32_BIT[index],
            X86Size::QWord =>
                GENERAL_PURPOSE_REGS_64_BIT[index],
        }
    }
}


impl Display for X86Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // 8-bit
            X86Register::AL => write!(f, "al"),
            X86Register::AH => write!(f, "ah"),
            X86Register::BL => write!(f, "bl"),
            X86Register::BH => write!(f, "bh"),
            X86Register::CL => write!(f, "cl"),
            X86Register::CH => write!(f, "ch"),
            X86Register::DL => write!(f, "dl"),
            X86Register::DH => write!(f, "dh"),
            X86Register::SIL => write!(f, "sil"),
            X86Register::DIL => write!(f, "dil"),
            X86Register::BPL => write!(f, "bpl"),
            X86Register::SPL => write!(f, "spl"),
            X86Register::R8B => write!(f, "r8b"),
            X86Register::R9B => write!(f, "r9b"),
            X86Register::R10B => write!(f, "r10b"),
            X86Register::R11B => write!(f, "r11b"),
            X86Register::R12B => write!(f, "r12b"),
            X86Register::R13B => write!(f, "r13b"),
            X86Register::R14B => write!(f, "r14b"),
            X86Register::R15B => write!(f, "r15b"),

            // 16-bit
            X86Register::AX => write!(f, "ax"),
            X86Register::BX => write!(f, "bx"),
            X86Register::CX => write!(f, "cx"),
            X86Register::DX => write!(f, "dx"),
            X86Register::SI => write!(f, "si"),
            X86Register::DI => write!(f, "di"),
            X86Register::BP => write!(f, "bp"),
            X86Register::SP => write!(f, "sp"),
            X86Register::R8W => write!(f, "r8w"),
            X86Register::R9W => write!(f, "r9w"),
            X86Register::R10W => write!(f, "r10w"),
            X86Register::R11W => write!(f, "r11w"),
            X86Register::R12W => write!(f, "r12w"),
            X86Register::R13W => write!(f, "r13w"),
            X86Register::R14W => write!(f, "r14w"),
            X86Register::R15W => write!(f, "r15w"),

            // 32-bit
            X86Register::EAX => write!(f, "eax"),
            X86Register::EBX => write!(f, "ebx"),
            X86Register::ECX => write!(f, "ecx"),
            X86Register::EDX => write!(f, "edx"),
            X86Register::ESI => write!(f, "esi"),
            X86Register::EDI => write!(f, "edi"),
            X86Register::EBP => write!(f, "ebp"),
            X86Register::ESP => write!(f, "esp"),
            X86Register::R8D => write!(f, "r8d"),
            X86Register::R9D => write!(f, "r9d"),
            X86Register::R10D => write!(f, "r10d"),
            X86Register::R11D => write!(f, "r11d"),
            X86Register::R12D => write!(f, "r12d"),
            X86Register::R13D => write!(f, "r13d"),
            X86Register::R14D => write!(f, "r14d"),
            X86Register::R15D => write!(f, "r15d"),

            // 64-bit
            X86Register::RAX => write!(f, "rax"),
            X86Register::RBX => write!(f, "rbx"),
            X86Register::RCX => write!(f, "rcx"),
            X86Register::RDX => write!(f, "rdx"),
            X86Register::RSI => write!(f, "rsi"),
            X86Register::RDI => write!(f, "rdi"),
            X86Register::RBP => write!(f, "rbp"),
            X86Register::RSP => write!(f, "rsp"),
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


#[derive(Debug, Clone, Eq, PartialEq)]
enum X86AddressingMode {
    Immediate(X86Immediate),
    // Immediate value
    Register(X86Register),
    // Register direct
    Indirect(X86Register),
    // Register indirect
    Displacement {
        // Base with displacement
        base: X86Register,
        displacement: i32,
    },
    Indexed {
        // Indexed with displacement
        base: X86Register,
        index: X86Register,
        displacement: i32,
        scale: i32,
    },
    DataLabel(GlobalLabelIdx),
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct X86Operand {
    mode: X86AddressingMode,
    size: X86Size,
}

impl X86Operand {
    pub fn register(reg: X86Register) -> Self {
        X86Operand {
            size: reg.size(),
            mode: X86AddressingMode::Register(reg),
        }
    }

    pub fn immediate(imm: X86Immediate) -> Self {
        X86Operand {
            size: imm.size(),
            mode: X86AddressingMode::Immediate(imm),
        }
    }

    pub fn const_bp_offset(offset: StackOffset, size: X86Size) -> Self {
        Self::const_register_offset(X86Register::RBP, offset.to_rbp_offset(), size)
    }

    pub fn const_register_offset(reg: X86Register, offset: i32, size: X86Size) -> Self {
        X86Operand {
            size,
            mode: X86AddressingMode::Displacement {
                base: reg,
                displacement: offset,
            },
        }
    }

    pub fn add_offset(&mut self, offset: i32) {
        match &mut self.mode {
            X86AddressingMode::Displacement { displacement, .. } => {
                *displacement += offset;
            }
            _ => panic!("Cannot add offset to non-displacement operand"),
        }
    }

    pub fn is_register(&self) -> bool {
        match self.mode {
            X86AddressingMode::Register(_) => true,
            _ => false,
        }
    }

    pub fn is_mem_operand(&self) -> bool {
        match self.mode {
            X86AddressingMode::Immediate(_) => false,
            X86AddressingMode::Register(_) => false,
            _ => true,
        }
    }
}


impl Display for X86Operand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            X86Operand { mode, size } => {
                match mode {
                    X86AddressingMode::Immediate(imm) => write!(f, "{}", imm),
                    X86AddressingMode::Register(reg) => write!(f, "{}", reg),
                    X86AddressingMode::Indirect(reg) => write!(f, "[{}]", reg),
                    X86AddressingMode::Displacement { base, displacement } => {
                        if *displacement == 0 {
                            write!(f, "{} [{}]", size, base)
                        } else {
                            write!(f, "{} [{} {}]", size, base, format_displacement(*displacement))
                        }
                    }
                    X86AddressingMode::Indexed {
                        base,
                        index,
                        displacement,
                        scale,
                    } => {
                        let displacement = *displacement;
                        if displacement == 0 && *scale == 1 {
                            write!(f, "{} [{} + {}]", size, base, index)
                        } else if displacement == 0 {
                            write!(f, "{} [{} + {} * {}]", size, base, index, scale)
                        } else if *scale == 1 {
                            write!(f, "{} [{} + {} {}]", size, base, index, format_displacement(displacement))
                        } else {
                            write!(f, "{} [{} + {} * {} {}]", size, base, index, scale, format_displacement(displacement))
                        }
                    }
                    X86AddressingMode::DataLabel(label) => {
                        write!(f, "[rip + {}]", label)
                    }
                }
            }
        }
    }
}

fn format_displacement(displacement: i32) -> String {
    if displacement < 0 {
        format!("- {}", -displacement)
    } else {
        format!("+ {}", displacement)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum X86Immediate {
    QWord(i64),
    DWord(i32),
    Word(i16),
    Byte(i8),
}

impl Display for X86Immediate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let size = self.size();
        match self {
            X86Immediate::QWord(imm) => write!(f, "{} {}", size, imm),
            X86Immediate::DWord(imm) => write!(f, "{} {}", size, imm),
            X86Immediate::Word(imm) => write!(f, "{} {}", size, imm),
            X86Immediate::Byte(imm) => write!(f, "{} {}", size, imm),
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

struct GlobalLabel {
    global_idx: GlobalIdx,
}

idx!(GlobalLabelIdx);

impl Display for GlobalLabelIdx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, ".LC{}", self.as_idx())
    }
}

struct Label {
    bb: BasicBlockIdx,
    function: FunctionIdx,
}
idx!(LabelIdx);

impl Display for LabelIdx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, ".L{}", self.index)
    }
}

enum X86Instruction {
    Mov(X86Operand, X86Operand),
    Add(X86Operand, X86Operand),
    Sub(X86Operand, X86Operand),
    Neg(X86Operand),
    Mul(X86Operand),
    Div(X86Operand),
    And(X86Operand, X86Operand),
    Or(X86Operand, X86Operand),
    Xor(X86Operand, X86Operand),
    Not(X86Operand),
    Cmp(X86Operand, X86Operand),
    Jmp(LabelIdx),
    Je(LabelIdx),
    Jne(LabelIdx),
    Jg(LabelIdx),
    Jge(LabelIdx),
    Jl(LabelIdx),
    Jle(LabelIdx),
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
    Setg(X86Register),
    Setge(X86Register),
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
            X86Instruction::Setg(reg) => write!(f, "setg {}", reg),
            X86Instruction::Setge(reg) => write!(f, "setge {}", reg),
            X86Instruction::Neg(op) => write!(f, "neg {}", op),
            X86Instruction::Or(op1, op2) => write!(f, "or {}, {}", op1, op2),
            X86Instruction::Xor(op1, op2) => write!(f, "xor {}, {}", op1, op2),
            X86Instruction::Not(op) => write!(f, "not {}", op),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum X86Size {
    Byte,
    Word,
    DWord,
    QWord,
}

impl X86Size {
    fn from_size(size: u8) -> Self {
        match size {
            1 => X86Size::Byte,
            2 => X86Size::Word,
            4 => X86Size::DWord,
            8 => X86Size::QWord,
            _ => panic!("Unsupported size: {}", size),
        }
    }

    fn from_layout(layout: &Layout) -> Self {
        Self::from_size(layout.size as u8)
    }

    fn num_bytes(&self) -> u32 {
        match self {
            X86Size::Byte => 1,
            X86Size::Word => 2,
            X86Size::DWord => 4,
            X86Size::QWord => 8,
        }
    }
}

impl From<&Layout> for X86Size {
    fn from(layout: &Layout) -> Self {
        X86Size::from_layout(layout)
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

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum PlaceLocation {
    Stack(StackFrameBlockIdx),
    Register(X86Register),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Temp {
    SavedReg(LocalIdx, StackFrameBlockIdx),
    SavedStack(LocalIdx, StackFrameBlockIdx),
}

pub struct X86Codegen<'a> {
    memory_location_allocator: Option<MemoryLocationAllocator>,
    mir: &'a MIR,
    scope: GlobalScopeCell,
    asm: String,
    labels: IdxVec<LabelIdx, Label>,
    global_labels: IdxVec<GlobalLabelIdx, GlobalLabel>,
    current_body: Option<&'a Body>,
    temps: HashMap<X86Register, Temp>,
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
            memory_location_allocator: None,
            labels: IdxVec::new(),
            current_body: None,
            global_labels: IdxVec::new(),
            temps: HashMap::new(),
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
        for (global_idx, global) in self.mir.globals.indexed_iter() {
            let label = self.get_global_label(global_idx);
            self.asm.push_str(&format!("{}:\n", label));
            self.asm.push_str("    ");
            match &global.value {
                GlobalValue::String(s) => {
                    self.asm.push_str(&format!(".asciz \"{}\"\n", s));
                }
            }
        }
    }

    pub fn gen_start(&mut self) {
        self.asm.push_str("_start:\n");
        self.mov_unchecked(
            X86Operand::register(X86Register::RBP),
            X86Operand::register(X86Register::RSP),
        );
        self.and_unchecked(
            X86Operand::register(X86Register::RSP),
            X86Operand::immediate(X86Immediate::QWord(-16)),
        );
        self.sub_unchecked(
            X86Operand::register(X86Register::RSP),
            X86Operand::immediate(X86Immediate::QWord(8)),
        );
        self.call("root__main".to_string());
        self.mov_unchecked(
            X86Operand::register(X86Register::RDI),
            X86Operand::register(X86Register::RAX),
        );
        self.mov_unchecked(
            X86Operand::register(X86Register::RAX),
            X86Operand::immediate(X86Immediate::QWord(60)),
        );
        self.syscall();
    }

    pub fn gen_body(&mut self, body: &'a Body) {
        self.current_body = Some(body);
        let scope = self.scope.borrow();
        let function = scope.get_function(&body.function);
        self.asm.push_str(&format!("{}:\n", self.format_qualified_name(&function.name)));
        drop(scope);
        self.create_memory_location_allocator(body);
        self.push(
            X86Operand::register(X86Register::RBP),
        );
        self.mov_unchecked(
            X86Operand::register(X86Register::RBP),
            X86Operand::register(X86Register::RSP),
        );
        self.allocator_mut().allocate(&body.scope);
        for bb in body.basic_blocks.indexed_iter() {
            self.gen_basic_block(bb);
        }
        self.temps.clear();
    }

    fn create_memory_location_allocator(&mut self, body: &'a Body) {
        let scope_ref = self.scope.borrow();
        let function = scope_ref.get_function(&body.function);
        let parameter_places = function.parameters.iter().map(|var_id| {
            body.scope.get_variable(var_id).unwrap().clone()
        }).collect();
        let mut allocator = MemoryLocationAllocator::new(
            parameter_places,
        );
        let mut alive_places = vec![];
        for instruction in body.basic_blocks.iter().map(|bb| &bb.instructions).flatten() {
            match &instruction.kind {
                InstructionKind::StorageLive { local } => {
                    allocator.add_local(*local);
                    for alive_place in alive_places.iter() {
                        allocator.add_interference(local, alive_place)
                    }
                    alive_places.push(*local);
                }
                InstructionKind::StorageDead { local } => {
                    alive_places.retain(|alive_place| alive_place != local);
                }
                _ => {}
            }
        }
        self.memory_location_allocator = Some(allocator);
    }

    fn allocator(&self) -> &MemoryLocationAllocator {
        self.memory_location_allocator.as_ref().unwrap()
    }

    fn allocator_mut(&mut self) -> &mut MemoryLocationAllocator {
        self.memory_location_allocator.as_mut().unwrap()
    }

    fn body(&self) -> &Body {
        self.current_body.as_ref().unwrap()
    }

    fn format_qualified_name(&self, name: &QualifiedName) -> String {
        name.name.replace(":", "_")
    }

    fn clear_stack_frame(&mut self) {
        self.free_temp_registers(&self.temps.iter().map(|(reg, _)| *reg).collect());
        self.mov_unchecked(
            X86Operand::register(X86Register::RSP),
            X86Operand::register(X86Register::RBP),
        );
        assert_eq!(self.allocator().stack.stack_pointer, 8);

        self.pop(
            X86Operand::register(X86Register::RBP),
        );
        assert_eq!(self.allocator().stack.stack_pointer, 0);
        self.ret();
    }

    fn get_label(&mut self, bb: BasicBlockIdx) -> LabelIdx {
        if let Some((idx, _)) = self.labels.indexed_iter().find(
            |(_, label)| label.bb == bb && label.function == self.body().function
        ) {
            return idx;
        }
        self.labels.push(Label {
            bb,
            function: self.body().function,
        })
    }

    fn get_global_label(&mut self, global: GlobalIdx) -> GlobalLabelIdx {
        if let Some((idx, _)) = self.global_labels.indexed_iter().find(
            |(_, label)| label.global_idx == global
        ) {
            return idx;
        }
        self.global_labels.push(GlobalLabel {
            global_idx: global
        })
    }

    pub fn gen_basic_block(&mut self, bb: (BasicBlockIdx, &BasicBlock)) {
        let saved_stack_frame = self.allocator().stack.clone();
        let label_idx = self.get_label(bb.0);
        self.asm.push_str(&format!("{}:\n", label_idx));
        for inst in bb.1.instructions.iter() {
            self.gen_instruction(inst);
        }
        let terminator = &bb.1.terminator;
        self.gen_term(terminator);
        match &terminator.kind {
            TerminatorKind::Return => {
                self.allocator_mut().stack = saved_stack_frame;
            }
            _ => {}
        }
    }

    pub fn gen_term(&mut self, term: &crate::mir::Terminator) {
        match &term.kind {
            crate::mir::TerminatorKind::Goto(label) => {
                let label_idx = self.get_label(*label);
                self.jmp(label_idx);
            }
            crate::mir::TerminatorKind::Return => {
                self.clear_stack_frame();
            }
            crate::mir::TerminatorKind::If {
                condition,
                then,
                else_,
            } => {
                // todo: maybe do not pass a Value here but rather a new type called condition which can hold bool binary expressions or literal values
                let (condition_operand, temps) = self.gen_operand_op(condition);
                let zero_check = X86Operand::immediate(X86Immediate::Byte(0));
                self.cmp_unchecked(
                    condition_operand,
                    zero_check,
                );
                self.free_temp_registers(&temps);
                let then_label_idx = self.get_label(*then);
                self.jne(then_label_idx);
                let else_label_idx = self.get_label(*else_);
                self.jmp(else_label_idx);
            }
            crate::mir::TerminatorKind::Next => {}
            TerminatorKind::Unresolved => {
                unreachable!()
            }
        }
    }

    pub fn gen_instruction(&mut self, instruction: &Instruction) {
        match &instruction.kind {
            InstructionKind::Assign {
                place,
                value
            } => {
                self.gen_value(value, place);
            }
            InstructionKind::Call {
                return_value_place,
                args,
                function_id
            } => {
                let scope = self.scope.borrow();
                let function = scope.get_function(function_id);
                let args = args.iter().zip(function.parameters.iter()).map(|(arg, param)| {
                    let param = scope.get_variable(param);
                    (arg, MIRType::from_type(&param.ty,&scope))
                }).collect();

                drop(scope);
                let (used_regs, arg_size) = self.layout_function_call_args(args);
                let scope = self.scope.borrow();

                let function = scope.get_function(function_id);
                let function_name = self.format_qualified_name(&function.name);
                drop(scope);
                self.call(function_name);
                // todo
                // self.add_unchecked(
                //     X86Operand::register(X86Register::RSP),
                //     X86Operand::immediate(X86Immediate::QWord(arg_size as i64)),
                // );
                // todo: for now we assume that each function returns its value in rax
                self.free_temp_registers(&used_regs);
                let (return_value_operand, temps) = self.get_operand_for_place(return_value_place);
                let size = return_value_operand.size;
                self.mov_unchecked(
                    return_value_operand,
                    X86Operand::register(X86Register::RAX.resize(&size)),
                );
                self.free_temp_registers(&temps);
            }
            InstructionKind::StorageLive { local: local_idx } => {
                let layout = self.layout_local(*local_idx);
                let sp = self.allocator().stack.stack_pointer;
                self.allocator_mut().mark_local_as_alive(*local_idx, layout);
                let diff = self.allocator().stack.stack_pointer - sp;
                if diff > 0 {
                    self.sub_unchecked(
                        X86Operand::register(X86Register::RSP),
                        X86Operand::immediate(X86Immediate::QWord(diff as i64)),
                    );
                }
            }
            InstructionKind::StorageDead { local } => {
                self.allocator_mut().mark_variable_as_dead(*local);
                self.cleanup_stack();
            }
            InstructionKind::PlaceMention(_) => {}
        }
    }


    fn use_temp_reg(&mut self, size: X86Size) -> X86Register {
        // todo: spill any register if there is no free register
        let reg = self.allocator_mut().get_free_register(size).expect("no free temp register");
        self.use_specific_temp_reg(reg)
    }

    fn use_specific_temp_reg(&mut self, register: X86Register) -> X86Register {
        let in_use_by_local = self.allocator().alive_locals.iter().find(
            |local| match &self.allocator().locals[local] {
                PlaceLocation::Stack(_) => {
                    false
                }
                PlaceLocation::Register(local_reg) => {
                    *local_reg == register
                }
            }
        ).copied();
        match in_use_by_local {
            None => {
                register
            }
            Some(local) => {
                // todo: replace with push
                let block = self.push(X86Operand::register(register));
                self.allocator_mut().locals.insert(local, PlaceLocation::Stack(block));
                self.allocator_mut().temp_registers.insert(register);
                self.temps.insert(register, Temp::SavedReg(local, block));
                register
            }
        }
    }

    fn free_temp_register(&mut self, register: X86Register) {
        // todo: develop algorithm to check if we can free place on the stack every time we operate on it
        match self.temps.get(&register).copied() {
            None => {}
            Some(temp) => {
                match temp {
                    Temp::SavedReg(local, block) => {
                        self.allocator_mut().locals.insert(local, PlaceLocation::Register(register));
                        self.allocator_mut().temp_registers.remove(&register);
                        self.temps.remove(&register);
                        let is_on_top = self.allocator().stack.blocks.iter().last().map(|last_block| last_block.idx == block).unwrap_or(false);
                        if is_on_top {
                            self.pop(X86Operand::register(register));
                        } else {
                            self.allocator_mut().stack.free_block(block);
                            let saved_value_offset = self.allocator().get_block_offset(block);
                            self.mov_unchecked(
                                X86Operand::register(register),
                                X86Operand::const_bp_offset(saved_value_offset, register.size()),
                            );
                        }
                    }
                    Temp::SavedStack(local, block) => {
                        self.allocator_mut().locals.insert(local, PlaceLocation::Stack(block));
                        self.allocator_mut().temp_registers.remove(&register);
                        self.temps.remove(&register);
                        let offset = self.allocator().get_block_offset(block);
                        self.mov_unchecked(
                            X86Operand::const_bp_offset(offset, register.size()),
                            X86Operand::register(register),
                        );
                    }
                }
            }
        }
    }

    fn cleanup_stack(&mut self) {
        let diff_to_last_allocation = self.allocator().stack.check_difference();
        match diff_to_last_allocation {
            None => {}
            Some(diff) => {
                self.decrease_stack_size(diff);
            }
        }
    }

    fn free_temp_registers(&mut self, registers: &Vec<X86Register>) {
        for register in registers.iter().rev() {
            self.free_temp_register(*register);
        }
    }

    fn ensure_in_reg(&mut self, operand: X86Operand) -> X86Register {
        if let X86AddressingMode::Register(reg) = operand.mode {
            return reg;
        }
        let register = self.use_temp_reg(operand.size);
        self.mov_unchecked(X86Operand::register(register), operand);
        register
    }

    fn ensure_in_specific_reg(&mut self, operand: X86Operand, register: X86Register) -> X86Register {
        if let X86AddressingMode::Register(reg) = operand.mode {
            if reg == register {
                return reg;
            }
        }
        let register = self.use_specific_temp_reg(register);
        self.mov_unchecked(X86Operand::register(register), operand);
        register
    }

    fn ensure_local_in_register(&mut self, local_idx: LocalIdx) -> X86Register {
        match *self.allocator().get_location(&local_idx) {
            PlaceLocation::Stack(block) => {
                let local_layout = self.layout_local(local_idx);
                let offset = self.allocator().get_block_offset(block);
                let register = self.use_temp_reg(X86Size::from_layout(&local_layout));
                self.mov_unchecked(X86Operand::register(register), X86Operand::const_bp_offset(offset,
                                                                                               X86Size::from_layout(&local_layout),
                ));
                self.allocator_mut().locals.insert(local_idx, PlaceLocation::Register(register));
                self.temps.insert(register, Temp::SavedStack(
                    local_idx,
                    block,
                ));
                self.allocator_mut().temp_registers.insert(register);
                register
            }
            PlaceLocation::Register(reg) => {
                reg
            }
        }
    }

    fn mov(&mut self, destination: X86Operand, source: X86Operand) {
        if destination.is_mem_operand() && source.is_mem_operand() {
            let temp_register = self.use_temp_reg(source.size);
            self.mov_unchecked(X86Operand::register(temp_register), source);
            self.mov_unchecked(destination, X86Operand::register(temp_register));
            self.free_temp_register(temp_register);
        } else {
            self.mov_unchecked(destination, source);
        }
    }

    fn mov_unchecked(&mut self, destination: X86Operand, source: X86Operand) {
        if destination == source {
            return;
        }
        self._push_instruction(X86Instruction::Mov(destination, source));
    }

    fn sub_unchecked(&mut self, lhs: X86Operand, rhs: X86Operand) {
        self._push_instruction(X86Instruction::Sub(lhs, rhs));
    }

    fn and_unchecked(&mut self, lhs: X86Operand, rhs: X86Operand) {
        self._push_instruction(X86Instruction::And(lhs, rhs));
    }

    fn add_unchecked(&mut self, lhs: X86Operand, rhs: X86Operand) {
        self._push_instruction(X86Instruction::Add(lhs, rhs));
    }

    fn neg_unchecked(&mut self, operand: X86Operand) {
        self._push_instruction(X86Instruction::Neg(operand));
    }

    fn not_unchecked(&mut self, operand: X86Operand) {
        self._push_instruction(X86Instruction::Not(operand));
    }

    fn call(&mut self, name: String) {
        self._push_instruction(X86Instruction::Call(name));
    }

    fn push(&mut self, operand: X86Operand) -> StackFrameBlockIdx {
        let block = self.allocator_mut().stack.push_block(operand.size.num_bytes());
        self._push_instruction(X86Instruction::Push(operand));
        block
    }

    fn pop(&mut self, operand: X86Operand) {
        self.allocator_mut().stack.pop_block();
        self._push_instruction(X86Instruction::Pop(operand));
    }

    fn increase_stack_size(&mut self, size: u32) {
        self.allocator_mut().stack.push_block(size);
        self.sub_unchecked(X86Operand::register(X86Register::RSP), X86Operand::immediate(X86Immediate::QWord(size as i64)));
    }

    fn decrease_stack_size(&mut self, size: u32) {
        self.allocator_mut().stack.decrease_stack_by(size);
        self.add_unchecked(X86Operand::register(X86Register::RSP), X86Operand::immediate(X86Immediate::QWord(size as i64)));
    }

    fn ret(&mut self) {
        self._push_instruction(X86Instruction::Ret);
    }

    fn cmp_unchecked(&mut self, lhs: X86Operand, rhs: X86Operand) {
        self._push_instruction(X86Instruction::Cmp(lhs, rhs));
    }

    fn je(&mut self, label: LabelIdx) {
        self._push_instruction(X86Instruction::Je(label));
    }

    fn jne(&mut self, label: LabelIdx) {
        self._push_instruction(X86Instruction::Jne(label));
    }

    fn jmp(&mut self, label: LabelIdx) {
        self._push_instruction(X86Instruction::Jmp(label));
    }

    fn lea(&mut self, destination: X86Operand, source: X86Operand) {
        if !destination.is_register() {
            let temp_register = self.use_temp_reg(source.size);
            self.lea_unchecked(X86Operand::register(temp_register), source);
            self.mov_unchecked(destination, X86Operand::register(temp_register));
            self.free_temp_register(temp_register);
            return;
        }
        self._push_instruction(X86Instruction::Lea(destination, source));
    }

    fn lea_unchecked(&mut self, destination: X86Operand, source: X86Operand) {
        self._push_instruction(X86Instruction::Lea(destination, source));
    }

    fn syscall(&mut self) {
        self._push_instruction(X86Instruction::Syscall);
    }

    fn raw(&mut self, instruction: &'static str) {
        self._push_instruction(X86Instruction::Raw(instruction));
    }

    fn cqo(&mut self) {
        self._push_instruction(X86Instruction::Cqo);
    }

    fn div_unchecked(&mut self, operand: X86Operand) {
        self._push_instruction(X86Instruction::Div(operand));
    }

    fn mul_unchecked(&mut self, operand: X86Operand) {
        self._push_instruction(X86Instruction::Mul(operand));
    }

    fn copy_value(&mut self, from: &Place, to: &Place) {
        let (from_mem_op, from_temps) = self.get_operand_for_place(&from);
        let (to_mem_op, to_temps) = self.get_operand_for_place(&to);
        let to_layout = self.layout_local(to.local);
        if to_layout.size <= Layout::POINTER_SIZE {
            self.mov(to_mem_op, from_mem_op);
        } else {
            let from_addr_reg = self.use_specific_temp_reg(X86Register::RSI);
            self.lea_unchecked(
                X86Operand::register(from_addr_reg),
                from_mem_op,
            );
            let to_addr_reg = self.use_specific_temp_reg(X86Register::RDI);
            self.lea_unchecked(
                X86Operand::register(to_addr_reg),
                to_mem_op,
            );
            let count_reg = self.use_specific_temp_reg(X86Register::RCX);
            self.mov_unchecked(
                X86Operand::register(count_reg),
                X86Operand::immediate(X86Immediate::QWord(to_layout.size as i64)),
            );
            self.raw("rep movsb");
            self.free_temp_register(from_addr_reg);
            self.free_temp_register(to_addr_reg);
            self.free_temp_register(count_reg);
        }
        self.free_temp_registers(&from_temps);
        self.free_temp_registers(&to_temps);
    }

    fn gen_value(&mut self, value: &Rvalue, store_at: &Place) {
        match value {
            Rvalue::Struct(values) => {
                let mut store_at = store_at.clone();
                for (field_idx, operand) in values.iter() {
                    store_at.projection = Some(Projection::Field(*field_idx));
                    self.gen_operand_and_store(operand, &store_at);
                }
            }
            Rvalue::AddressOf(_, place) => {
                self.gen_place(&place, store_at);
            }
            Rvalue::Use(operand) => {
                self.gen_operand_and_store(operand, store_at);
            }
            Rvalue::BinaryOp(op, (lhs, rhs)) => {
                self.gen_binop(op, lhs, rhs, store_at)
            }
            Rvalue::UnaryOp(op, operand) => {
                self.gen_unop(op, operand, store_at)
            }
            Rvalue::Global(global_idx) => {
                let label = self.get_global_label(*global_idx);
                let data_label_op = X86Operand {
                    size: X86Size::QWord,
                    mode: X86AddressingMode::DataLabel(label),
                };
                let (store_at, temps) = self.get_operand_for_place(store_at);
                self.lea(store_at, data_label_op);
                self.free_temp_registers(&temps);
            }
        }
    }

    fn gen_binop(&mut self, op: &BinOp, lhs: &Operand, rhs: &Operand, store_at: &Place) {
        let (lhs_op, temps_lhs) = self.gen_operand_op(lhs);
        let (rhs_op, temps_rhs) = self.gen_operand_op(rhs);
        match op {
            BinOp::Add
            | BinOp::Sub
            | BinOp::Mul
            | BinOp::Div
            | BinOp::And
            => {
                self.gen_arithmetic_op(op, lhs_op, rhs_op, store_at);
            }
            BinOp::Eq
            | BinOp::Neq
            | BinOp::Lt
            | BinOp::Leq
            | BinOp::Gt
            | BinOp::Geq
            => {
                self.gen_comp_op(op, lhs, rhs, store_at);
            }
            _ => unimplemented!("Binary operator {:?} not implemented", op),
        };
        self.free_temp_registers(&temps_lhs);
        self.free_temp_registers(&temps_rhs);
    }

    fn gen_arithmetic_op(&mut self, op: &BinOp, lhs_op: X86Operand, rhs_op: X86Operand, store_at: &Place) {
        let (store_at_op, store_at_temps) = self.get_operand_for_place(store_at);
        match op {
            BinOp::Add => {
                let lhs_op = self.ensure_in_reg(lhs_op);
                self.mov_unchecked(
                    store_at_op.clone(),
                    X86Operand::register(lhs_op),
                );
                // todo: ensure that store_at_op is a register
                self.add_unchecked(
                    store_at_op,
                    rhs_op,
                );
                self.free_temp_register(lhs_op);
            }
            BinOp::Sub => {
                let lhs_op = self.ensure_in_reg(lhs_op);
                self.sub_unchecked(
                    X86Operand::register(lhs_op),
                    rhs_op,
                );
                self.mov_unchecked(
                    store_at_op,
                    X86Operand::register(lhs_op),
                );
                self.free_temp_register(lhs_op);
            }
            BinOp::Mul => {
                self.ensure_in_specific_reg(lhs_op, X86Register::RAX);
                self.cqo();
                self.mul_unchecked(
                    rhs_op,
                );
                self.mov_unchecked(
                    store_at_op,
                    X86Operand::register(X86Register::RAX),
                );
                self.free_temp_register(X86Register::RAX);
            }
            BinOp::Div => {
                self.ensure_in_specific_reg(lhs_op, X86Register::RAX);
                self.cqo();
                self.div_unchecked(
                    rhs_op,
                );
                self.mov_unchecked(
                    store_at_op,
                    X86Operand::register(X86Register::RAX),
                );
                self.free_temp_register(X86Register::RAX);
            }
            BinOp::And => {
                let lhs_op = self.ensure_in_reg(lhs_op);
                self.and_unchecked(
                    X86Operand::register(lhs_op),
                    rhs_op,
                );
                self.mov_unchecked(
                    store_at_op,
                    X86Operand::register(lhs_op),
                );
                self.free_temp_register(lhs_op);
            }
            _ => unimplemented!("Binary operator {:?} not implemented", op),
        };
        self.free_temp_registers(&store_at_temps);
    }

    fn gen_comp_op(&mut self, op: &BinOp, lhs: &Operand, rhs: &Operand, store_at: &Place) {
        let (lhs_op, lhs_temps) = self.gen_operand_op(lhs);
        let (rhs_op, rhs_temps) = self.gen_operand_op(rhs);
        // todo: check if store_at is already mapped to a register
        let store_at_reg = self.use_temp_reg(X86Size::Byte);
        assert_eq!(store_at_reg.size(), X86Size::Byte);
        assert_eq!(rhs_op.size, lhs_op.size, "Comparing operands (op: {}) of different sizes is not supported: {:?} and {:?}", op, lhs_op, rhs_op);
        let source = self.ensure_in_reg(lhs_op);
        let comp_instruction = match op {
            BinOp::And => {
                X86Instruction::And(
                    X86Operand::register(source),
                    rhs_op,
                )
            }
            BinOp::Or => {
                X86Instruction::Or(
                    X86Operand::register(source),
                    rhs_op,
                )
            }
            BinOp::Xor => {
                X86Instruction::Xor(
                    X86Operand::register(source),
                    rhs_op,
                )
            }
            _ => {
                self.cmp_unchecked(
                    X86Operand::register(source),
                    rhs_op,
                );
                match op {
                    BinOp::Eq => {
                        X86Instruction::Sete(
                            store_at_reg,
                        )
                    }
                    BinOp::Neq => {
                        X86Instruction::Setne(
                            store_at_reg,
                        )
                    }
                    BinOp::Lt => {
                        X86Instruction::Setl(
                            store_at_reg
                        )
                    }
                    BinOp::Leq => {
                        X86Instruction::Setle(
                            store_at_reg
                        )
                    }
                    BinOp::Gt => {
                        X86Instruction::Setg(
                            store_at_reg
                        )
                    }
                    BinOp::Geq => {
                        X86Instruction::Setge(
                            store_at_reg
                        )
                    }
                    _ => unimplemented!("Comp operator {:?} not implemented", op),
                }
            }
        };
        self._push_instruction(comp_instruction);
        let (operand, operand_temps) = self.get_operand_for_place(store_at);
        self.mov(
            operand,
            X86Operand::register(store_at_reg),
        );
        self.free_temp_registers(&lhs_temps);
        self.free_temp_registers(&rhs_temps);
        self.free_temp_register(source);
        self.free_temp_register(store_at_reg);
        self.free_temp_registers(&operand_temps);
    }

    fn gen_unop(&mut self, op: &UnOp, operand: &Operand, store_at: &Place) {
        let (operand_op, operand_temps) = self.gen_operand_op(operand);
        let (store_at_op, store_temps) = self.get_operand_for_place(store_at);
        match op {
            UnOp::Neg => {
                self.mov(
                    store_at_op.clone(),
                    operand_op,
                );
                self.neg_unchecked(
                    store_at_op,
                );
            }
            UnOp::Not => {
                self.mov(
                    store_at_op.clone(),
                    operand_op,
                );
                self.not_unchecked(
                    store_at_op,
                );
            }
            _ => unimplemented!("Unary operator {:?} not implemented", op),
        };
        self.free_temp_registers(&operand_temps);
        self.free_temp_registers(&store_temps);
    }

    fn gen_place(&mut self, place: &Place, store_at: &Place) {
        let (place_op, place_temps) = self.get_operand_for_place(place);
        let (store_at_op, store_temps) = self.get_operand_for_place(store_at);
        self.lea(
            store_at_op,
            place_op,
        );
        self.free_temp_registers(&place_temps);
        self.free_temp_registers(&store_temps);
    }

    fn gen_operand_and_store(&mut self, operand: &Operand, store_at: &Place) {
        match operand {
            Operand::Copy(place) => {
                self.copy_value(place, store_at);
            }
            Operand::Constant(value) => {
                self.gen_constant_and_store(value, store_at);
            }
        }
    }

    fn gen_operand_op(&mut self, operand: &Operand) -> (X86Operand, Vec<X86Register>) {
        match operand {
            Operand::Copy(place) => {
                self.get_operand_for_place(place)
            }
            Operand::Constant(value) => {
                (self.gen_constant_op(value), vec![])
            }
        }
    }

    fn gen_constant_and_store(&mut self, value: &ConstantValue, store_at: &Place) {
        let (op, temps) = self.get_operand_for_place(store_at);
        let val_op = self.gen_constant_op(value);
        self.mov(op, val_op);
        self.free_temp_registers(&temps);
    }

    fn gen_constant_op(&mut self, value: &ConstantValue) -> X86Operand {
        match value {
            ConstantValue::Scalar(scalar) => {
                let size = X86Size::from_size(scalar.size);
                let immediate = match size {
                    X86Size::Byte => X86Immediate::Byte(scalar.into_i8().unwrap()),
                    X86Size::Word => X86Immediate::Word(scalar.into_i16().unwrap()),
                    X86Size::DWord => X86Immediate::DWord(scalar.into_i32().unwrap()),
                    X86Size::QWord => X86Immediate::QWord(scalar.into_i64().unwrap()),
                };
                X86Operand::immediate(immediate)
            }
            ConstantValue::ZeroSized => {
                unimplemented!()
            }
        }
    }

    fn get_operand_for_place(&mut self, place: &Place) -> (X86Operand, Vec<X86Register>) {
        let local_layout = self.layout_local(place.local);
        let loc = self.allocator().get_location(&place.local);
        match *loc {
            PlaceLocation::Stack(block) => {
                let mut layout = local_layout;
                let mut offset = self.allocator().get_block_offset(block);
                if let Some(projection) = place.projection.clone() {
                    match projection {
                        Projection::Field(idx) => {
                            let additional_offset = self.scope.borrow().get_field_offset(&idx);
                            offset.add_offset(additional_offset);
                            layout = self.scope.borrow().get_field(&idx).ty.layout(&self.scope.borrow());
                        }
                        Projection::Index(index_stored_at) => {
                            let index = self.ensure_local_in_register(index_stored_at);
                            let local = self.get_local(place.local);

                            return (X86Operand {
                                size: X86Size::from_layout(&local.ty.deref_type().expect("Cannot deref type").layout()),
                                mode: X86AddressingMode::Indexed {
                                    base: X86Register::RBP,
                                    index,
                                    displacement: offset.to_rbp_offset(),
                                    scale: 1,
                                },
                            }, vec![index]);
                        }
                        Projection::ConstantIndex(index) => {
                            let additional_offset = index * layout.size as u32;
                            offset.add_offset(additional_offset);
                        }
                        Projection::Deref => {
                            let temp_reg = self.use_temp_reg(
                                X86Size::QWord,
                            );
                            self.mov_unchecked(X86Operand::register(temp_reg),
                                               X86Operand::const_bp_offset(offset, X86Size::from_layout(&layout)),
                            );
                            let local = self.get_local(place.local);
                            let ty = local.ty.deref_type().expect("Cannot deref type");
                            layout = ty.layout();
                            return (X86Operand {
                                size: X86Size::from_layout(&layout),
                                mode: X86AddressingMode::Indirect(temp_reg),
                            }, vec![temp_reg]);
                        }
                    }
                }
                (X86Operand::const_bp_offset(offset, X86Size::from_layout(&layout)), vec![])
            }
            PlaceLocation::Register(reg) => {
                let mut operand = X86Operand::register(reg);
                let local = self.get_local(place.local);
                let mut ty = &local.ty;
                for projection in &place.projection {
                    match projection {
                        Projection::Deref => {
                            ty = ty.deref_type().expect("Cannot deref type");
                            operand = X86Operand {
                                mode: X86AddressingMode::Indirect(reg),
                                size: X86Size::from_layout(&ty.layout()),
                            };
                        }
                        Projection::Index(idx) => {
                            let index = self.ensure_local_in_register(*idx);
                            let local = self.get_local(place.local);

                            return (X86Operand {
                                size: X86Size::from_layout(&local.ty.deref_type().expect("Cannot deref type").layout()),
                                mode: X86AddressingMode::Indexed {
                                    base: reg,
                                    index,
                                    displacement: 0,
                                    scale: 1,
                                },
                            }, vec![index]);
                        }
                        _ => panic!("Cannot project from register: {:?}", place.projection),
                    }
                }
                (operand, vec![])
            }
        }
    }
    fn layout_local(&self, local: LocalIdx) -> Layout {
        self.body().scope.locals.get(local).ty.layout()
    }

    fn get_local(&self, local: LocalIdx) -> &Local {
        self.body().scope.locals.get(local)
    }

    fn layout_function_call_args(&mut self, args: Vec<(&Operand, MIRType)>) -> (Vec<X86Register>, u32) {
        let mut arg_size = 0;
        let registers =[
            X86Register::RDI,
            X86Register::RSI,
            X86Register::RDX,
            X86Register::RCX,
            X86Register::R8,
            X86Register::R9,
        ];
        let arg_registers = args.iter().zip(
            registers.iter()
        ).map(|((_,ty),reg)| {
            let size = X86Size::from_layout(&ty.layout());
            reg.resize(&size)
        }).collect::<Vec<_>>();
        let mut used_registers = Vec::new();
        for (index, (value,_)) in args.iter().enumerate() {
            let (operand, temps) = self.gen_operand_op(value);
            if index < arg_registers.len() {
                let reg = self.use_specific_temp_reg(arg_registers[index]);
                self.mov_unchecked(
                    X86Operand::register(reg),
                    operand,
                );
                used_registers.push(reg);
            } else {
                arg_size += operand.size.num_bytes();
                self.push(operand);
            }
            used_registers.extend(temps);
        }
        (used_registers, arg_size)
    }

    fn _push_instruction(&mut self, instruction: X86Instruction) {
        self.asm.push_str("    ");
        self.asm.push_str(format!("{}", instruction).as_str());
        self.asm.push_str("\n");
    }
}