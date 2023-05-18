use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::io::Write;
use std::ops::Deref;
use std::rc::Rc;

use fusion_compiler::{id, id_generator};

use crate::diagnostics::DiagnosticsBagCell;
use crate::hir;
use crate::hir::{FunctionId, HIR, HIRAssignmentTargetKind, HIRBinaryOperator, HIRCallee, HIRExpression, HIRExpressionKind, HIRFunction, HIRGlobal, HIRLiteralValue, HIRStatement, HIRStatementKind, HIRUnaryOperator, VariableId};
use crate::modules::scopes::{GlobalScope, GlobalScopeCell};
use crate::modules::symbols::{Function, Variable};
use crate::text::span::TextSpan;
use crate::typings::{Layout, Type};

pub struct Body {
    pub basic_blocks: Vec<BasicBlock>,
    pub scope: MIRScope,
    pub function: FunctionId,
    pub parameters: Vec<LocalPlace>,
    label_gen: Rc<RefCell<LabelGenerator>>,
}

// todo: add support for external packages (std)

impl Body {
    pub fn new(
        function: FunctionId,
        label_gen: Rc<RefCell<LabelGenerator>>,
        scope: GlobalScopeCell,
    ) -> Self {
        Self {
            scope: MIRScope::new(scope),
            basic_blocks: vec![],
            function,
            label_gen,
            parameters: vec![],

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
        write!(f, ".L{}", self.index)
    }
}

id!(GlobalLabel);
id_generator!(GlobalLabelGenerator, GlobalLabel);

impl Display for GlobalLabel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, ".LC{}", self.index)
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

#[derive(Debug, Clone)]
pub enum GlobalValue {
    String(String),
}

impl GlobalValue {
    pub fn layout(&self) -> Layout {
        match self {
            GlobalValue::String(string) => Layout {
                size: string.len() as u32 + 1,
                alignment: 1,
            },
        }
    }
}

pub struct GlobalMIRScope {
    global_place_gen: Rc<RefCell<GlobalLabelGenerator>>,
    globals: HashMap<GlobalLabel, GlobalValue>,
}

impl GlobalMIRScope {
    pub fn new() -> Self {
        Self {
            global_place_gen: Rc::new(RefCell::new(GlobalLabelGenerator::new())),
            globals: HashMap::new(),
        }
    }

    pub fn add_global_string(&mut self, string: String) -> GlobalPlace {
        let label = self.global_place_gen.borrow_mut().next();
        let value = GlobalValue::String(string);
        let layout = value.layout();
        self.globals.insert(label, value);
        GlobalPlace::new(label, layout)
    }
}

pub struct MIRScope {
    pub place_count: u32,
    mapped_variables: HashMap<VariableId, LocalPlace>,
    pub(crate) locals: Vec<VariableId>,
    scope: GlobalScopeCell,
    pub locals_size: u32,
    nesting_level: u32,
}

impl MIRScope {
    pub fn new(
        scope: GlobalScopeCell,
    ) -> Self {
        Self {
            place_count: 0,
            mapped_variables: HashMap::new(),
            locals: vec![],
            scope,
            locals_size: 0,
            nesting_level: 0,
        }
    }

    pub fn new_local_place(&mut self, layout: Layout) -> LocalPlace {
        let current_count = self.place_count;
        self.place_count += 1;
        // todo: we should handle different scopes (lifetimes are different)
        self.locals_size += layout.size;
        LocalPlace::no_offset(Var::new(current_count), layout)
    }

    pub fn new_mapped_variable(&mut self, id: &VariableId) -> LocalPlace {
        let scope = self.get_scope();
        let variable = scope.get_variable(id);
        let layout = variable.ty.layout(&scope);
        drop(scope);
        let temp = self.new_local_place(layout);
        self.mapped_variables.insert(*id, temp);
        self.locals.push(*id);
        temp
    }

    fn get_scope(&self) -> Ref<GlobalScope> {
        self.scope.borrow()
    }

    pub fn get_variable(&self, id: &VariableId) -> Option<LocalPlace> {
        self.mapped_variables.get(id).copied()
    }

