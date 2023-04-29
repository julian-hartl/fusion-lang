use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt::{Error, format};
use std::ops::Deref;
use std::path::Path;
use std::process::{Command, exit};

use inkwell::{AddressSpace, IntPredicate, OptimizationLevel};
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::execution_engine::{ExecutionEngine, JitFunction};
use inkwell::module::{Linkage, Module};
use inkwell::support::LLVMString;
use inkwell::targets::{CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine, TargetTriple};
use inkwell::types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum};
use inkwell::types::AnyTypeEnum::IntType;
use inkwell::values::{AsValueRef, BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue, IntMathValue, IntValue, PointerValue};

use crate::ast::ASTStmtId;
use crate::compilation::symbols::function::FunctionSymbol;
use crate::ir;
use crate::ir::{basic_block, IR};
use crate::ir::basic_block::{BasicBlock, Label};
use crate::ir::instruction::{Instruction, InstructionKind, IRBinaryOperator, IRUnaryOperator, Member, Primary, Terminator};
use crate::ir::terminator::TerminatorKind;
use crate::typings::{FunctionType, SymbolKind, Type};

pub struct LLVMCodegen<'ctx> {
    pub result: String,
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    pub type_builder: LLVMTypeBuilder<'ctx>,
    pub ptrs: HashMap<String, PointerValue<'ctx>>,
    pub temps: HashMap<String, BasicValueEnum<'ctx>>,
    pub basic_blocks: HashMap<Label, inkwell::basic_block::BasicBlock<'ctx>>,
}


#[derive(Debug, Clone, PartialEq)]
struct LLVMFunction<'a> {
    function: FunctionValue<'a>,
}

impl<'ctx> LLVMFunction<'ctx> {
    pub fn new(function: FunctionValue<'ctx>) -> Self {
        Self {
            function,
        }
    }
}

pub struct LLVMTypeBuilder<'ctx> {
    context: &'ctx Context,
    types: RefCell<HashMap<Type, BasicTypeEnum<'ctx>>>,
    functions: RefCell<HashMap<ASTStmtId, inkwell::types::FunctionType<'ctx>>>,
}

pub type Entry = unsafe extern "C" fn() -> i64;

impl<'ctx> LLVMTypeBuilder<'ctx> {
    pub fn new(context: &'ctx Context) -> Self {
        let mut types = HashMap::new();
        types.insert(Type::I64, context.i64_type().into());
        types.insert(Type::Bool, context.bool_type().into());
        Self {
            context,
            types: RefCell::new(types),
            functions: RefCell::new(HashMap::new()),
        }
    }

    pub fn get_type(&self, ty: &Type) -> BasicTypeEnum<'ctx> {
        assert_ne!(ty, &Type::Error);
        assert_ne!(ty, &Type::Unresolved);
        assert_ne!(ty, &Type::Void);
        let mut types = self.types.borrow_mut();
        if !types.contains_key(ty) {
            match ty {
                Type::I64 => types.insert(Type::I64, self.context.i64_type().into()),
                Type::Bool => types.insert(Type::Bool, self.context.bool_type().into()),
                Type::Str => types.insert(Type::Str, self.context.i8_type().ptr_type(AddressSpace::default()).into()),
                Type::Error | Type::Unresolved | Type::Void => panic!("Unresolved type"),
                Type::Class(name) => {
                    let struct_type = self.context.opaque_struct_type(&name);
                    types.insert(Type::Class(name.clone()), struct_type.into())
                }
                Type::Function(FunctionType { parameters, return_type, name }) => {
                    let parameter_types: Vec<BasicMetadataTypeEnum> = parameters.iter().map(|parameter| self.get_type(&parameter).into()).collect::<Vec<_>>();
                    match **return_type {
                        Type::Void => {
                            let function_type = self.context.void_type().fn_type(parameter_types.as_slice(), false).ptr_type(AddressSpace::default());
                            types.insert(ty.clone(), function_type.into())
                        }
                        _ => {
                            let function_type = self.get_type(&return_type).fn_type(parameter_types.as_slice(), false).ptr_type(AddressSpace::default());
                            types.insert(ty.clone(), function_type.into())
                        }
                    }
                }

            };
        }
        types.get(ty).unwrap().clone()
    }

    pub fn get_function_type(&self, function: &FunctionSymbol) ->  inkwell::types::FunctionType<'ctx> {
        if self.functions.borrow().contains_key(&function.body.unwrap()) {
            return self.functions.borrow().get(&function.body.unwrap()).unwrap().clone();
        }
        let mut parameter_types: Vec<BasicMetadataTypeEnum> = function.parameters.iter().map(|parameter| self.get_type(&parameter.ty).into()).collect::<Vec<_>>();
        let function_type = match function.return_type {
            Type::Void => {
                self.context.void_type().fn_type(parameter_types.as_slice(), false)
            }
            _ => {
                self.get_type(&function.return_type).fn_type(parameter_types.as_slice(), false)
            }
        };
        self.functions.borrow_mut().insert(function.body.unwrap(), function_type);
        function_type
    }
}


