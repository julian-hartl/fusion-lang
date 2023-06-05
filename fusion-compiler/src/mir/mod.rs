use std::any::Any;
use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::env::var;
use std::fmt::{Display, Formatter, write};
use std::io::Write;
use std::ops::{Add, Deref};
use std::rc::Rc;

use fusion_compiler::{idx, Idx, IdxVec};

use crate::diagnostics::DiagnosticsBagCell;
use crate::hir::{FieldIdx, FunctionIdx, HIR, HIRBinaryOperator, HIRCallee, HIRExpression, HIRExpressionKind, HIRFunction, HIRGlobal, HIRLiteralExpression, HIRLiteralValue, HIRStatement, HIRStatementKind, HIRUnaryOperator, IntegerLiteralValue, VariableIdx};
use crate::mir::Category::RValue;
use crate::modules::scopes::{GlobalScope, GlobalScopeCell};
use crate::modules::symbols::Function;
use crate::text::span::TextSpan;
use crate::typings::{IntSize, Layout, Type};

idx!(BasicBlockIdx);

impl Display for BasicBlockIdx {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "bb{}", self.as_idx())
    }
}

impl Add<usize> for BasicBlockIdx {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self::new(self.as_idx() + rhs)
    }
}

pub struct Body {
    pub basic_blocks: IdxVec<BasicBlockIdx, BasicBlock>,
    pub scope: BodyScope,
    pub function: FunctionIdx,
}

impl Body {
    pub fn new(
        function: FunctionIdx,
        scope: GlobalScopeCell,
    ) -> Self {
        Self {
            scope: BodyScope::new(scope, function),
            function,
            basic_blocks: IdxVec::new(),
        }
    }
}

#[derive(Debug)]
pub struct BasicBlock {
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}

impl BasicBlock {
    pub fn new() -> Self {
        Self {
            instructions: vec![],
            terminator: Terminator {
                kind: TerminatorKind::Unresolved,
                span: None,
            },
        }
    }
}

idx!(GlobalIdx);

impl Display for GlobalIdx {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "global_{}", self.as_idx())
    }
}

#[derive(Debug, Clone)]
pub struct Global {
    pub value: GlobalValue,
    pub ty: MIRType,
}

#[derive(Debug, Clone)]
pub enum GlobalValue {
    String(String),
}

impl Display for GlobalValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GlobalValue::String(string) => write!(f, "\"{}\"", string),
        }
    }
}

pub struct GlobalMIRScope {
    globals: IdxVec<GlobalIdx, Global>,
}

impl GlobalMIRScope {
    pub fn new() -> Self {
        Self {
            globals: IdxVec::new(),
        }
    }

    pub fn add_global_string(&mut self, string: String) -> GlobalIdx {
        let value = GlobalValue::String(string);
        self.globals.push(
            Global {
                ty: MIRType::Ptr(Box::new(MIRType::Char)),
                value,
            }
        )
    }
}


pub struct Local {
    pub ty: MIRType,
    pub variable_idx: Option<VariableIdx>,
}
idx!(LocalScopeIdx);

impl Display for LocalScopeIdx {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "scope_{}", self.as_idx())
    }
}

pub struct LocalScope {
    pub children: Vec<LocalScopeIdx>,
    pub locals: Vec<LocalIdx>,
}

pub struct BodyScope {
    mapped_variables: HashMap<VariableIdx, LocalIdx>,
    scope: GlobalScopeCell,
    scopes: IdxVec<LocalScopeIdx, LocalScope>,
    scope_stack: Vec<LocalScopeIdx>,
    pub(crate) locals: IdxVec<LocalIdx, Local>,
    function_id: FunctionIdx,
    arg_count: usize,
}

impl BodyScope {
    pub fn new(
        scope: GlobalScopeCell,
        function_id: FunctionIdx,
    ) -> Self {
        let mut scope = Self {
            scope_stack: vec![],
            mapped_variables: HashMap::new(),
            scope,
            scopes: IdxVec::new(),
            locals: IdxVec::new(),
            function_id,
            arg_count: 0,
        };
        scope.populate();
        scope
    }

    pub fn return_place(&self) -> Place {
        Place::new(Place::RETURN_PLACE, None)
    }

    fn populate(&mut self) {
        let scope = self.scope.borrow();
        let function = scope.get_function(&self.function_id);
        self.arg_count = function.parameters.len();
        let params = function.parameters.clone();
        let return_type = self.map_hir_ty(&function.return_type);
        drop(scope);
        self.enter_scope();

        self.add_local(
            Local {
                variable_idx: None,
                ty: return_type,
            },
        );
        for param in params.iter() {
            self.new_mapped_local(param);
        }
    }

    pub fn all_locals(&self) -> &Vec<Local> {
        self.locals.as_vec()
    }

    pub fn new_local(&mut self, ty: &Type) -> LocalIdx {
        let ty = self.map_hir_ty(ty);
        self.add_local(Local {
            ty,
            variable_idx: None,
        })
    }

    pub fn new_mapped_local(&mut self, id: &VariableIdx) -> LocalIdx {
        let scope = self.get_scope();
        let variable = scope.get_variable(id);
        let ty = variable.ty.clone();
        drop(scope);
        let temp = self.add_local(Local {
            ty: self.map_hir_ty(&ty),
            variable_idx: Some(*id),
        });
        self.mapped_variables.insert(*id, temp.clone());
        temp
    }

