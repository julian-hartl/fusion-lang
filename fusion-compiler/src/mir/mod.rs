use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::io::Write;
use std::rc::Rc;

use fusion_compiler::{id, id_generator};

use crate::diagnostics::DiagnosticsBagCell;
use crate::hir;
use crate::hir::{Function, FunctionId, HIR, HIRAssignmentTargetKind, HIRBinaryOperator, HIRCallee, HIRExpression, HIRExpressionKind, HIRFunction, HIRGlobal, HIRLiteralValue, HIRStatement, HIRStatementKind, HIRUnaryOperator, VariableId};
use crate::interpreter::Interpreter;
use crate::text::span::TextSpan;
use crate::typings::{Layout, Type};

#[derive(Debug)]
pub struct Body {
    pub basic_blocks: Vec<BasicBlock>,
    pub scope: MIRScope,
    pub function: FunctionId,
    label_gen: Rc<RefCell<LabelGenerator>>,
}

impl Body {
    pub fn new(
        function: FunctionId,
        label_gen: Rc<RefCell<LabelGenerator>>,
    ) -> Self {
        Self {
            scope: MIRScope::new(),
            basic_blocks: vec![],
            function,
            label_gen,

        }
    }

    pub fn new_basic_block(&mut self) -> Label {
        self.label_gen.borrow_mut().next()
    }

    pub fn find_basic_block(&self, label: &Label) -> Option<&BasicBlock> {
        self.basic_blocks.iter()
            .find(|block| block.label == *label)
    }
}

id!(Label);
id_generator!(LabelGenerator, Label);

impl Display for Label {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "L{}", self.index)
    }
}

#[derive(Debug)]
pub struct BasicBlock {
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
    pub label: Label,
}

impl BasicBlock {
    pub fn new(label: Label) -> Self {
        Self {
            instructions: vec![],
            terminator: Terminator {
                kind: TerminatorKind::Next,
                span: None,
            },
            label,
        }
    }
}

#[derive(Debug)]
pub struct MIRScope {
    pub variable_count: usize,
    mapping: HashMap<VariableId, usize>,
}

impl MIRScope {
    pub fn new() -> Self {
        Self {
            variable_count: 0,
            mapping: HashMap::new(),
        }
    }

    pub fn new_temp_variable(&mut self) -> usize {
        self.variable_count += 1;
        self.variable_count - 1
    }

    pub fn new_variable(&mut self, id: &VariableId) -> usize {
        let temp = self.new_temp_variable();
        self.mapping.insert(*id, temp);
        temp
    }

    pub fn get_variable(&self, id: &VariableId) -> Option<usize> {
        self.mapping.get(id).copied()
    }
}

#[derive(Debug)]
pub struct Instruction {
    pub kind: InstructionKind,
    pub span: TextSpan,
    pub assign_to: Option<usize>,
}

impl Instruction {
    fn format(
        &self,
        hir: &HIR,
    ) -> String {
        let instr = match &self.kind {
            InstructionKind::Store(ptr, primary) => {
                format!("store {}, {}", ptr.format(), primary.format())
            }
            InstructionKind::Call(ptr, args) => {
                let mut s = format!("@{}(", hir.scope.get_function(ptr).name);
                for (i, arg) in args.iter().enumerate() {
                    if i != 0 {
                        s.push_str(", ");
                    }
                    s.push_str(&arg.format());
                }
                s.push_str(")");
                s
            }
            InstructionKind::Primary(primary) => {
                primary.format()
            }
            InstructionKind::BinaryOp(op, lhs, rhs, _) => {
                format!("{} {} {}", self.format_bin_op(op), lhs.0.format(), rhs.0.format())
            }
            InstructionKind::UnaryOp(op, primary, _) => {
                format!("{} {}", self.format_un_op(op), primary.format())
            }
            InstructionKind::Load(expr) => {
                format!("load {}", expr.format())
            }
            InstructionKind::GetAddress(expr) => {
                format!("getaddr {}", expr.format())
            }
            InstructionKind::Cast(expr, ty) => {
                format!("cast {} to {}", expr.format(), ty)
            }
        };
        if let Some(assign_to) = self.assign_to {
            format!("%{} = {}", assign_to, instr)
        } else {
            instr
        }
    }