impl<'ctx> LLVMCodegen<'ctx> {
    pub fn new(
        context: &'ctx Context,
        type_builder: LLVMTypeBuilder<'ctx>,
    ) -> Self {
        let module = context.create_module("fusion");
        let builder = context.create_builder();
        Self { result: String::new(), context, module, builder, type_builder, ptrs: HashMap::new(), temps: HashMap::new(), basic_blocks: HashMap::new() }
    }

    pub fn include_c_stdlib(&self) {
        // Include the C standard library header files
        let i8_type = self.context.i8_type();
        let void_type = self.context.void_type();
        let printf_type = void_type.fn_type(&[i8_type.ptr_type(AddressSpace::default()).into()], true);
        self.module.add_function("printf", printf_type, Some(Linkage::External));
    }

    pub fn get_type(&self, ty: &Type) -> BasicTypeEnum<'ctx> {
        self.type_builder.get_type(ty)
    }

    pub fn get_function_type(&self, function: &FunctionSymbol) -> inkwell::types::FunctionType {
        self.type_builder.get_function_type(function)
    }

    pub fn append_basic_block(&self, basic_block: &BasicBlock, function: FunctionValue<'ctx>) -> inkwell::basic_block::BasicBlock<'ctx> {
        let basic_block = self.context.append_basic_block(
            function,
            &basic_block.label.name,
        );
        self.builder.position_at_end(basic_block);
        basic_block
    }

    pub fn get_assign_to(&self, instruction: &Instruction) -> String {
        instruction.assign_to.as_ref().map(|s| s.name.as_str()).unwrap_or("tmp").to_string()
    }
}