    fn add_local(&mut self, local: Local) -> LocalIdx {
        let id = self.locals.push(local);
        self.current_scope_mut().expect(
            "No current scope"
        ).locals.push(id);
        id
    }

    fn get_scope(&self) -> Ref<GlobalScope> {
        self.scope.borrow()
    }

    pub fn get_variable(&self, id: &VariableIdx) -> Option<&LocalIdx> {
        self.mapped_variables.get(id)
    }

    fn enter_scope(&mut self) {
        let id = self.scopes.push(LocalScope {
            children: vec![],
            locals: vec![],
        }
        );
        if let Some(scope) = self.current_scope_mut() {
            scope.children.push(id);
        }
        self.scope_stack.push(id);
    }

    fn exit_scope(&mut self) {
        self.scope_stack.pop();
    }

    fn current_scope_mut(&mut self) -> Option<&mut LocalScope> {
        Some(self.scopes.get_mut(*self.scope_stack.last()?))
    }

    fn current_scope(&self) -> &LocalScope {
        self.scopes.get(*self.scope_stack.last().unwrap())
    }

    fn map_hir_ty(&self, ty: &Type) -> MIRType {
        MIRType::from_type(ty, &self.get_scope())
    }

    pub fn alive_locals(&self) -> Vec<LocalIdx> {
        let mut alive = vec![];
        for scope in self.scope_stack.iter().rev() {
            alive.extend(self.scopes.get(*scope).locals.iter().rev().cloned());
        }
        alive
    }
}

#[derive(Debug)]
pub struct Instruction {
    pub kind: InstructionKind,
    pub span: TextSpan,
}

impl Instruction {
    fn format(
        &self,
        scope: &GlobalScope,
    ) -> String {
        match &self.kind {
            InstructionKind::Assign { place, value: init } => {
                format!("{} = {}", place, init)
            }
            InstructionKind::Call { function_id, args, return_value_place: return_value_ptr } => {
                let mut s = format!("{} = @{}(", return_value_ptr, scope.get_function(function_id).name);
                for (i, arg) in args.iter().enumerate() {
                    if i != 0 {
                        s.push_str(", ");
                    }
                    s.push_str(format!("{}", arg).as_str());
                }
                s.push_str(")");
                s
            }
            InstructionKind::StorageLive { local } => {
                format!("StorageLive({})", local)
            }
            InstructionKind::StorageDead { local } => {
                format!("StorageDead({})", local)
            }
            InstructionKind::PlaceMention(place) => {
                format!("{}", place)
            }
        }
    }
}
idx!(LocalIdx);

impl Display for LocalIdx {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "_{}", self.index)
    }
}

#[derive(Debug, Clone)]
pub struct Place {
    pub local: LocalIdx,
    pub projection: Option<Projection>,
}

impl Place {
    pub fn ty(&self, scope: &BodyScope, global_scope: &GlobalScope) -> Option<MIRType> {
        let local = scope.locals.get(self.local);
        let ty = local.ty.clone();
        let ty = match &self.projection {
            None => {
                ty
            }
            Some(proj) => {
                match proj {
                    Projection::Field(field_idx) => {
                        MIRType::from_type(&global_scope.get_field(field_idx).ty, global_scope)
                    }
                    Projection::Index(_) => {
                        ty.deref_type().cloned()?
                    }
                    Projection::ConstantIndex(_) => {
                        ty.deref_type().cloned()?
                    }
                    Projection::Deref => {
                        ty.deref_type().cloned()?
                    }
                }
            }
        };
        Some(ty)
    }
}

#[derive(Debug, Clone)]
pub enum Projection {
    Field(FieldIdx),
    Index(LocalIdx),
    ConstantIndex(u32),
    Deref,
}


impl Place {
    pub const RETURN_PLACE: LocalIdx = LocalIdx {
        index: 0,
    };

    pub fn return_place() -> Self {
        Self::new(Self::RETURN_PLACE, None)
    }

    pub fn new(local: LocalIdx, projection: Option<Projection>) -> Self {
        Self {
            local,
            projection,
        }
    }
}

impl Display for Place {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.local)?;
        for projection in &self.projection {
            match projection {
                Projection::Field(field) => {
                    write!(f, ".{}", field.as_idx())?;
                }
                Projection::Index(local) => {
                    write!(f, "[{}]", local)?;
                }
                Projection::ConstantIndex(index) => {
                    write!(f, "[{}]", index)?;
                }
                Projection::Deref => {
                    write!(f, "^")?;
                }
            }
        }
        Ok(())
    }
}



#[derive(Debug, Clone, Eq, PartialEq)]
pub enum MIRType {
    Integer(IntSize),
    Bool,
    Char,
    Struct(Vec<MIRType>),
    Ptr(Box<MIRType>),
    Void,
}

