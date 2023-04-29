use std::collections::HashMap;
use std::fmt::Display;
use std::fs::File;
use std::io::Write;

use basic_block::{BasicBlock, Label};
use instruction::{Instruction, InstructionKind, IRUnaryOperator, Primary, Terminator};
use label_gen::LabelGen;
use tem_var_gen::TempVarGen;
use terminator::TerminatorKind;

use crate::ast::{Ast, ASTBinaryOperatorKind, ASTExpression, ASTExpressionKind, ASTExprId, ASTFuncDeclStatement, ASTIdentifierExpression, ASTIdentifierKind, ASTLetStatement, ASTNode, ASTNodeId, ASTNumberExpression, ASTStatement, ASTStatementKind, ASTStmtId, ASTUnaryExpression, ASTUnaryOperatorKind};
use crate::ast::lexer::{Token, TokenKind};
use crate::compilation;
use crate::compilation::symbols::function::FunctionSymbol;
use crate::compilation::symbols::variable::{VariableId, VariableSymbol};
use crate::compilation::global_scope::GlobalScope;
use crate::ir::instruction::Member;
use crate::text::SourceText;
use crate::text::span::TextSpan;
use crate::typings::{FunctionType, Type};

pub mod tem_var_gen;
pub mod label_gen;
pub mod basic_block;
pub mod instruction;
pub mod terminator;


pub struct VariableMetadata {
    pub assignments: u32,
    pub usages: u32,
}

impl VariableMetadata {
    pub fn new() -> Self {
        Self {
            assignments: 0,
            usages: 0,
        }
    }

    pub fn new_assignment(&mut self) {
        self.assignments += 1;
    }

    pub fn new_usage(&mut self) {
        self.usages += 1;
    }

    pub fn has_been_reassigned(&self) -> bool {
        self.assignments > 1
    }

    pub fn has_been_used(&self) -> bool {
        self.usages > 0
    }
}

pub struct BasicBlockEdge {
    pub to: Label,
    pub condition: Option<bool>,
}

impl BasicBlockEdge {
    pub fn new(to: Label, condition: Option<bool>) -> Self {
        Self {
            to,
            condition,
        }
    }
}

pub struct IR {
    pub functions: Vec<Label>,
    pub basic_blocks: Vec<BasicBlock>,
}

impl IR {
    pub fn graphviz_repr(&self) -> String {
        let mut graph = String::new();
        graph.push_str("digraph G {\n");
        for block in self.basic_blocks.iter() {
            let mut label = String::new();
            label.push_str(&format!("{}:\n", block.label.name));
            for instruction in block.instructions.iter() {
                label.push_str(&format!("    {}\n", instruction));
            }
            label.push_str(&format!("    {}\n", block.terminator));
            graph.push_str(&format!("    {} [label=\"{}\"];\n", block.label.name, label));
            match &block.terminator.kind {
                TerminatorKind::Goto(label) => {
                    graph.push_str(&format!("    {} -> {};\n", block.label.name, label.name));
                }
                TerminatorKind::If(condition, then_label, else_label) => {
                    graph.push_str(&format!("    {} -> {} [label=\"{}\"];\n", block.label.name, then_label.name, condition));
                    graph.push_str(&format!("    {} -> {} [label=\"!{}\"];\n", block.label.name, else_label.name, condition));
                }
                TerminatorKind::Return(_) => {}
                TerminatorKind::Unresolved => {}
            }
        }
        graph.push_str("}\n");
        graph
    }

    pub fn save_graphviz(&self, path: &str) -> Result<(), std::io::Error> {
        let graph = self.graphviz_repr();
        let mut file = File::create(path)?;
        file.write_all(graph.as_bytes())
    }

    pub fn get_block(&self, label: &Label) -> Option<&BasicBlock> {
        self.basic_blocks.iter().find(|block| &block.label == label)
    }

    pub fn get_block_mut(&mut self, label: &Label) -> Option<&mut BasicBlock> {
        self.basic_blocks.iter_mut().find(|block| &block.label == label)
    }

    pub fn save(&self, path: &str) -> Result<(), std::io::Error> {
        let mut file = File::create(path)?;
        file.write_all(self.to_string().as_bytes())
    }

    pub fn get_entry_point(&self) -> &BasicBlock {
        let main_function = self.functions.iter().find(|label| label.name == "main").unwrap();
        self.get_block(&main_function).expect("main function not found")
    }