impl<'ctx> LLVMCodegen<'ctx> {
    pub fn gen(&mut self, ir: &IR) -> Result<String, Error> {
        self.include_c_stdlib();

        let mut current_function = None;
        let mut basic_blocks: Vec<(&basic_block::BasicBlock, inkwell::basic_block::BasicBlock<'ctx>)> = Vec::new();
        for basic_block in ir.basic_blocks.iter() {
            let appended = match &basic_block.function {
                None => {
                    self.append_basic_block(basic_block, current_function.unwrap())
                }
                Some(symbol) => {
                    let function_type = self.type_builder.get_function_type(symbol);
                    let function = self.module.add_function(&symbol.name, function_type, None);
                    current_function = Some(function);
                    let appended = if !symbol.parameters.is_empty() {
                        let block = self.context.append_basic_block(function, "entry");
                        self.builder.position_at_end(block);
                        block
                    } else {
                        self.append_basic_block(basic_block, function)
                    };
                    for (arg, parameter) in function.get_param_iter().zip(symbol.parameters.iter()) {
                        let ptr = self.builder.build_alloca(arg.get_type(), &parameter.name);
                        self.builder.build_store(ptr, arg);
                        self.ptrs.insert(parameter.name.clone(), ptr);
                    }
                    appended
                }
            };
            basic_blocks.push((basic_block, appended));
            self.basic_blocks.insert(basic_block.label.clone(), appended);
        }
        for (basic_block, block) in basic_blocks.iter() {
            self.builder.position_at_end(*block);
            self.gen_basic_block(basic_block);
        }

        Ok(self.module.print_to_string().to_string())
    }

    pub fn gen_basic_block(&mut self, basic_block: &BasicBlock) {
        for instruction in basic_block.instructions.iter() {
            self.gen_instruction(instruction);
        }
        self.gen_terminator(&basic_block.terminator)
    }

    pub fn gen_instruction(&mut self, instruction: &Instruction) {
        let assign_to = &self.get_assign_to(instruction);
        let result = match &instruction.kind {
            InstructionKind::Binary(op, lhs, rhs) => {
                let (lhs, rhs) = self.gen_binary(lhs, rhs);
                let lhs = lhs.into_int_value();
                let rhs = rhs.into_int_value();
                match op {
                    IRBinaryOperator::Add => {
                        Some(self.builder.build_int_add::<IntValue>(lhs, rhs, assign_to))
                    }
                    IRBinaryOperator::Sub => {
                        Some(self.builder.build_int_sub::<IntValue>(lhs, rhs, assign_to))
                    }
                    IRBinaryOperator::Mul => {
                        Some(self.builder.build_int_mul::<IntValue>(lhs, rhs, assign_to))
                    }
                    IRBinaryOperator::Div => {
                        Some(self.builder.build_int_signed_div::<IntValue>(lhs, rhs, assign_to))
                    }
                    IRBinaryOperator::BitAnd => {
                        Some(self.builder.build_and::<IntValue>(lhs, rhs, assign_to))
                    }
                    IRBinaryOperator::BitOr => {
                        Some(self.builder.build_or::<IntValue>(lhs, rhs, assign_to))
                    }
                    IRBinaryOperator::BitXor => {
                        Some(self.builder.build_xor::<IntValue>(lhs, rhs, assign_to))
                    }
                    IRBinaryOperator::Eq => {
                        Some(self.builder.build_int_compare(IntPredicate::EQ, lhs, rhs, assign_to))
                    }
                    IRBinaryOperator::Neq => {
                        Some(self.builder.build_int_compare(IntPredicate::NE, lhs, rhs, assign_to))
                    }
                    IRBinaryOperator::Lt => {
                        Some(self.builder.build_int_compare(IntPredicate::SLT, lhs, rhs, assign_to))
                    }
                    IRBinaryOperator::Lte => {
                        Some(self.builder.build_int_compare(IntPredicate::SLE, lhs, rhs, assign_to))
                    }
                    IRBinaryOperator::Gt => {
                        Some(self.builder.build_int_compare(IntPredicate::SGT, lhs, rhs, assign_to))
                    }
                    IRBinaryOperator::Gte => {
                        Some(self.builder.build_int_compare(IntPredicate::SGE, lhs, rhs, assign_to))
                    }
                }
            }
            InstructionKind::Primary(primary) => {
                self.gen_primary(primary).map(|value| value.into_int_value())
            }
            InstructionKind::Store(var, primary) => {
                let primary = self.gen_primary_no_void(primary);
                self.builder.build_store(self.ptrs[
                                             &var.name
                                             ], primary);
                None
            }
            InstructionKind::Alloc(var) => {
                let ty = self.get_type(&var.ty);
                let pointer = self.builder.build_alloca(ty, &var.name);
                self.ptrs.insert(var.name.clone(), pointer);
                None
            }
            InstructionKind::Unary(op, operand) => {
                let operand = self.gen_primary_no_void(operand);
                match op {
                    IRUnaryOperator::Neg => {
                        Some(self.builder.build_int_neg::<IntValue>(operand.into_int_value(), assign_to))
                    }
                    IRUnaryOperator::BitNot => {
                        Some(self.builder.build_not::<IntValue>(operand.into_int_value(), assign_to))
                    }
                }
            }
        };

        if let Some(result) = result {
            self.temps.insert(assign_to.clone(), result.as_basic_value_enum());
        }
    }


    fn build_compare(&self, predicate: IntPredicate, lhs: &Primary, rhs: &Primary, assign_to: &str) -> IntValue<'ctx> {
        let (lhs, rhs) = self.gen_binary(lhs, rhs);
        self.builder.build_int_compare(predicate, lhs.into_int_value(), rhs.into_int_value(), assign_to)
    }

    fn gen_binary(&self, lhs: &Primary, rhs: &Primary) -> (BasicValueEnum<'ctx>, BasicValueEnum<'ctx>) {
        let lhs = self.gen_primary_no_void(lhs);
        let rhs = self.gen_primary_no_void(rhs);
        (lhs, rhs)
    }