impl MIRType {
    pub fn from_type(ty: &Type, global_scope: &GlobalScope) -> Self {
        match ty {
            Type::Integer(size) => MIRType::Integer(*size),
            Type::Bool => MIRType::Bool,
            Type::Char => MIRType::Char,
            Type::Void => MIRType::Void,
            Type::Ptr(to, _) => MIRType::Ptr(Box::new(Self::from_type(to, global_scope))),
            Type::Struct(id) => {
                let struct_ = global_scope.get_struct(id);
                let field_types = struct_.fields.iter()
                    .map(|field| {
                        let field = global_scope.get_field(field);
                        Self::from_type(&field.ty, global_scope)
                    })
                    .collect();
                MIRType::Struct(field_types)
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
        }
    }

    pub fn layout(&self) -> Layout {
        match self {
            MIRType::Integer(size) => match size {
                IntSize::I8 => Layout {
                    size: 1,
                    alignment: 1,
                },
                IntSize::I16 => Layout {
                    size: 2,
                    alignment: 2,
                },
                IntSize::I32 => Layout {
                    size: 4,
                    alignment: 4,
                },
                IntSize::I64 => Layout {
                    size: 8,
                    alignment: 8,
                },
                IntSize::ISize => Layout {
                    size: 8,
                    alignment: 8,
                },
            },
            MIRType::Bool => {
                Layout {
                    size: 1,
                    alignment: 1,
                }
            }
            MIRType::Char => {
                Layout {
                    size: 1,
                    alignment: 1,
                }
            }
            MIRType::Struct(fields) => {
                let mut size = 0;
                let mut alignment = 0;
                for field in fields {
                    let field_layout = field.layout();
                    size += field_layout.size;
                    alignment = std::cmp::max(alignment, field_layout.alignment);
                }
                Layout {
                    size,
                    alignment,
                }
            }
            MIRType::Ptr(_) => {
                Layout {
                    size: 8,
                    alignment: 8,
                }
            }
            // todo
            MIRType::Void => {
                Layout {
                    size: 8,
                    alignment: 8,
                }
            }
        }
    }

    pub fn deref_type(&self) -> Option<&MIRType> {
        match self {
            MIRType::Ptr(ty) => {
                Some(ty)
            }
            _ => {
                None
            }
        }
    }
}

impl Display for MIRType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            MIRType::Integer(size) => {
                write!(f, "{}", size)
            }
            MIRType::Bool => {
                write!(f, "bool")
            }
            MIRType::Char => {
                write!(f, "char")
            }
            MIRType::Struct(fields) => {
                write!(f, "struct {{ ")?;
                for field in fields {
                    write!(f, "{} ", field)?;
                }
                write!(f, "}}")
            }
            MIRType::Void => {
                write!(f, "void")
            }
            MIRType::Ptr(ty) => {
                write!(f, "*{}", ty)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Scalar {
    pub data: u64,
    pub size: u8,
}

impl Scalar {
    pub fn new(data: u64, size: u8) -> Self {
        Self {
            data,
            size,
        }
    }

    pub fn from_i8(data: i8) -> Self {
        Self {
            data: data as u64,
            size: 1,
        }
    }

    pub fn from_i16(data: i16) -> Self {
        Self {
            data: data as u64,
            size: 2,
        }
    }

    pub fn from_i32(data: i32) -> Self {
        Self {
            data: data as u64,
            size: 4,
        }
    }

    pub fn from_i64(data: i64) -> Self {
        Self {
            data: data as u64,
            size: 8,
        }
    }

    pub fn from_isize(data: isize) -> Self {
        Self {
            data: data as u64,
            size: 8,
        }
    }

    pub fn from_char(data: char) -> Self {
        Self {
            data: u64::from(data as u8),
            size: 1,
        }
    }

    pub fn from_bool(data: bool) -> Self {
        Self {
            data: if data { 1 } else { 0 },
            size: 1,
        }
    }

    pub fn into_i8(&self) -> Option<i8> {
        if self.size == 1 {
            Some(self.data as i8)
        } else {
            None
        }
    }

    pub fn into_i16(&self) -> Option<i16> {
        if self.size == 2 {
            Some(self.data as i16)
        } else {
            None
        }
    }

    pub fn into_i32(&self) -> Option<i32> {
        if self.size == 4 {
            Some(self.data as i32)
        } else {
            None
        }
    }

    pub fn into_i64(&self) -> Option<i64> {
        if self.size == 8 {
            Some(self.data as i64)
        } else {
            None
        }
    }
}

impl Display for Scalar {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{:x} => {{ size: {} }}", self.data, self.size)
    }
}

#[derive(Debug, Clone)]
pub enum ConstantValue {
    Scalar(Scalar),
    ZeroSized,
}

impl Display for ConstantValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ConstantValue::Scalar(scalar) => {
                write!(f, "{}", scalar)
            }
            ConstantValue::ZeroSized => {
                write!(f, "zero sized")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Operand {
    Copy(Place),
    Constant(ConstantValue),
}

impl Display for Operand {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Operand::Copy(place) => {
                write!(f, "{}", place)
            }
            Operand::Constant(constant) => {
                write!(f, "{}", constant)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Mutability {
    Mutable,
    Immutable,
}

impl Display for Mutability {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Mutability::Mutable => {
                write!(f, "mut")
            }
            Mutability::Immutable => {
                write!(f, "imut")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    And,
    Or,
    Xor,
    Shl,
    Shr,
    Eq,
    Neq,
    Lt,
    Gt,
    Leq,
    Geq,
}

impl Display for BinOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            BinOp::Add => {
                write!(f, "+")
            }
            BinOp::Sub => {
                write!(f, "-")
            }
            BinOp::Mul => {
                write!(f, "*")
            }
            BinOp::Div => {
                write!(f, "/")
            }
            BinOp::Mod => {
                write!(f, "%")
            }
            BinOp::And => {
                write!(f, "&")
            }
            BinOp::Or => {
                write!(f, "|")
            }
            BinOp::Xor => {
                write!(f, "^")
            }
            BinOp::Shl => {
                write!(f, "<<")
            }
            BinOp::Shr => {
                write!(f, ">>")
            }
            BinOp::Eq => {
                write!(f, "==")
            }
            BinOp::Neq => {
                write!(f, "!=")
            }
            BinOp::Lt => {
                write!(f, "<")
            }
            BinOp::Gt => {
                write!(f, ">")
            }
            BinOp::Leq => {
                write!(f, "<=")
            }
            BinOp::Geq => {
                write!(f, ">=")
            }
        }
    }
}


#[derive(Debug, Clone)]
pub enum UnOp {
    Neg,
    Not,
}

impl Display for UnOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            UnOp::Neg => {
                write!(f, "-")
            }
            UnOp::Not => {
                write!(f, "!")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Rvalue {
    AddressOf(Mutability, Place),
    Use(Operand),
    Struct(Vec<(FieldIdx, Operand)>),
    BinaryOp(BinOp, (Operand, Operand)),
    UnaryOp(UnOp, Operand),
    Global(GlobalIdx),
}

impl Display for Rvalue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Rvalue::Struct(values) => {
                write!(f, "{{")?;
                for (i, (id, op)) in values.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", id.as_idx(), op)?;
                }
                write!(f, "}}")
            }
            Rvalue::AddressOf(m, place) => {
                write!(f, "&{} {}", m, place)
            }
            Rvalue::Use(op) => {
                write!(f, "{}", op)
            }
            Rvalue::BinaryOp(op, (lhs, rhs)) => {
                write!(f, "({} {} {})", lhs, op, rhs)
            }
            Rvalue::UnaryOp(op, operand) => {
                write!(f, "({}{})", op, operand)
            }
            Rvalue::Global(idx) => {
                write!(f, "{}", idx)
            }
        }
    }
}