    pub fn get_variable_metadata(&self) -> HashMap<VariableId, VariableMetadata> {
        let mut variable_usages = HashMap::new();
        for block in self.basic_blocks.iter() {
            if let Some(function) = &block.function {
                for param in function.parameters.iter() {
                    let mut metadata = VariableMetadata::new();
                    metadata.new_assignment();
                    variable_usages.insert(param.id.clone(), metadata);
                }
            }
            for instruction in block.instructions.iter() {
                if let Some(assign_to) = &instruction.assign_to {
                    let mut metadata = VariableMetadata::new();
                    metadata.new_assignment();
                    variable_usages.insert(assign_to.id.clone(), metadata);
                }
                match &instruction.kind {
                    InstructionKind::Alloc(alloc) => {
                        let metadata = VariableMetadata::new();
                        variable_usages.insert(alloc.id.clone(), metadata);
                    }
                    InstructionKind::Store(store, ..) => {
                        variable_usages.get_mut(&store.id).unwrap().new_assignment();
                    }
                    InstructionKind::Primary(prim) => {
                        match prim {
                            crate::ir::instruction::Primary::Variable(var) => {
                                variable_usages.get_mut(&var.id).unwrap().new_usage();
                            }
                            _ => {}
                        }
                    }
                    InstructionKind::Binary(_, lhs, rhs) => {
                        if let crate::ir::instruction::Primary::Variable(var) = lhs {
                            variable_usages.get_mut(&var.id).expect(
                                format!("Variable {:?} not found", var.id).as_str()
                            ).new_usage();
                        }
                        if let crate::ir::instruction::Primary::Variable(var) = rhs {
                            variable_usages.get_mut(&var.id).unwrap().new_usage();
                        }
                    }

                    InstructionKind::Unary(_, primary) => {
                        if let crate::ir::instruction::Primary::Variable(var) = primary {
                            variable_usages.get_mut(&var.id).unwrap().new_usage();
                        }
                    }
                }
            }
            match &block.terminator.kind {
                TerminatorKind::Goto(_) => {}
                TerminatorKind::If(cond, _, _) => {
                    if let crate::ir::instruction::Primary::Variable(ref var) = cond {
                        variable_usages.get_mut(&var.id).unwrap().new_usage();
                    }
                }
                TerminatorKind::Return(Some(ref primary)) => {
                    if let crate::ir::instruction::Primary::Variable(ref var) = primary {
                        variable_usages.get_mut(&var.id).unwrap().new_usage();
                    }
                }
                TerminatorKind::Return(None) => {}
                TerminatorKind::Unresolved => {}
            }
        }
        variable_usages
    }

    pub fn get_edges(&self) -> HashMap<Label, Vec<BasicBlockEdge>> {
        let mut edges = HashMap::new();
        for block in self.basic_blocks.iter() {
            let mut successors = Vec::new();
            match &block.terminator.kind {
                TerminatorKind::Goto(label) => successors.push(
                    BasicBlockEdge::new(label.clone(), None)
                ),
                TerminatorKind::If(cond, then_label, else_label) => {
                    let value = match &cond {
                        Primary::Boolean(value) => Some(*value),
                        _ => None
                    };
                    successors.push(
                        BasicBlockEdge::new(then_label.clone(), value)
                    );
                    successors.push(
                        BasicBlockEdge::new(else_label.clone(), value.map(|v| !v))
                    );
                }
                TerminatorKind::Return(_) => {}
                TerminatorKind::Unresolved => panic!("unresolved terminator"),
            }
            edges.insert(block.label.clone(), successors);
        }
        edges
    }


}

impl Display for IR {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for block in self.basic_blocks.iter() {
            write!(f, "{}", block.label.name)?;
            if let Some(function) = &block.function {
                // write parameters
                write!(f, "(")?;
                for (i, param) in function.parameters.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", param.name)?;
                }
                write!(f, ")")?;
            }
            writeln!(f, ":")?;
            for instruction in block.instructions.iter() {
                writeln!(f, "    {}", instruction)?;
            }
            writeln!(f, "    {}", block.terminator)?;
        }
        Ok(())
    }
}

pub struct IRGen<'a> {
    label_gen: LabelGen,
    functions: Vec<Label>,
    basic_blocks: Vec<BasicBlock>,
    temp_var_gen: TempVarGen,
    source_text: &'a SourceText,
}

impl<'a> IRGen<'a> {
    pub fn new(source_text: &'a SourceText) -> Self {
        Self {
            label_gen: LabelGen::new(),
            functions: Vec::new(),
            basic_blocks: Vec::new(),
            temp_var_gen: TempVarGen::new(),
            source_text,
        }

    }