    pub fn gen_primary_no_void(&self, primary: &Primary) -> BasicValueEnum<'ctx> {
        self.gen_primary(primary).unwrap()
    }

    pub fn gen_primary(&self, primary: &Primary) -> Option<BasicValueEnum<'ctx>> {
        match primary {
            Primary::Integer(value) => Some(self.context.i64_type().const_int(*value as u64, false).into()),
            Primary::Boolean(value) => Some(self.context.bool_type().const_int(*value as u64, false).into()),
            Primary::Variable(variable) => {
                let temp = self.temps.get(&variable.name);
                if let Some(temp) = temp {
                    return Some(*temp);
                }
                let pointer = *self.ptrs.get(&variable.name).expect(format!("No pointer for {}", variable.name).as_str());
                let ty = self.get_type(&variable.ty);
                let name = &variable.name;
                Some(self.builder.build_load(
                    ty,
                    pointer,
                    name,
                ))
            }
            Primary::Call(
                name, args
            ) => {
                let function = self.module.get_function(name).expect(format!("No function named {}", name).as_str());
                let args: Vec<BasicMetadataValueEnum> = args.iter().map(|arg| self.gen_primary_no_void(arg).into()).collect::<Vec<_>>();
                self.builder.build_call(function, &args, "call").try_as_basic_value().left()
            }
            Primary::String(expr) => {
                let string = self.context.const_string(expr.as_bytes(), false);
                let global = self.module.add_global(string.get_type(), None, "string");
                global.set_initializer(&string);
                global.set_constant(true);
                Some(global.as_basic_value_enum())
            }
            Primary::MemberAccess(obj, target) => {
                let obj = self.gen_primary_no_void(obj);
                let obj = obj.into_pointer_value();
                let target_ty = self.get_type(&target.ty);
                let ptr = self.builder.build_struct_gep(target_ty, obj, target.index, "member_access").expect("Failed to build struct gep");
                Some(self.builder.build_load(target_ty, ptr, "member_access").into())

            }
            Primary::Self_(_) => {
                // let self_ = self.ptrs.get("self").expect("No self pointer");
                // let self_ = self.builder.build_load(self.get_type(&Type::Self_), *self_, "self");
                // Some(self_)
                todo!()
            }
            Primary::FuncRef(func) => {
                let func = self.module.get_function(&func.name).expect(format!("No function named {}", func.name).as_str());
                Some(func.as_global_value().as_pointer_value().into())
            }
            Primary::New(class) => {
                let class = self.module.get_struct_type(&class.name).expect(format!("No class named {}", class.name).as_str());
                let pointer = self.builder.build_alloca(class, "new");
                Some(pointer.into())
            }
            Primary::MethodAccess(obj, method) => {
                let object = self.gen_primary_no_void(obj);
                let object = object.into_pointer_value();
                let method = self.module.get_function(&method.name).expect(format!("No function named {}", method.name).as_str());
                let ptr = method.as_global_value().as_pointer_value();
                Some(ptr.into())


            }
        }
    }

    pub fn gen_terminator(&self, terminator: &Terminator) {
        match &terminator.kind {
            TerminatorKind::Return(value) => {
                let value = value.as_ref().map(|value| self.gen_primary(value)).flatten();
                match value {
                    None => self.builder.build_return(None),
                    Some(value) => self.builder.build_return(Some(&value)),
                };
            }
            TerminatorKind::Goto(target) => {
                let target = self.get_basic_block(target);
                self.builder.build_unconditional_branch(target);
            }
            TerminatorKind::If(condition, true_target, false_target) => {
                let condition = self.gen_primary_no_void(condition);
                let true_target = self.get_basic_block(true_target);
                let false_target = self.get_basic_block(false_target);
                self.builder.build_conditional_branch(condition.into_int_value(), true_target, false_target);
            }
            TerminatorKind::Unresolved => {
                panic!("unresolved terminator");
            }
        }
    }

    fn get_basic_block(&self, label: &Label) -> inkwell::basic_block::BasicBlock {
        self.basic_blocks.get(label).expect(format!("No basic block for label {:?}", label).as_str()).clone()
    }


    pub fn get_jit(&self) -> Result<JitFunction<Entry>, LLVMString> {
        let exec = self.module.create_jit_execution_engine(OptimizationLevel::Aggressive)?;
        let function = unsafe { exec.get_function::<Entry>("main").expect("Could not find main") };
        Ok(function)
    }

    pub fn save_ir(&self) -> Result<(), LLVMString> {
        self.module.print_to_file("output.ll")
    }

    pub fn save_executable(&self) -> Result<(), ()> {
        let target_machine = self.create_target_machine();

        // Write the x86 assembly code to a file
        let output_file = Path::new("temp.o");
        target_machine.write_to_file(&self.module, FileType::Object, &output_file).map_err(|_| ())?;

        // Link the object file with the system's C runtime
        let mut command = Command::new("gcc");
        command.arg("-o").arg("output");
        command.arg("temp.o");
        command.status().expect("Could not run gcc");
        Ok(())
    }

    fn create_target_machine(&self) -> TargetMachine {
        let target_triple = TargetTriple::create("aarch64-apple-darwin");
        let target = Target::from_triple(&target_triple).expect("Could not create target");
        self.module.set_triple(&target_triple);

        // Create a target machine
        let target_machine = target.create_target_machine(
            &target_triple,
            "generic",
            "",
            OptimizationLevel::None,
            RelocMode::Default,
            CodeModel::Default,
        ).expect("Could not create target machine");
        target_machine
    }

    pub fn save_asm(&self) -> Result<(), LLVMString> {

        // Create a target machine
        let target_machine = self.create_target_machine();

        // Write the x86 assembly code to a file
        let output_file = Path::new("output.s");
        target_machine.write_to_file(&self.module, FileType::Assembly, &output_file)
    }
}