#[derive(Debug)]
pub enum InstructionKind {
    Assign { place: Place, value: Rvalue },
    Call { return_value_place: Place, function_id: FunctionIdx, args: Vec<Operand> },
    StorageLive { local: LocalIdx },
    StorageDead { local: LocalIdx },
    PlaceMention(Place),
}

#[derive(Debug)]
pub struct Terminator {
    pub kind: TerminatorKind,
    pub span: Option<TextSpan>,
}

impl Terminator {
    pub fn new(kind: TerminatorKind, span: Option<TextSpan>) -> Self {
        Self {
            kind,
            span,
        }
    }

    pub fn return_(span: TextSpan) -> Self {
        Self::new(TerminatorKind::Return, Some(span))
    }

    pub fn goto(bb: BasicBlockIdx) -> Self {
        Self::new(TerminatorKind::Goto(bb), None)
    }

    pub fn if_(span: TextSpan, condition: Operand, then: BasicBlockIdx, else_: BasicBlockIdx) -> Self {
        Self::new(TerminatorKind::If {
            condition,
            then,
            else_,
        }, Some(span))
    }
}

#[derive(Debug)]
pub enum TerminatorKind {
    Goto(BasicBlockIdx),
    If { condition: Operand, then: BasicBlockIdx, else_: BasicBlockIdx },
    Return,
    Next,
    Unresolved,
}

idx!(BodyIdx);

pub struct MIR {
    pub _bodies: IdxVec<BodyIdx, Body>,
    pub globals: IdxVec<GlobalIdx, Global>,
}

impl MIR {
    pub fn new() -> Self {
        Self {
            _bodies: IdxVec::new(),
            globals: IdxVec::new(),
        }
    }

    pub fn output_graphviz(
        &self,
        scope: &GlobalScope,
        filename: &str,
    ) {
        let mut file = std::fs::File::create(filename).unwrap();
        file.write_all(self.graphviz(scope).as_bytes()).unwrap();
    }

    pub fn graphviz(&self, scope: &GlobalScope) -> String {
        let mut s = String::new();
        s.push_str("digraph {\n");
        for body in self.sorted_bodies(scope).iter() {
            let function = scope.get_function(&body.function);
            s.push_str(&format!("  subgraph cluster_{} {{\n", body.function.as_idx()));
            s.push_str(&format!("    label = \"{}\";\n", function.name));
            for (index, bb) in body.basic_blocks.indexed_iter() {
                let mut instructions = format!("{}:\\n", index);
                instructions.push_str(bb.instructions.iter().map(|i| i.format(scope)).collect::<Vec<_>>().join("\\n").as_str());
                match &bb.terminator.kind {
                    TerminatorKind::Goto(label) => {
                        s.push_str(&format!("    {} -> {};\n", index, label));
                    }
                    TerminatorKind::If { condition: cond, then: then_label, else_: else_label } => {
                        // create a branch block
                        instructions.push_str(format!("\\nif {} then goto bb{} else goto bb{}", cond, then_label, else_label).as_str());
                        s.push_str(&format!("    {} -> {};\n", index, then_label));
                        s.push_str(&format!("    {} -> {};\n", index, else_label));
                    }
                    TerminatorKind::Return => {
                        instructions.push_str(format!("\\nreturn").as_str());
                    }
                    TerminatorKind::Next => {
                        s.push_str(&format!("    {} -> {};\n", index, index.add(1)));
                    }
                    TerminatorKind::Unresolved => {}
                }
                s.push_str(&format!("    {} [label=\"{}\"];\n", index, instructions));
            }
            s.push_str("  }\n");
        }
        s.push_str("}\n");
        s
    }