    pub fn gen_ir(mut self, ast: &mut Ast,global_scope: &mut GlobalScope,) -> IR {
        self.merge_top_level_statements_into_main(ast,global_scope);
        for (function_name, function) in global_scope.functions.iter() {
            self.gen_ir_for_function(ast, function, function_name,global_scope);
        }
        IR {
            functions: self.functions,
            basic_blocks: self.basic_blocks,
        }
    }



    fn gen_ir_for_function(&mut self, ast: &Ast, function: &FunctionSymbol, function_name: &String,global_scope: &GlobalScope,) {
        if let Some(body) = function.body {
            self.begin_function(function_name.as_str(), function);
            self.gen_stmt(ast, body,global_scope);
            self.maybe_add_return_to_current_block();
        }

    }


    fn maybe_add_return_to_current_block(&mut self) {
        let last_block = self.basic_blocks.last_mut().unwrap();
        if let TerminatorKind::Unresolved = last_block.terminator.kind {
            last_block.terminator = Terminator::new(TerminatorKind::Return(None), ASTNodeId::Unknown);
        }
    }


    fn gen_stmt(&mut self, ast: &Ast, stmt_id: ASTStmtId,global_scope: & GlobalScope,) {
        let stmt = ast.query_stmt(&stmt_id);
        match &stmt.kind {
            ASTStatementKind::Expression(expr_id) => {
                let primary = self.gen_expr(ast, *expr_id,global_scope);
                self.push_instruction_no_assign(InstructionKind::Primary(primary), &stmt_id.into());
            }
            ASTStatementKind::Let(stmt) => {
                let primary = self.gen_expr(ast, stmt.initializer,global_scope);
                let variable = stmt.variable.as_ref().map(
                    |var| var.clone()
                ).unwrap();
                self.push_instruction_no_assign(InstructionKind::Alloc(variable.clone()), &stmt_id.into());
                self.push_instruction_no_assign(InstructionKind::Store(variable, primary), &stmt_id.into());
            }
            ASTStatementKind::If(if_stmt) => {
                let condition = self.gen_expr(ast, if_stmt.condition,global_scope);
                let then_label = self.label_gen.next_label();
                let else_label = if_stmt.else_branch.as_ref().map(|_| self.label_gen.next_label());
                let end_label = self.label_gen.next_label();
                let if_instruction = TerminatorKind::If(condition, then_label.clone(), else_label.as_ref().unwrap_or(&end_label).clone());
                self.push_terminator_or_crash(if_instruction, &if_stmt.condition.into());
                self.begin_block(then_label);
                self.gen_stmt(ast, if_stmt.then_branch,global_scope);
                self.push_terminator_or_crash(TerminatorKind::Goto(end_label.clone()), &if_stmt.then_branch.into());
                if let Some(else_branch) = &if_stmt.else_branch {
                    self.begin_block(else_label.unwrap());
                    self.gen_stmt(ast, else_branch.else_statement,global_scope);
                    self.push_terminator_or_crash(TerminatorKind::Goto(end_label.clone()), &else_branch.else_statement.into());
                }
                self.begin_block(end_label);
            }
            ASTStatementKind::Block(stmt) => {
                for statement in &stmt.statements {
                    self.gen_stmt(ast, *statement,global_scope);
                }
            }
            ASTStatementKind::While(stmt) => {
                let cond_label = self.label_gen.next_label();
                let loop_label = self.label_gen.next_label();
                let end_label = self.label_gen.next_label();
                ast.query_expr(&stmt.condition);
                self.push_terminator_or_crash(TerminatorKind::Goto(cond_label.clone()), &stmt.condition.into());
                self.begin_block(cond_label.clone());
                let condition = self.gen_expr(ast, stmt.condition,global_scope);
                let if_instruction = TerminatorKind::If(condition, loop_label.clone(), end_label.clone());
                self.push_terminator_or_crash(if_instruction, &stmt.condition.into());
                self.begin_block(loop_label.clone());
                self.gen_stmt(ast, stmt.body,global_scope);
                self.push_terminator_or_crash(TerminatorKind::Goto(cond_label), &stmt.body.into());
                self.begin_block(end_label);
            }
            ASTStatementKind::FuncDecl(_) => {}
            ASTStatementKind::Return(stmt) => {
                let primary = stmt.return_value.map(|expr_id| self.gen_expr(ast, expr_id,global_scope));
                self.push_terminator_or_crash(TerminatorKind::Return(primary), &stmt_id.into());
                if !stmt.is_top_level {
                    self.begin_block(self.label_gen.next_label());
                }
            }
            ASTStatementKind::Class(_) => {}
        };
    }