    fn format_bin_op(
        &self,
        op: &HIRBinaryOperator,
    ) -> &'static str {
        match op {
            HIRBinaryOperator::Add => {
                "add"
            }
            HIRBinaryOperator::Subtract => {
                "sub"
            }
            HIRBinaryOperator::Multiply => {
                "mul"
            }
            HIRBinaryOperator::Divide => {
                "div"
            }
            HIRBinaryOperator::Equals => {
                "eq"
            }
            HIRBinaryOperator::NotEquals => {
                "ne"
            }
            HIRBinaryOperator::LessThan => {
                "lt"
            }
            HIRBinaryOperator::LessThanOrEqual => {
                "le"
            }
            HIRBinaryOperator::GreaterThan => {
                "gt"
            }
            HIRBinaryOperator::GreaterThanOrEqual => {
                "ge"
            }
            HIRBinaryOperator::BitwiseAnd => {
                "and"
            }
            HIRBinaryOperator::BitwiseOr => {
                "or"
            }
            HIRBinaryOperator::BitwiseXor => {
                "xor"
            }
            HIRBinaryOperator::Modulo => {
                "mod"
            }
        }
    }

    fn format_un_op(
        &self,
        op: &HIRUnaryOperator,
    ) -> &'static str {
        match op {
            HIRUnaryOperator::Negate => {
                "neg"
            }
            HIRUnaryOperator::BitwiseNot => {
                "not"
            }
        }
    }
}

#[derive(Debug)]
pub enum InstructionKind {
    Store(MemoryPointer, Primary),
    Call(FunctionId, Vec<Primary>),
    Primary(Primary),
    BinaryOp(HIRBinaryOperator, (Primary, Type), (Primary, Type), Type),
    UnaryOp(HIRUnaryOperator, Primary, Type),
    Load(MemoryPointer),
    GetAddress(Primary),
    Cast(Primary, Type),
}

#[derive(Debug)]
pub enum MemoryPointer {
    Variable(usize, Type),
    Primary(Primary, Type),
}