    pub fn save_output(&self, scope: &GlobalScope, filename: &str) {
        let mut file = std::fs::File::create(filename).unwrap();
        file.write_all(self.output(scope).as_bytes()).unwrap();
    }

    pub fn output(&self, scope: &GlobalScope) -> String {
        let mut s = String::new();
        for (idx, global) in self.globals.indexed_iter() {
            s.push_str(&format!("{}@{} = {};\n", idx, global.ty, global.value));
        }
        for body in self.sorted_bodies(scope).iter() {
            let function = scope.get_function(&body.function);
            s.push_str(&format!("{}:\n", function.name));
            let scope_string = self.format_scopes(&body);
            s.push_str(&format!("{}\n", scope_string));
            for (index, bb) in body.basic_blocks.indexed_iter() {
                s.push_str(&format!("  {}:\n", index));
                for instruction in &bb.instructions {
                    s.push_str(&format!("    {}\n", instruction.format(scope)));
                }
                match &bb.terminator.kind {
                    TerminatorKind::Goto(label) => {
                        s.push_str(&format!("    goto {}\n", label));
                    }
                    TerminatorKind::If { condition: cond, then: then_label, else_: else_label } => {
                        s.push_str(&format!("    if {} then goto {} else goto {}\n", cond, then_label, else_label));
                    }
                    TerminatorKind::Return => {
                        s.push_str(&format!("    return\n"));
                    }
                    TerminatorKind::Next => {}
                    TerminatorKind::Unresolved => {}
                }
            }
        }
        s
    }

    fn format_scopes(&self, body: &Body) -> String {
        let mut s = String::new();
        self.format_scope(&mut s, &body.scope, LocalScopeIdx::new(0), 2);
        s
    }

    fn format_scope(&self, s: &mut String, scope: &BodyScope, local_scope_idx: LocalScopeIdx, indent: usize) {
        let local_scope = scope.scopes.get(local_scope_idx);
        s.push_str(&format!("{}{} {{\n", "  ".repeat(indent), local_scope_idx));
        for id in local_scope.locals.iter() {
            let local = scope.locals.get(*id);
            s.push_str(&format!("{}", "  ".repeat(indent + 1)));
            match local.variable_idx.as_ref() {
                None => {
                    s.push_str(&format!("  {} = let {}\n", id, local.ty));
                }
                Some(variable_idx) => {
                    let scope = scope.scope.borrow();
                    let var = scope.get_variable(variable_idx);
                    s.push_str(&format!("  {} = let {}: {}\n", id, var.name, var.ty));
                }
            }
        }
        for child in local_scope.children.iter() {
            self.format_scope(s, scope, *child, indent + 1);
        }
        s.push_str(&format!("{}}}\n", "  ".repeat(indent)));
    }

    pub fn interpret(&self, scope: &GlobalScope) {
        // let mut interpreter = Interpreter::new(self, scope);
        // interpreter.interpret();
    }

    pub fn sorted_bodies(&self,global_scope: &GlobalScope) -> Vec<&Body> {
        let mut bodies = self._bodies.iter().collect::<Vec<_>>();
        bodies.sort_by_key(|b| {
            let function = global_scope.get_function(&b.function);
            &function.name.name
        });
        bodies
    }
}

pub struct MIRGen {
    pub ir: MIR,
    pub scope: GlobalScopeCell,
    diagnostics: DiagnosticsBagCell,
    global_scope: Rc<RefCell<GlobalMIRScope>>,
}

impl MIRGen {
    pub fn new(
        diagnostics: DiagnosticsBagCell,
        scope: GlobalScopeCell,
    ) -> Self {
        Self {
            ir: MIR::new(),
            diagnostics,
            scope,
            global_scope: Rc::new(RefCell::new(GlobalMIRScope::new())),
        }
    }

    pub fn construct(mut self, hir: &HIR) -> MIR {
        self.construct_globals(hir);
        self.construct_functions(hir);
        let globals = &self.global_scope.borrow().globals;
        self.ir.globals = globals.clone();
        self.ir
    }

    fn construct_globals(&mut self, hir: &HIR) {
        // todo
    }

    fn construct_functions(&mut self, hir: &HIR) {
        let scope = &self.scope.borrow();
        for (function_idx, body) in hir.function_bodies.iter() {
            let function = scope.get_function(function_idx);
            if function.is_extern() {
                continue;
            }
            self.ir._bodies.push(self.construct_function(*function_idx, body));
        }
    }

    pub fn construct_function(&self, function: FunctionIdx, statements: &Vec<HIRStatement>) -> Body {
        let body = Body::new(function, self.scope.clone());

        let body_gen = BodyGen::new(body, self.scope.clone(), self.global_scope.clone());
        body_gen.gen(statements)
    }
}