    fn gen_expr(&mut self, ast: &Ast, expr_id: ASTExprId,global_scope: &GlobalScope) -> Primary {
        let expr = ast.query_expr(&expr_id);
        let (expr, instructions): (Primary, Option<Vec<Instruction>>) = match &expr.kind {
            ASTExpressionKind::Number(expr) => {
                let primary = Primary::Integer(expr.number);
                (primary, None)
            }
            ASTExpressionKind::Binary(expr) => {
                let left = self.gen_expr(ast, expr.left,global_scope);
                let right = self.gen_expr(ast, expr.right,global_scope);
                let (primary, variable) = self.next_temp_var(
                    Type::I64
                );
                let instruction = self.get_binary_expr_instruction(left, right, variable.clone(), &expr.operator.kind, &expr_id);
                (primary, Some(vec![instruction]))
            }
            ASTExpressionKind::Unary(expr) => {
                let operand = self.gen_expr(ast, expr.operand,global_scope);
                let (primary, variable) = self.next_temp_var(
                    Type::I64
                );
                let instruction = self.get_unary_expr_instruction(operand, variable.clone(), &expr.operator.kind, &expr_id);
                (primary, Some(vec![instruction]))
            }
            ASTExpressionKind::Parenthesized(expr) => {
                (self.gen_expr(ast, expr.expression,global_scope), None)
            }
            ASTExpressionKind::Identifier(expr) => {
                let primary = match &expr.kind {
                    ASTIdentifierKind::Variable(variable) => Primary::Variable(variable.clone()),
                    ASTIdentifierKind::Class(class) => Primary::New(class.clone()),
                    ASTIdentifierKind::Function(function) => Primary::FuncRef(function.clone()),
                    ASTIdentifierKind::Unknown => panic!("Unknown identifier"),
                };
                (primary, None)
            }
            ASTExpressionKind::Assignment(expr) => {
                let assignment = self.gen_expr(ast, expr.expression,global_scope);
                let variable = expr.variable.as_ref().unwrap();
                let primary = Primary::Variable(variable.clone());
                let instruction = Instruction {
                    kind: InstructionKind::Store(variable.clone(), assignment),
                    assign_to: None,
                    node_id: expr_id.into(),
                };
                (primary, Some(vec![instruction]))
            }
            ASTExpressionKind::Boolean(expr) => {
                let primary = Primary::Boolean(expr.value);
                (primary, None)
            }
            ASTExpressionKind::Call(expr) => {
                let mut args = Vec::new();
                for arg in expr.arguments.iter() {
                    args.push(self.gen_expr(ast, *arg,global_scope));
                }
                let name = match &ast.query_expr(&expr.callee).ty {
                    Type::Function(FunctionType { parameters, return_type, name }) => name,
                    Type::Class(class_name) => {
                        class_name
                    }
                    _ => panic!("Call expression callee is not a function"),
                };
                let primary = Primary::Call(name.clone(), args);
                (primary, None)
            }
            ASTExpressionKind::Error(_) => {
                panic!("Error expression found in IRGen: {:?}", expr);
            }
            ASTExpressionKind::String(expr) => {
                let primary = Primary::String(expr.string.to_string().clone());
                (primary, None)
            }
            ASTExpressionKind::MemberAccess(member_expr) => {
                let obj_expr = ast.query_expr(&member_expr.object);
                let obj = self.gen_expr(ast, member_expr.object,global_scope);
                let primary = match &obj_expr.ty {
                    Type::Class(class_name) => {
                        let class = global_scope.lookup_class(class_name.as_str()).unwrap();
                        let member_name = member_expr.target.span.literal.as_str();
                        match class.lookup_field(member_name) {
                            Some((_, offset)) => {
                                let member = Member{
                                    index: offset,
                                    ty: expr.ty.clone(),
                                };
                                Primary::MemberAccess(Box::new(obj), member)
                            }
                            None => {
                                let function = class.lookup_method(member_name).unwrap();
                                Primary::MethodAccess(Box::new(obj), function.clone())
                            }
                        }
                    }
                    _ => {
                        panic!("Member access expression object is not a class")
                    }
                };

                (primary, None)
            }
            ASTExpressionKind::Self_(_) => {
                let primary = Primary::Self_(expr.ty.clone());
                (primary, None)
            }

        };

        if let Some(instructions) = instructions {
            self.push_instructions(instructions);
        }
        expr
    }