    fn enter_scope(&mut self) {
        self.nesting_level += 1;
    }

    fn exit_scope(&mut self) {
        self.nesting_level -= 1;
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
            InstructionKind::Store { place, value: init } => {
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
            InstructionKind::BinaryOp { operator: op, lhs, rhs, result_place: result_ptr } => {
                format!("{} = {} {} {}", result_ptr, self.format_bin_op(op), lhs, rhs)
            }
            InstructionKind::UnaryOp { operator: op, operand: primary, result_place: result_ptr } => {
                format!("{} = {} {}", result_ptr, self.format_un_op(op), primary)
            }
            InstructionKind::Move {
                from,
                to,
            }
            =>
                {
                    format!("{} = move {}", to, from)
                }
            InstructionKind::Deref { from, to } => {
                format!("{} = deref {}", to, from)
            }
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Var {
    pub id: u32,
}

impl Var {
    pub fn new(id: u32) -> Self {
        Self {
            id,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Place {
    Local(LocalPlace),
    Global(GlobalPlace),
}

impl Place {
    pub fn layout(&self) -> &Layout {
        match self {
            Place::Local(local) => {
                &local.layout
            }
            Place::Global(global) => {
                &global.layout
            }
        }
    }

    pub fn into_local(self) -> LocalPlace {
        match self {
            Place::Local(local) => {
                local
            }
            Place::Global(global) => {
                panic!("Expected local place, got global place: {}", global)
            }
        }
    }
}

impl Display for Place {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Place::Local(local) => {
                write!(f, "{}", local)
            }
            Place::Global(global) => {
                write!(f, "{}", global)
            }
        }
    }
}


#[derive(Debug, Copy, Clone)]
pub struct GlobalPlace {
    pub label: GlobalLabel,
    pub layout: Layout,
}

impl GlobalPlace {
    pub fn new(label: GlobalLabel, layout: Layout) -> Self {
        Self {
            label,
            layout,
        }
    }

    pub fn into_place(self) -> Place {
        Place::Global(self)
    }
}

impl Display for GlobalPlace {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Place({})", self.label)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct LocalPlace {
    pub var: Var,
    pub offset: u32,
    pub layout: Layout,
}

impl LocalPlace {
    pub fn new(var: Var, offset: u32, layout: Layout) -> Self {
        Self {
            var,
            offset,
            layout,
        }
    }

    pub fn no_offset(var: Var, layout: Layout) -> Self {
        Self {
            var,
            offset: 0,
            layout,
        }
    }

    pub fn into_place(self) -> Place {
        Place::Local(self)
    }
}

impl Display for LocalPlace {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Place(")?;
        match self.offset {
            0 => {
                write!(f, "%{}", self.var.id)
            }
            _ => {
                write!(f, "[%{} + {}]", self.var.id, self.offset)
            }
        }?;
        write!(f, ")")
    }
}

#[derive(Debug)]
pub enum Value {
    I64(i64),
    Char(char),
    Bool(bool),
    Struct(Vec<Value>),
    Void,
    StoredAt(Place),
    Ptr(Place),
}

impl Value {
    pub fn cast_to(self, ty: &Type) -> Value {
        unimplemented!()
    }

    pub fn into_place(self) -> Place {
        match self {
            Value::StoredAt(place) => {
                place
            }
            _ => {
                panic!("Expected place, got {}", self)
            }
        }
    }

    pub fn layout(&self) -> Layout {
        match self {
            Value::I64(_) => {
                Layout {
                    size: 8,
                    alignment: 8,
                }
            }
            Value::Char(_) => {
                Layout {
                    size: 1,
                    alignment: 1,
                }
            }
            Value::Bool(_) => {
                Layout {
                    size: 1,
                    alignment: 1,
                }
            }
            Value::Struct(values) => {
                let mut gt_alignment = 1;
                let mut size = 0;
                for value in values {
                    let layout = value.layout();
                    size += layout.size;
                    gt_alignment = gt_alignment.max(layout.alignment);
                }
                Layout {
                    size,
                    alignment: gt_alignment,
                }
            }
            Value::Void => {
                Layout {
                    size: 0,
                    alignment: 0,
                }
            }
            Value::StoredAt(place) => {
                *place.layout()
            }
            Value::Ptr(_) => {
                Layout {
                    size: Layout::POINTER_SIZE,
                    alignment: Layout::POINTER_SIZE,
                }
            }
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Value(")?;
        match self {
            Value::I64(value) => {
                write!(f, "{}", value)
            }
            Value::Char(value) => {
                write!(f, "'{}'", value)
            }
            Value::Bool(value) => {
                write!(f, "{}", value)
            }
            Value::Struct(values) => {
                write!(f, "{{")?;
                for (i, value) in values.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", value)?;
                }
                write!(f, "}}")
            }
            Value::Void => {
                write!(f, "()")
            }
            Value::Ptr(place) => {
                write!(f, "Ptr {}", place)
            }
            Value::StoredAt(place) => {
                write!(f, "{}", place)
            }
        }?;
        write!(f, ")")
    }
}

#[derive(Debug)]
pub enum InstructionKind {
    Store { place: LocalPlace, value: Value },
    Call { return_value_place: LocalPlace, function_id: FunctionId, args: Vec<LocalPlace> },
    BinaryOp { operator: HIRBinaryOperator, lhs: Value, rhs: Value, result_place: LocalPlace },
    UnaryOp { operator: HIRUnaryOperator, operand: Value, result_place: LocalPlace },
    Move { to: LocalPlace, from: Place },
    Deref { to: LocalPlace, from: Place },
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

    pub fn return_(span: TextSpan, return_value_place: LocalPlace) -> Self {
        Self::new(TerminatorKind::Return(return_value_place), Some(span))
    }

    pub fn goto(label: Label) -> Self {
        Self::new(TerminatorKind::Goto(label), None)
    }

    pub fn if_(span: TextSpan, condition: Place, then_label: Label, else_label: Label) -> Self {
        Self::new(TerminatorKind::If {
            condition,
            then: then_label,
            else_: else_label,
        }, Some(span))
    }
}

#[derive(Debug)]
pub enum TerminatorKind {
    Goto(Label),
    If { condition: Place, then: Label, else_: Label },
    Return(LocalPlace),
    Next,
}

pub struct MIR {
    pub bodies: Vec<Body>,
    pub globals: Vec<(GlobalLabel, GlobalValue)>,
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
        scope: &GlobalScope,
        filename: &str,
    ) {
        let mut file = std::fs::File::create(filename).unwrap();
        file.write_all(self.graphviz(scope).as_bytes()).unwrap();
    }

    pub fn graphviz(&self, scope: &GlobalScope) -> String {
        let mut s = String::new();
        s.push_str("digraph {\n");
        for body in &self.bodies {
            let function = scope.get_function(&body.function);
            s.push_str(&format!("  subgraph cluster_{} {{\n", body.function.index));
            s.push_str(&format!("    label = \"{}\";\n", function.name));
            for (index, bb) in body.basic_blocks.iter().enumerate() {
                let mut instructions = format!("{}:\\n", bb.label);
                instructions.push_str(bb.instructions.iter().map(|i| i.format(scope)).collect::<Vec<_>>().join("\\n").as_str());
                match &bb.terminator.kind {
                    TerminatorKind::Goto(label) => {
                        s.push_str(&format!("    {} -> {};\n", bb.label, label));
                    }
                    TerminatorKind::If { condition: cond, then: then_label, else_: else_label } => {
                        // create a branch block
                        instructions.push_str(format!("\\nif {} then goto {} else goto {}", cond, then_label, else_label).as_str());
                        s.push_str(&format!("    {} -> {};\n", bb.label, then_label));
                        s.push_str(&format!("    {} -> {};\n", bb.label, else_label));
                    }
                    TerminatorKind::Return(place) => {
                        instructions.push_str(format!("\\nreturn {}", place).as_str());
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

    pub fn save_output(&self, scope: &GlobalScope, filename: &str) {
        let mut file = std::fs::File::create(filename).unwrap();
        file.write_all(self.output(scope).as_bytes()).unwrap();
    }

    pub fn output(&self, scope: &GlobalScope) -> String {
        let mut s = String::new();
        for body in &self.bodies {
            let function = scope.get_function(&body.function);
            s.push_str(&format!("{}:\n", function.name));
            for id in body.scope.locals.iter() {
                let local = scope.get_variable(id);
                let place = body.scope.get_variable(id).unwrap();
                s.push_str(&format!("  {} = let {}: {}\n", place, local.name, local.ty));
            }
            for bb in &body.basic_blocks {
                s.push_str(&format!("  {}:\n", bb.label));
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
                    TerminatorKind::Return(place) => {
                        s.push_str(&format!("    return {}\n", place));
                    }
                    TerminatorKind::Next => {}
                }
            }
        }
        s
    }

    pub fn interpret(&self, scope: &GlobalScope) {
        // let mut interpreter = Interpreter::new(self, scope);
        // interpreter.interpret();
    }
}

pub struct MIRGen {
    pub ir: MIR,
    pub scope: GlobalScopeCell,
    diagnostics: DiagnosticsBagCell,
    label_generator: Rc<RefCell<LabelGenerator>>,
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
            label_generator: Rc::new(RefCell::new(LabelGenerator::new())),
            global_scope: Rc::new(RefCell::new(GlobalMIRScope::new())),
        }
    }

    pub fn construct(mut self, hir: &HIR) -> MIR {
        self.construct_globals(hir);
        self.construct_functions(hir);
        let globals = &self.global_scope.borrow().globals;
        self.ir.globals = globals.iter().map(|(id, global)| {
            (id.clone(), global.clone())
        }).collect();
        self.ir
    }

    fn construct_globals(&mut self, hir: &HIR) {
        // todo
    }

    fn construct_functions(&mut self, hir: &HIR) {
        let scope = &self.scope.borrow();
        self.ir.bodies = hir.functions(
            scope,
        ).iter().map(|(function, body)| {
            if function.is_extern() {
                return None;
            }
            Some(self.construct_function(function, *body))
        }).flatten().collect();
    }

    pub fn construct_function(&self, function: &Function, statements: Option<&Vec<HIRStatement>>) -> Body {
        let body = Body::new(function.id, self.label_generator.clone(), self.scope.clone());

        let mut body = match statements {
            None => {
                body
            }
            Some(statements) => {
                let body_gen = BodyGen::new(body, self.scope.clone(), self.global_scope.clone());
                body_gen.gen(statements)
            }
        };
        let bb = body.basic_blocks.last_mut();
        if let Some(bb) = bb {
            if let TerminatorKind::Next = bb.terminator.kind {
                let place = body.scope.new_local_place(Value::Void.layout());
                bb.terminator = Terminator {
                    span: None,
                    kind: TerminatorKind::Return(place),
                };
            }
        }
        body
    }
}

pub struct BodyGen {
    pub body: Body,
    pub basic_blocks: Vec<BasicBlock>,
    pub current_block: usize,
    pub scope: GlobalScopeCell,
    pub global_scope: Rc<RefCell<GlobalMIRScope>>,
}

impl BodyGen {
    pub fn new(mut body: Body, scope: GlobalScopeCell,
               global_scope: Rc<RefCell<GlobalMIRScope>>,
    ) -> Self {
        Self {
            basic_blocks: vec![
                BasicBlock::new(body.new_basic_block()),
            ],
            body,
            current_block: 0,
            scope,
            global_scope,
        }
    }

    fn gen(mut self, statements: &Vec<HIRStatement>) -> Body {
        let scope = self.scope.borrow();
        let function = scope.get_function(&self.body.function);
        for param_id in function.parameters.iter() {
            self.body.parameters.push(self.body.scope.new_mapped_variable(param_id));
        }
        drop(scope);
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
        match &stmt.kind {
            HIRStatementKind::VariableDeclaration(variable_declaration) => {
                let expr = &variable_declaration.initializer;
                let var_place = self.body.scope.new_mapped_variable(&variable_declaration.variable_id);
                self.gen_expr(&expr, Some(&var_place));
            }
            HIRStatementKind::Expression(expr) => {
                let value_place = self.body.scope.new_local_place(expr.expression.ty.layout(
                    &self.scope.borrow(),
                ));
                self.gen_expr(&expr.expression, Some(&value_place));
            }
            HIRStatementKind::If(if_stmt) => {
                let condition = self.body.scope.new_local_place(Type::Bool.layout(
                    &self.scope.borrow(),
                ));
                self.gen_expr(&if_stmt.condition, Some(&condition));
                self.body.scope.enter_scope();
                let current_block = self.current_block;
                let then_block = self.push_basic_block();
                let else_block = if_stmt.else_.as_ref().map(|_| self.push_basic_block());
                let end_block = self.push_basic_block();
                self.set_current_block(current_block);
                let terminator = Terminator::if_(stmt.span.clone(), condition.into_place(), then_block, else_block.clone().unwrap_or(end_block));
                self.push_terminator_no_block(terminator);
                self.set_current_block_by_label(&then_block);
                self.gen_stmts(&if_stmt.then);
                if let Some(else_block) = else_block {
                    self.set_current_block_by_label(&else_block);
                    self.gen_stmts(&if_stmt.else_.as_ref().unwrap());
                }
                self.body.scope.exit_scope();
                self.set_current_block_by_label(&end_block);
            }
            HIRStatementKind::Block(stmt) => {
                let current_block = self.current_block;
                self.push_basic_block();
                self.body.scope.enter_scope();
                self.gen_stmts(&stmt.statements);
                self.body.scope.exit_scope();
                self.set_current_block(current_block);
            }
            HIRStatementKind::While(while_stmt) => {
                let condition_block = self.push_basic_block();
                let body_block = self.push_basic_block();
                let end_block = self.push_basic_block();
                self.set_current_block_by_label(&condition_block);
                let condition = self.body.scope.new_local_place(Type::Bool.layout(
                    &self.scope.borrow(),
                ));
                self.gen_expr(&while_stmt.condition, Some(&condition));
                let terminator = Terminator::if_(stmt.span.clone(), condition.into_place(), body_block, end_block);
                self.push_terminator(terminator);
                self.set_current_block_by_label(&body_block);
                self.body.scope.enter_scope();
                self.gen_stmts(&while_stmt.body);
                self.body.scope.exit_scope();
                let terminator = Terminator::goto(condition_block);
                self.push_terminator(terminator);
                self.set_current_block_by_label(&end_block);
            }

            HIRStatementKind::Return(return_stmt) => {
                let place = self.body.scope.new_local_place(return_stmt.expression.ty.layout(
                    &self.scope.borrow(),
                ));
                self.gen_expr(&return_stmt.expression, Some(&place));
                // todo: develop a calling convention
                let terminator = Terminator::return_(stmt.span.clone(), place);
                if self.body.scope.nesting_level == 0 {
                    self.push_terminator_no_block(terminator);
                } else {
                    self.push_terminator(terminator);
                }
            }
        }
    }

    fn gen_expr(&mut self, expr: &HIRExpression, store_at: Option<&LocalPlace>) -> Value {
        match &expr.kind {
            HIRExpressionKind::Literal(literal_expr) => {
                let value = match &literal_expr.value {
                    HIRLiteralValue::Integer(int) => {
                        Value::I64(*int)
                    }
                    HIRLiteralValue::Boolean(bool) => {
                        Value::Bool(*bool)
                    }
                    HIRLiteralValue::String(string) => {
                        let global_place = self.global_scope.borrow_mut().add_global_string(string.clone());
                        Value::Ptr(global_place.into_place())
                    }
                    HIRLiteralValue::Char(char) => {
                        Value::Char(*char)
                    }
                };
                if let Some(place) = store_at {
                    let instruction = Instruction {
                        kind: InstructionKind::Store {
                            value,
                            place: *place,
                        },
                        span: expr.span.clone(),
                    };
                    self.push_instruction(instruction);
                    return Value::StoredAt(place.into_place());
                }
                value
            }
            HIRExpressionKind::Binary(bin_expr) => {
                let left = self.gen_expr(&bin_expr.left, None);
                let right = self.gen_expr(&bin_expr.right, None);
                let result_place = (store_at.copied()).unwrap_or(self.body.scope.new_local_place(
                    expr.ty.layout(self.scope.borrow().deref()),
                ));
                let bin_op = Instruction {
                    kind: InstructionKind::BinaryOp {
                        operator: bin_expr.op.clone(),
                        lhs: left,
                        rhs: right,
                        result_place,
                    },
                    span: expr.span.clone(),
                };
                self.push_instruction(bin_op);
                Value::StoredAt(result_place.into_place())
            }
            HIRExpressionKind::Unary(un_op) => {
                let value = self.gen_expr(&un_op.operand, None);
                let result_place = (store_at.copied()).unwrap_or(self.body.scope.new_local_place(
                    expr.ty.layout(self.scope.borrow().deref()),
                ));
                let unary_op = Instruction {
                    kind: InstructionKind::UnaryOp {
                        operator: un_op.op.clone(),
                        operand: value,
                        result_place,
                    },
                    span: expr.span.clone(),
                };
                self.push_instruction(unary_op);
                Value::StoredAt(result_place.into_place())
            }
            HIRExpressionKind::Parenthesized(expr) => {
                self.gen_expr(&expr.expression, store_at)
            }
            HIRExpressionKind::Assignment(a_expr) => {
                let place = match &a_expr.target.kind {
                    HIRAssignmentTargetKind::Variable(var_id) => {
                        self.body.scope.get_variable(var_id).unwrap()
                    }
                    HIRAssignmentTargetKind::Deref(expr) => {
                        unimplemented!()
                    }
                    HIRAssignmentTargetKind::Error => {
                        unreachable!()
                    }
                    HIRAssignmentTargetKind::Field(id, target) => {
                        let value = self.gen_expr(target, None).into_place().into_local();
                        let offset = self.scope.borrow().get_field_offset(id);
                        let field_layout = self.scope.borrow().get_field(id).ty.layout(&self.scope.borrow());
                        LocalPlace::new(value.var, offset, field_layout)
                    }
                };
                self.gen_expr(&a_expr.value, Some(&place));
                let place = place.into_place();
                if let Some(store_at) = store_at {
                    let instruction = Instruction {
                        kind: InstructionKind::Move {
                            from: place,
                            to: store_at.clone(),
                        },
                        span: expr.span.clone(),
                    };
                    self.push_instruction(instruction);
                }
                Value::StoredAt(place)
            }
            HIRExpressionKind::Call(call_expr) => {
                let function_id = match &call_expr.callee {
                    HIRCallee::Function(id) => {
                        *id
                    }
                    HIRCallee::Undeclared(_) => {
                        unreachable!()
                    }
                    HIRCallee::Invalid(_) => { unreachable!() }
                };
                let mut args = Vec::new();
                for arg in &call_expr.arguments {
                    let arg_place = self.body.scope.new_local_place(arg.ty.layout(&self.scope.borrow()));
                    self.gen_expr(arg, Some(&arg_place));
                    args.push(
                        arg_place
                    );
                }

                let return_value_place = (store_at.copied()).unwrap_or(self.body.scope.new_local_place(
                    expr.ty.layout(self.scope.borrow().deref()),
                ));
                let call_instr = Instruction {
                    kind: InstructionKind::Call {
                        return_value_place,
                        function_id,
                        args,
                    },
                    span: expr.span.clone(),
                };

                self.push_instruction(call_instr);
                Value::StoredAt(return_value_place.into_place())
            }
            HIRExpressionKind::FieldAccess(member_expr) => {
                let primary = self.gen_expr(&member_expr.target, None);
                let object_place = primary.into_place().into_local();
                let offset = self.scope.borrow().get_field_offset(&member_expr.field_id);
                let field_layout = self.scope.borrow().get_field(&member_expr.field_id).ty.layout(&self.scope.borrow());
                let field_place = LocalPlace::new(object_place.var, offset, field_layout).into_place();
                if let Some(store_at) = store_at {
                    let instruction = Instruction {
                        kind: InstructionKind::Move {
                            from: field_place,
                            to: store_at.clone(),
                        },
                        span: expr.span.clone(),
                    };
                    self.push_instruction(instruction);
                    return Value::StoredAt(store_at.into_place());
                }
                Value::StoredAt(field_place)
            }

            HIRExpressionKind::Variable(var_expr) => {
                let place = self.body.scope.get_variable(&var_expr.variable_id).unwrap().into_place();
                // todo: reduce duplication
                if let Some(store_at) = store_at {
                    let instruction = Instruction {
                        kind: InstructionKind::Move {
                            from: place,
                            to: store_at.clone(),
                        },
                        span: expr.span.clone(),
                    };
                    self.push_instruction(instruction);
                    return Value::StoredAt(store_at.into_place());
                }
                Value::StoredAt(place)
            }
            HIRExpressionKind::Void => {
                Value::Void
            }
            HIRExpressionKind::Ref(ref_expr) => {
                let place_of_value = self.gen_expr_as_place(&ref_expr.expression);
                let place = (store_at.copied()).unwrap_or(self.body.scope.new_local_place(
                    expr.ty.layout(self.scope.borrow().deref()),
                ));
                let instruction = Instruction {
                    kind: InstructionKind::Store {
                        place,
                        value: Value::Ptr(place_of_value),
                    },
                    span: expr.span.clone(),
                };
                self.push_instruction(instruction);
                Value::StoredAt(place.into_place())
            }
            HIRExpressionKind::Deref(deref_expr) => {
                let primary = self.gen_expr(&deref_expr.expression, None);
                // todo: we should ont move the place here but the value that is stored in the place
                let from = primary.into_place();
                let to = (store_at.copied()).unwrap_or(self.body.scope.new_local_place(
                    expr.ty.layout(self.scope.borrow().deref()),
                ));
                let load = Instruction {
                    kind: InstructionKind::Deref { to, from },
                    span: expr.span.clone(),
                };
                self.push_instruction(load);
                Value::StoredAt(to.into_place())
            }
            HIRExpressionKind::Cast(cast_expr) => {
                self.gen_expr(&cast_expr.expression, store_at).cast_to(&cast_expr.ty)
            }
            HIRExpressionKind::StructInit(struct_init_expr) => {
                let value = Value::Struct(
                    struct_init_expr.fields.iter().map(|field| self.gen_expr(&field.value, None)).collect()
                );
                match store_at {
                    Some(store_at) => {
                        let instruction = Instruction {
                            kind: InstructionKind::Store {
                                place: *store_at,
                                value,
                            },
                            span: expr.span.clone(),
                        };
                        self.push_instruction(instruction);
                        Value::StoredAt(store_at.into_place())
                    }
                    None => {
                        value
                    }
                }
            }
        }
    }

    pub fn gen_expr_as_place(&mut self, expr: &HIRExpression) -> Place {
        let value = self.gen_expr(expr, None);
        match value {
            Value::StoredAt(place) => place,
            _ => {
                let place = self.body.scope.new_local_place(
                    expr.ty.layout(self.scope.borrow().deref())
                );
                let instruction = Instruction {
                    kind: InstructionKind::Store {
                        place,
                        value,
                    },
                    span: expr.span.clone(),
                };
                self.push_instruction(instruction);
                place.into_place()
            }
        }
    }

    pub fn push_basic_block(&mut self) -> Label {
        let label = self.body.new_basic_block();
        self.basic_blocks.push(BasicBlock::new(label.clone()));
        label
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
        self.push_terminator_no_block(terminator);
        // todo: uncomment this when there are issues
        // self.push_basic_block();
    }

    #[inline]
    pub fn push_terminator_no_block(&mut self, terminator: Terminator) {
        self.current_block().terminator = terminator;
    }
}