pub struct BodyGen {
    pub body: Body,
    pub current_block: BasicBlockIdx,
    pub scope: GlobalScopeCell,
    pub global_scope: Rc<RefCell<GlobalMIRScope>>,
}

impl BodyGen {
    pub fn new(mut body: Body, scope: GlobalScopeCell,
               global_scope: Rc<RefCell<GlobalMIRScope>>,
    ) -> Self {
        let idx = body.basic_blocks.push(BasicBlock::new());
        Self {
            body,
            current_block: idx,
            scope,
            global_scope,
        }
    }

    fn gen(mut self, statements: &Vec<HIRStatement>) -> Body {
        for arg_idx in 1..self.body.scope.arg_count + 1 {
            self.start_var_lifetime(LocalIdx::new(arg_idx));
        }
        self.gen_stmts(statements);
        let bb_len = self.body.basic_blocks.len();
        for (idx, bb) in self.body.basic_blocks.iter_mut().enumerate() {
            match &bb.terminator.kind {
                TerminatorKind::Unresolved => {
                    if idx == bb_len - 1 {
                        bb.terminator.kind = TerminatorKind::Return;
                    } else {
                        bb.terminator.kind = TerminatorKind::Next;
                    }
                }
                _ => {}
            }
        }
        self.exit_scope();
        self.body
    }

    fn gen_stmts(&mut self, statements: &Vec<HIRStatement>) {
        for stmt in statements {
            self.gen_stmt(stmt);
        }
    }

    fn gen_stmt(&mut self, stmt: &HIRStatement) {
        match &stmt.kind {
            HIRStatementKind::VariableDeclaration(variable_declaration) => {
                let expr = &variable_declaration.initializer;
                let var_place = self.get_place_for_variable(&variable_declaration.variable_id);
                let value = self.gen_as_rvalue(expr);
                self.push_instruction(Instruction {
                    span: stmt.span.clone(),
                    kind: InstructionKind::Assign {
                        value,
                        place: var_place,
                    },
                })
            }
            HIRStatementKind::Expression(expr) => {
                let place = self.gen_as_place(&expr.expression);
                self.push_instruction(Instruction {
                    span: stmt.span.clone(),
                    kind: InstructionKind::PlaceMention(place),
                })
            }
            HIRStatementKind::If(if_stmt) => {
                let cond_value = self.gen_as_operand(&if_stmt.condition);
                self.enter_scope();
                let current_block = self.current_block;
                let then_block = self.push_basic_block();
                let else_block = if_stmt.else_.as_ref().map(|_| self.push_basic_block());
                let end_block = self.push_basic_block();
                self.set_current_block(current_block);
                let terminator = Terminator::if_(stmt.span.clone(), cond_value, then_block, else_block.clone().unwrap_or(end_block));
                self.push_terminator(terminator);
                self.set_current_block(then_block);
                self.gen_stmts(&if_stmt.then);
                self.push_terminator(Terminator {
                    span: Some(stmt.span.clone()),
                    kind: TerminatorKind::Goto(end_block),
                });
                self.exit_scope();
                if let Some(else_block) = else_block {
                    self.enter_scope();
                    self.set_current_block(else_block);
                    self.gen_stmts(&if_stmt.else_.as_ref().unwrap());
                    self.exit_scope();
                }
                self.set_current_block(end_block);
            }
            HIRStatementKind::Block(stmt) => {
                self.enter_scope();
                self.gen_stmts(&stmt.statements);
                self.exit_scope();
            }
            HIRStatementKind::While(while_stmt) => {
                let condition_block = self.push_basic_block();
                let body_block = self.push_basic_block();
                let end_block = self.push_basic_block();
                self.set_current_block(condition_block);
                let condition = self.gen_as_operand(&while_stmt.condition);
                let terminator = Terminator::if_(stmt.span.clone(), condition, body_block, end_block);
                self.push_terminator(terminator);
                self.set_current_block(body_block);
                self.enter_scope();
                self.gen_stmts(&while_stmt.body);
                self.exit_scope();
                let terminator = Terminator::goto(condition_block);
                self.push_terminator(terminator);
                self.set_current_block(end_block);
            }

            HIRStatementKind::Return(return_stmt) => {
                let return_place = self.body.scope.return_place();
                self.start_var_lifetime(return_place.local);
                let value = self.gen_as_rvalue(&return_stmt.expression);
                self.push_instruction(Instruction {
                    span: stmt.span.clone(),
                    kind: InstructionKind::Assign {
                        value,
                        place: return_place,
                    },
                });
                let terminator = Terminator::return_(stmt.span.clone());
                self.push_terminator(terminator);

            }
        }
    }

    pub fn gen_as_operand(&mut self, expr: &HIRExpression) -> Operand {
        let category = Category::from_expr_kind(&expr.kind);
        match category {
            Category::RValue | Category::Place => {
                Operand::Copy(self.gen_as_temp(expr))
            }
            Category::Constant => {
                Operand::Constant(self.gen_as_constant(expr))
            }
        }
    }