    fn get_unary_expr_instruction(&self, operand: Primary, assign_to: VariableSymbol, operator: &ASTUnaryOperatorKind, expr_id: &ASTExprId) -> Instruction {
        let operator: IRUnaryOperator = operator.into();
        Instruction {
            kind: InstructionKind::Unary(operator, operand),
            assign_to: Some(assign_to),
            node_id: expr_id.into(),
        }
    }

    fn get_binary_expr_instruction(&self, left: Primary, right: Primary, assign_to: VariableSymbol, operator: &ASTBinaryOperatorKind, expr_id: &ASTExprId) -> Instruction {
        let op = operator.into();

        Instruction {
            kind: InstructionKind::Binary(op, left, right),
            assign_to: Some(assign_to),
            node_id: expr_id.into(),
        }
    }

    fn next_temp_var(&mut self, ty: Type) -> (Primary, VariableSymbol) {
        let temp_var = self.temp_var_gen.next_temp_var(
            ty
        );
        let primary = Primary::Variable(temp_var.clone());
        (primary, temp_var)
    }

    fn push_instruction(&mut self, instruction: InstructionKind, assign_to: Option<VariableSymbol>, node_id: &ASTNodeId) {
        let instruction = Instruction {
            kind: instruction,
            assign_to,
            node_id: *node_id,
        };
        let block = self.basic_blocks.last_mut().unwrap();
        block.instructions.push(instruction);
    }

    fn push_instruction_no_assign(&mut self, instruction: InstructionKind, node_id: &ASTNodeId) {
        self.push_instruction(instruction, None, node_id);
    }

    fn maybe_push_terminator(&mut self, terminator: TerminatorKind, node_id: &ASTNodeId) -> () {
        self.push_terminator(terminator, node_id).unwrap_or(())
    }

    fn push_terminator_or_crash(&mut self, terminator: TerminatorKind, node_id: &ASTNodeId) -> () {
        self.push_terminator(terminator, node_id).unwrap()
    }

    fn push_terminator(&mut self, terminator: TerminatorKind, node_id: &ASTNodeId) -> Result<(), String> {
        let terminator = Terminator::new(terminator, *node_id);
        let block = self.basic_blocks.last_mut().unwrap();

        if block.terminator.kind == TerminatorKind::Unresolved {
            block.terminator = terminator;
            Ok(())
        } else {
            Err(format!("Cannot set terminator to {} when it is already set to {}", terminator, block.terminator))
        }
    }

    fn push_instructions(&mut self, instructions: Vec<Instruction>) {
        let block = self.basic_blocks.last_mut().unwrap();
        block.instructions.extend(instructions);
    }

    fn begin_function(&mut self, name: &str, function_symbol: &FunctionSymbol) {
        let label = Label::new(name.to_string());
        self.functions.push(label.clone());
        self.begin_block(label);
        let block = self.basic_blocks.last_mut().unwrap();
        block.function = Some(function_symbol.clone());
    }

    fn begin_block(&mut self, label: Label) -> &BasicBlock {
        let block = BasicBlock {
            label,
            instructions: Vec::new(),
            terminator: Terminator::unresolved(),
            function: None,
        };
        self.basic_blocks.push(block);
        self.basic_blocks.last().unwrap()
    }

    fn merge_top_level_statements_into_main(
        &self,
        ast: &mut Ast,
        global_scope: &mut GlobalScope,
    ) {
        let main = global_scope.lookup_function("main");
        let mut statements = ast.top_level_statements.clone();
        match main {
            None => {
                let body = ast.block_statement(Token::new(TokenKind::OpenBrace, TextSpan::default()),statements,Token::new(TokenKind::CloseBrace, TextSpan::default()));
                global_scope.declare_function(
                    "main",
                    Some(&body.id),
                    vec![],
                    Type::Void,
                    vec![],
                ).unwrap();
            }
            Some(main) => {

                if let Some(main_body) = main.body {
                    statements.push(main_body);
                }
                let open_brace = main.body.as_ref().map(|id| ast.query_stmt(id).into_block_stmt().open_brace.clone()).unwrap_or(Token::new(TokenKind::OpenBrace, TextSpan::default()));
                let close_brace = main.body.as_ref().map(|id| ast.query_stmt(id).into_block_stmt().close_brace.clone()).unwrap_or(Token::new(TokenKind::CloseBrace, TextSpan::default()));
                let body = ast.block_statement(open_brace, statements, close_brace);
                global_scope.replace_function(
                    "main",
                    Some(&body.id),
                    vec![],
                    main.return_type.clone(),
                    main.modifiers.clone(),
                );
            }
        }
    }
}