impl MemoryPointer {
    pub fn format(
        &self,
    ) -> String {
        match self {
            MemoryPointer::Variable(id, ty) => {
                format!("%{}@{}", id, ty)
            }
            MemoryPointer::Primary(primary, ty) => {
                format!("{} ptr@{}", primary.format(), ty)
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Primary {
    Bool(bool),
    I64(i64),
    Str(String),
    Char(char),
    Variable(usize, Type),
    Void,
}

impl Primary {
    fn format(
        &self,
    ) -> String {
        match self {
            Primary::Bool(b) => {
                format!("{}", b)
            }
            Primary::I64(i) => {
                format!("{}", i)
            }
            Primary::Str(s) => {
                format!("\\\"{}\\\"", s)
            }

            Primary::Variable(id, ty) => {
                format!("%{}@{}", id, ty)
            }
            Primary::Void => {
                format!("void")
            }
            Primary::Char(value) => {
                format!("'{}'", value)
            }
        }
    }

    pub fn ty(&self) -> Type {
        match self {
            Primary::Bool(_) => {
                Type::Bool
            }
            Primary::I64(_) => {
                Type::I64
            }
            Primary::Str(_) => {
                Type::StringSlice(true)
            }
            Primary::Variable(_, ty) => {
                ty.clone()
            }
            Primary::Void => {
                Type::Void
            }
            Primary::Char(_) => {
                Type::Char
            }
        }
    }
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

    pub fn return_(span: TextSpan, value: Primary) -> Self {
        Self::new(TerminatorKind::Return(value), Some(span))
    }

    pub fn goto(label: Label) -> Self {
        Self::new(TerminatorKind::Goto(label), None)
    }

    pub fn if_(span: TextSpan, condition: Primary, then_label: Label, else_label: Label) -> Self {
        Self::new(TerminatorKind::If(condition, then_label, else_label), Some(span))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum TerminatorKind {
    Goto(Label),
    If(Primary, Label, Label),
    Return(Primary),
    Next,
}

pub struct MIR {
    pub bodies: Vec<Body>,
    pub globals: Vec<Global>,
}

pub enum Global {
    Variable(usize, Vec<Instruction>, Primary),
}

impl MIR {
    pub fn new() -> Self {
        Self {
            bodies: Vec::new(),
            globals: Vec::new(),
        }
    }

    pub fn output_graphviz(
        &self,
        hir: &HIR,
        filename: &str,
    ) {
        let mut file = std::fs::File::create(filename).unwrap();
        file.write_all(self.graphviz(hir).as_bytes()).unwrap();
    }

    pub fn graphviz(&self, hir: &HIR) -> String {
        let mut s = String::new();
        s.push_str("digraph {\n");
        for body in &self.bodies {
            let function = hir.scope.get_function(&body.function);
            s.push_str(&format!("  subgraph cluster_{} {{\n", body.function.index));
            s.push_str(&format!("    label = \"{}\";\n", function.name));
            for (index, bb) in body.basic_blocks.iter().enumerate() {
                let mut instructions = format!("{}:\\n", bb.label);
                instructions.push_str(bb.instructions.iter().map(|i| i.format(hir)).collect::<Vec<_>>().join("\\n").as_str());
                match &bb.terminator.kind {
                    TerminatorKind::Goto(label) => {
                        s.push_str(&format!("    {} -> {};\n", bb.label, label));
                    }
                    TerminatorKind::If(cond, then_label, else_label) => {
                        // create a branch block
                        instructions.push_str(format!("\\nif {} then goto {} else goto {}", cond.format(), then_label, else_label).as_str());
                        s.push_str(&format!("    {} -> {};\n", bb.label, then_label));
                        s.push_str(&format!("    {} -> {};\n", bb.label, else_label));
                    }
                    TerminatorKind::Return(value) => {
                        instructions.push_str(format!("\\nreturn {}", value.format()).as_str());
                    }
                    TerminatorKind::Next => {
                        s.push_str(&format!("    {} -> {};\n", bb.label, body.basic_blocks[index + 1].label));
                    }
                }
                s.push_str(&format!("    {} [label=\"{}\"];\n", bb.label, instructions));
            }
            s.push_str("  }\n");
        }
        s.push_str("}\n");
        s
    }

    pub fn interpret(&self, scope: &hir::Scope) {
        let mut interpreter = Interpreter::new(self, scope);
        interpreter.interpret();
    }
}

pub struct MIRGen<'a> {
    pub ir: MIR,
    pub scope: &'a hir::Scope,
    diagnostics: DiagnosticsBagCell,
    label_generator: Rc<RefCell<LabelGenerator>>,
}

impl<'a> MIRGen<'a> {
    pub fn new(
        diagnostics: DiagnosticsBagCell,
        scope: &'a hir::Scope,
    ) -> Self {
        Self {
            ir: MIR::new(),
            diagnostics,
            scope,
            label_generator: Rc::new(RefCell::new(LabelGenerator::new())),
        }
    }

    pub fn construct(mut self, hir: &HIR) -> MIR {
        self.construct_globals(hir);
        self.construct_functions(hir);
        self.ir
    }

    fn construct_globals(&mut self, hir: &HIR) {
        // todo
    }

    fn construct_functions(&mut self, hir: &HIR) {
        self.ir.bodies = hir.functions().iter().map(|(function, body)| {
            self.construct_function(function, *body)
        }).collect();
    }

    pub fn construct_function(&self, function: &Function, statements: Option<&Vec<HIRStatement>>) -> Body {
        let body = Body::new(function.id, self.label_generator.clone());

        let mut body = match statements {
            None => {
                body
            }
            Some(statements) => {
                let body_gen = BodyGen::new(body, self.scope);
                body_gen.gen(statements)
            }
        };
        let bb = body.basic_blocks.last_mut();
        if let Some(bb) = bb {
            if bb.terminator.kind == TerminatorKind::Next {
                bb.terminator = Terminator {
                    span: None,
                    kind: TerminatorKind::Return(Primary::Void),
                };
            }
        }
        body
    }
}

pub struct BodyGen<'a> {
    pub body: Body,
    pub basic_blocks: Vec<BasicBlock>,
    pub current_block: usize,
    pub scope: &'a hir::Scope,
}

impl<'a> BodyGen<'a> {
    pub fn new(mut body: Body, scope: &'a hir::Scope) -> Self {
        Self {
            basic_blocks: vec![
                BasicBlock::new(body.new_basic_block()),
            ],
            body,
            current_block: 0,
            scope,
        }
    }

    fn gen(mut self, statements: &Vec<HIRStatement>) -> Body {
        let function = self.scope.get_function(&self.body.function);
        for param_id in function.parameters.iter() {
            self.body.scope.new_variable(param_id);
        }
        self.gen_stmts(statements);
        self.body.basic_blocks = self.basic_blocks;
        self.body
    }

    fn gen_stmts(&mut self, statements: &Vec<HIRStatement>) {
        for stmt in statements {
            self.gen_stmt(stmt);
        }
    }

    fn gen_stmt(&mut self, stmt: &HIRStatement) {
        let (instruction_kind, assign_to) = match &stmt.kind {
            HIRStatementKind::VariableDeclaration(variable_declaration) => {
                let var = self.body.scope.new_variable(&variable_declaration.variable_id);
                let expr = &variable_declaration.initializer;
                let primary = self.gen_expr(&expr);
                (InstructionKind::Store(
                    MemoryPointer::Variable(var, variable_declaration.initializer.ty.clone()),
                    primary,
                ), None)
            }
            HIRStatementKind::Expression(expr) => {
                let primary = self.gen_expr(&expr.expression);
                (InstructionKind::Primary(primary), Some(self.body.scope.new_temp_variable()))
            }
            HIRStatementKind::If(if_stmt) => {
                let condition = self.gen_expr(&if_stmt.condition);
                let current_block = self.current_block;
                let then_block = self.push_basic_block();
                let else_block = if_stmt.else_.as_ref().map(|_| self.push_basic_block());
                let end_block = self.push_basic_block();
                self.set_current_block(current_block);
                let terminator = Terminator::if_(stmt.span.clone(), condition, then_block, else_block.clone().unwrap_or(end_block));
                self.push_terminator(terminator);
                self.set_current_block_by_label(&then_block);
                self.gen_stmts(&if_stmt.then);
                if let Some(else_block) = else_block {
                    self.set_current_block_by_label(&else_block);
                    self.gen_stmts(&if_stmt.else_.as_ref().unwrap());
                }
                self.set_current_block_by_label(&end_block);
                return;
            }
            HIRStatementKind::Block(stmt) => {
                let current_block = self.current_block;
                self.push_basic_block();
                self.gen_stmts(&stmt.statements);
                self.set_current_block(current_block);
                return;
            }
            HIRStatementKind::While(while_stmt) => {
                let condition_block = self.push_basic_block();
                let body_block = self.push_basic_block();
                let end_block = self.push_basic_block();
                self.set_current_block_by_label(&condition_block);
                let terminator = Terminator::if_(stmt.span.clone(), self.gen_expr(&while_stmt.condition), body_block, end_block);
                self.push_terminator(terminator);
                self.set_current_block_by_label(&body_block);
                self.gen_stmts(&while_stmt.body);
                let terminator = Terminator::goto(condition_block);
                self.push_terminator(terminator);
                self.set_current_block_by_label(&end_block);
                return;
            }

            HIRStatementKind::Return(return_stmt) => {
                let primary = self.gen_expr(&return_stmt.expression);
                let terminator = Terminator::return_(stmt.span.clone(), primary);
                self.push_terminator(terminator);
                return;
            }
        };
        let instruction = Instruction {
            kind: instruction_kind,
            span: stmt.span.clone(),
            assign_to,
        };
        self.push_instruction(instruction);
    }

    fn gen_expr(&mut self, expr: &HIRExpression) -> Primary {
        let (instructions, primary) = match &expr.kind {
            HIRExpressionKind::Literal(expr) => {
                let primary = match &expr.value {
                    HIRLiteralValue::Integer(int) => {
                        Primary::I64(*int)
                    }
                    HIRLiteralValue::Boolean(bool) => {
                        Primary::Bool(*bool)
                    }
                    HIRLiteralValue::String(string) => {
                        Primary::Str(string.to_string())
                    }
                    HIRLiteralValue::Char(char) => {
                        Primary::Char(*char)
                    }
                };
                (None, primary)
            }
            HIRExpressionKind::Binary(bin_expr) => {
                let left = self.gen_expr(&bin_expr.left);
                let right = self.gen_expr(&bin_expr.right);
                let temp_var = self.body.scope.new_temp_variable();
                let bin_op = Instruction {
                    kind: InstructionKind::BinaryOp(bin_expr.op.clone(), (left, bin_expr.left.ty.clone()), (right, bin_expr.right.ty.clone()), expr.ty.clone()),
                    span: expr.span.clone(),
                    assign_to: Some(temp_var),
                };
                (Some(vec![bin_op]), Primary::Variable(temp_var, expr.ty.clone()))
            }
            HIRExpressionKind::Unary(un_op) => {
                let primary = self.gen_expr(&un_op.operand);
                let temp_var = self.body.scope.new_temp_variable();
                let unary_op = Instruction {
                    kind: InstructionKind::UnaryOp(un_op.op.clone(), primary, expr.ty.clone()),
                    span: expr.span.clone(),
                    assign_to: Some(temp_var),
                };
                (Some(vec![unary_op]), Primary::Variable(temp_var, expr.ty.clone()))
            }
            HIRExpressionKind::Parenthesized(expr) => {
                (None, self.gen_expr(&expr.expression))
            }
            HIRExpressionKind::Assignment(a_expr) => {
                let primary = self.gen_expr(&a_expr.value);
                let ptr = match &a_expr.target.kind {
                    HIRAssignmentTargetKind::Variable(var_id) => {
                        let ty = self.scope.get_variable(var_id).ty.clone();
                        let var = self.body.scope.get_variable(var_id).unwrap();
                        MemoryPointer::Variable(var, ty)
                    }
                    HIRAssignmentTargetKind::Deref(expr) => {
                        let primary = self.gen_expr(expr);
                        MemoryPointer::Primary(primary, expr.ty.clone())
                    }
                    HIRAssignmentTargetKind::Error => {
                        unreachable!()
                    }
                };
                let assignment_expr_temp_var = self.body.scope.new_temp_variable();
                let instruction = Instruction {
                    kind: InstructionKind::Store(ptr, primary),
                    span: expr.span.clone(),
                    assign_to: Some(assignment_expr_temp_var),
                };
                (Some(vec![instruction]), Primary::Variable(assignment_expr_temp_var, expr.ty.clone()))
            }
            HIRExpressionKind::Call(call_expr) => {
                let function_id = match &call_expr.callee {
                    HIRCallee::Function(id) => {
                        *id
                    }
                    HIRCallee::Undeclared => {
                        unreachable!()
                    }
                    HIRCallee::Invalid => { unreachable!() }
                };
                let mut args = Vec::new();
                for arg in &call_expr.arguments {
                    args.push(self.gen_expr(arg));
                }

                let return_var = self.body.scope.new_temp_variable();
                let call_instr = Instruction {
                    kind: InstructionKind::Call(function_id, args),
                    span: expr.span.clone(),
                    assign_to: Some(return_var),
                };

                let return_var_prim = Primary::Variable(return_var, expr.ty.clone());

                (Some(vec![call_instr]), return_var_prim)
            }
            HIRExpressionKind::MemberAccess(member_expr) => {
                unimplemented!()
            }

            HIRExpressionKind::Variable(var_expr) => {
                let var = self.body.scope.get_variable(&var_expr.variable_id).unwrap();
                (None, Primary::Variable(var, expr.ty.clone()))
            }
            HIRExpressionKind::Void => {
                (None, Primary::Void)
            }
            HIRExpressionKind::Ref(ref_expr) => {
                let primary = self.gen_expr(&ref_expr.expression);
                let temp_var = self.body.scope.new_temp_variable();
                let get_address = Instruction {
                    kind: InstructionKind::GetAddress(primary),
                    span: expr.span.clone(),
                    assign_to: Some(temp_var),
                };
                (Some(vec![get_address]), Primary::Variable(temp_var, expr.ty.clone()))
            }
            HIRExpressionKind::Deref(deref_expr) => {
                let primary = self.gen_expr(&deref_expr.expression);
                let temp_var = self.body.scope.new_temp_variable();
                let deref = Instruction {
                    kind: InstructionKind::Load(MemoryPointer::Primary(primary, expr.ty.clone())),
                    span: expr.span.clone(),
                    assign_to: Some(temp_var),
                };
                (Some(vec![deref]), Primary::Variable(temp_var, expr.ty.clone()))
            }
            HIRExpressionKind::Cast(cast_expr) => {
                let primary = self.gen_expr(&cast_expr.expression);
                let temp_var = self.body.scope.new_temp_variable();
                let cast = Instruction {
                    kind: InstructionKind::Cast(primary, cast_expr.ty.clone()),
                    span: expr.span.clone(),
                    assign_to: Some(temp_var),
                };
                (Some(vec![cast]), Primary::Variable(temp_var, expr.ty.clone()))
            }
        };
        if let Some(instructions) = instructions {
            for instruction in instructions {
                self.push_instruction(instruction);
            }
        }
        primary
    }

    pub fn push_basic_block(&mut self) -> Label {
        let label = self.body.new_basic_block();
        self.basic_blocks.push(BasicBlock::new(label.clone()));
        label
    }

    pub fn terminate_block(&mut self, terminator: Terminator) {
        self.current_block().terminator = terminator;
        self.basic_blocks.push(BasicBlock::new(self.body.new_basic_block()));
    }

    pub fn current_block(&mut self) -> &mut BasicBlock {
        self.basic_blocks.get_mut(self.current_block).expect("Current block is out of bounds")
    }

    pub fn set_current_block(&mut self, index: usize) {
        self.current_block = index;
    }

    pub fn set_current_block_by_label(&mut self, label: &Label) {
        let index = self.basic_blocks.iter().position(|bb| &bb.label == label).expect("Block not found");
        self.set_current_block(index);
    }

    pub fn push_instruction(&mut self, instruction: Instruction) {
        self.current_block().instructions.push(instruction);
    }

    pub fn push_terminator(&mut self, terminator: Terminator) {
        self.current_block().terminator = terminator;
        self.basic_blocks.push(BasicBlock::new(self.body.new_basic_block()));
    }
}