    pub fn gen_as_constant(&self, expr: &HIRExpression) -> ConstantValue {
        match &expr.kind {
            HIRExpressionKind::Literal(lit_expr) => {
                match &lit_expr.value {
                    HIRLiteralValue::Integer(value) =>
                        {
                            let scalar = match value {
                                IntegerLiteralValue::I8(value) => Scalar::from_i8(*value),
                                IntegerLiteralValue::I16(value) => Scalar::from_i16(*value),
                                IntegerLiteralValue::I32(value) => Scalar::from_i32(*value),
                                IntegerLiteralValue::I64(value) => Scalar::from_i64(*value),
                                IntegerLiteralValue::ISize(value) => Scalar::from_isize(*value),
                            };
                            ConstantValue::Scalar(scalar)
                        }

                    HIRLiteralValue::Boolean(value) =>
                        ConstantValue::Scalar(Scalar::from_bool(*value)),
                    HIRLiteralValue::String(_) =>
                        unreachable!("String literal should be lowered to a global variable and be used through Rvalue::Global"),
                    HIRLiteralValue::Char(value) =>
                        ConstantValue::Scalar(Scalar::from_char(*value)),
                }
            }
            _ => {
                panic!("Not a constant expression")
            }
        }
    }

    pub fn gen_as_temp(&mut self, expr: &HIRExpression) -> Place {
        let category = Category::from_expr_kind(&expr.kind);
        match category {
            Category::Place => self.gen_as_place(expr),
            Category::Constant |
            Category::RValue => {
                let mut temp = self.new_temp_place(&expr.ty);
                self.gen_expr_into(expr, &mut temp);
                temp
            }
        }
    }

    pub fn gen_as_place(&mut self, expr: &HIRExpression) -> Place {
        let category = Category::from_expr_kind(&expr.kind);
        match category {
            Category::Place => {
                match &expr.kind {
                    HIRExpressionKind::Variable(var_expr) => {
                        let local = self.body.scope.get_variable(&var_expr.variable_id).unwrap();
                        Place {
                            local: *local,
                            projection: None,
                        }
                    }
                    HIRExpressionKind::Call(_) |


                    HIRExpressionKind::FieldAccess(_) |
                    HIRExpressionKind::Ref(_) |
                    HIRExpressionKind::Deref(_) |
                    HIRExpressionKind::Index(_) => {
                        let mut place = self.new_temp_place(&expr.ty);
                        self.gen_expr_into(expr, &mut place);
                        place
                    }
                    _ => {
                        panic!("Cannot use expression as place")
                    }
                }
            }
            Category::Constant => panic!("Cannot use constant as place"),
            Category::RValue => {
                self.gen_as_temp(expr)
            }
        }
    }

    pub fn gen_expr_into(&mut self, expr: &HIRExpression, place: &mut Place) {
        match &expr.kind {
            HIRExpressionKind::FieldAccess(field_access_expr) => {
                let mut base = self.gen_as_place(&field_access_expr.target);
                base.projection = Some(Projection::Field(field_access_expr.field_id));
                *place = base;
            }
            HIRExpressionKind::Deref(deref_expr) => {
                let mut base = self.gen_as_place(&deref_expr.target);
                base.projection = Some(Projection::Deref);
                *place = base;
            }
            HIRExpressionKind::Index(index_expr) => {
                let base = self.gen_as_place(&index_expr.target);
                let index = self.gen_as_place(&index_expr.index);
                let projection = Projection::Index(index.local);
                *place = Place {
                    local: base.local,
                    projection: Some(projection),
                };
            }
            HIRExpressionKind::Call(call_expr) => {
                let function_id = match &call_expr.callee {
                    HIRCallee::Function(function_id) => *function_id,
                    _ => panic!("Cannot call non-function"),
                };
                let args = call_expr.args.iter().map(|arg| self.gen_as_operand(arg)).collect();
                self.push_instruction(
                    Instruction {
                        kind: InstructionKind::Call {
                            function_id,
                            args,
                            return_value_place: place.clone(),
                        },
                        span: expr.span.clone(),
                    }
                );
            }
            _ => {
                let rvalue = self.gen_as_rvalue(expr);
                self.push_instruction(Instruction {
                    span: expr.span.clone(),
                    kind: InstructionKind::Assign {
                        value: rvalue,
                        place: place.clone(),
                    },
                });
            }
        }
    }

    pub fn gen_as_rvalue(&mut self, expr: &HIRExpression) -> Rvalue {
        match &expr.kind {
            HIRExpressionKind::Assignment(assign_expr) => {
                let place = self.gen_as_place(&assign_expr.target);
                let value = self.gen_as_rvalue(&assign_expr.value);
                self.push_instruction(Instruction {
                    span: expr.span.clone(),
                    kind: InstructionKind::Assign {
                        place: place.clone(),
                        value,
                    },
                });
                Rvalue::Use(Operand::Copy(place))
            }
            HIRExpressionKind::Binary(bin_expr) => {
                let left = self.gen_as_operand(&bin_expr.left);
                let right = self.gen_as_operand(&bin_expr.right);
                let op = match bin_expr.op {
                    HIRBinaryOperator::Add => BinOp::Add,
                    HIRBinaryOperator::Subtract => BinOp::Sub,
                    HIRBinaryOperator::Multiply => BinOp::Mul,
                    HIRBinaryOperator::Divide => BinOp::Div,
                    HIRBinaryOperator::Modulo => BinOp::Mod,
                    HIRBinaryOperator::BitwiseAnd => BinOp::And,
                    HIRBinaryOperator::BitwiseOr => BinOp::Or,
                    HIRBinaryOperator::BitwiseXor => BinOp::Xor,
                    HIRBinaryOperator::Equals => BinOp::Eq,
                    HIRBinaryOperator::NotEquals => BinOp::Neq,
                    HIRBinaryOperator::LessThan => BinOp::Lt,
                    HIRBinaryOperator::LessThanOrEqual => BinOp::Leq,
                    HIRBinaryOperator::GreaterThan => BinOp::Gt,
                    HIRBinaryOperator::GreaterThanOrEqual => BinOp::Geq,
                    HIRBinaryOperator::LogicalAnd => BinOp::And,
                };
                Rvalue::BinaryOp(op, (left, right))
            }
            HIRExpressionKind::Unary(un_expr) => {
                let operand = self.gen_as_operand(&un_expr.operand);
                let op = match un_expr.op {
                    HIRUnaryOperator::Negate => UnOp::Neg,
                    HIRUnaryOperator::BitwiseNot => UnOp::Not,
                };
                Rvalue::UnaryOp(op, operand)
            }
            HIRExpressionKind::StructInit(struct_init_expr) => {
                let mut fields = vec![];
                for field in &struct_init_expr.fields {
                    fields.push((field.field_id, self.gen_as_operand(&field.value)));
                }
                Rvalue::Struct(fields)
            }
            HIRExpressionKind::Ref(expr) => {
                Rvalue::AddressOf(Mutability::Immutable, self.gen_as_place(&expr.expression))
            }
            HIRExpressionKind::Cast(_) => {
                unimplemented!("Cast")
            }
            HIRExpressionKind::Literal(
                HIRLiteralExpression {
                    value: HIRLiteralValue::String(value),
                    ..
                },
            ) => {
                Rvalue::Global(self.global_scope.borrow_mut().add_global_string(value.clone()))
            }
            _ => Rvalue::Use(self.gen_as_operand(expr)),
        }
    }

    pub fn push_basic_block(&mut self) -> BasicBlockIdx {
        self.body.basic_blocks.push(BasicBlock::new())
    }

    pub fn current_block(&self) -> &BasicBlock {
        self.body.basic_blocks.get(self.current_block)
    }

    pub fn current_block_mut(&mut self) -> &mut BasicBlock {
        self.body.basic_blocks.get_mut(self.current_block)
    }

    pub fn set_current_block(&mut self, basic_block: BasicBlockIdx) {
        self.current_block = basic_block;
    }

    pub fn push_instruction(&mut self, instruction: Instruction) {
        self.current_block_mut().instructions.push(instruction);
    }

    pub fn push_terminator(&mut self, terminator: Terminator) {
        match &self.current_block_mut().terminator.kind {
            TerminatorKind::Unresolved => {
                self.current_block_mut().terminator = terminator;
            }
            _ => {}
        }
    }


    fn enter_scope(&mut self) {
        self.body.scope.enter_scope();
    }

    fn exit_scope(&mut self) {
        let locals = match &self.current_block().terminator.kind {
            TerminatorKind::Return => {
                self.body.scope.alive_locals()
            }
            _ => {
                let mut locals = self.body.scope.current_scope().locals.clone();
                locals.reverse();
                locals
            }
        };
        for local in locals {
            self.end_var_lifetime(local);
        }
        self.body.scope.exit_scope();
    }

    fn new_temp_place(&mut self, ty: &Type) -> Place {
        let local = self.body.scope.new_local(ty);
        self.start_var_lifetime(local);
        Place {
            local,
            projection: None,
        }
    }

    fn get_place_for_variable(&mut self, variable: &VariableIdx) -> Place {
        let local = self.body.scope.new_mapped_local(variable);
        self.start_var_lifetime(local);
        Place {
            local,
            projection: None,
        }
    }

    fn start_var_lifetime(&mut self, local: LocalIdx) {
        self.push_instruction(Instruction {
            kind: InstructionKind::StorageLive {
                local,
            },
            span: TextSpan::new(0, 0, "".into()),
        });
    }

    fn end_var_lifetime(&mut self, local: LocalIdx) {
        if local == Place::RETURN_PLACE {
            return;
        }
        self.push_instruction(Instruction {
            kind: InstructionKind::StorageDead {
                local,
            },
            span: TextSpan::new(0, 0, "".into()),
        });
    }
}

pub enum Category {
    Place,
    Constant,
    RValue,
}

impl Category {
    pub fn from_expr_kind(expr: &HIRExpressionKind) -> Self {
        match expr {
            HIRExpressionKind::Literal(literal) => {
                match &literal.value {
                    HIRLiteralValue::String(_) => Category::RValue,
                    _ => Category::Constant,
                }
            }
            HIRExpressionKind::Void => Category::Constant,
            HIRExpressionKind::Variable(_) |
            HIRExpressionKind::FieldAccess(_) |
            HIRExpressionKind::Index(_) |
            HIRExpressionKind::Deref(_) => Category::Place,
            HIRExpressionKind::Unary(_) |
            HIRExpressionKind::Binary(_) |
            HIRExpressionKind::Call(_) |
            HIRExpressionKind::Cast(_) |
            HIRExpressionKind::StructInit(_) |
            HIRExpressionKind::Assignment(_) |
            HIRExpressionKind::Ref(_) => Category::RValue,
        }
    }
}


