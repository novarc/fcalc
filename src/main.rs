use rustyline;

mod lex;
mod parse;
use lex::{Token, lex};
use parse::{LangBlock, LangLine, parse_block};

use inkwell::OptimizationLevel;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::execution_engine::ExecutionEngine;
use inkwell::module::Module;
use inkwell::targets::{
	CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetMachine,
};

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::sync::{LazyLock, Mutex};

use inkwell::types::FloatType;
use inkwell::values::{FloatValue, FunctionValue};

// Global variable storage for the REPL session
static VARIABLES: LazyLock<Mutex<HashMap<String, f64>>> =
	LazyLock::new(|| Mutex::new(HashMap::new()));

// Global function storage for the REPL session
static FUNCTIONS: LazyLock<Mutex<HashMap<String, parse::LangFunction>>> =
	LazyLock::new(|| Mutex::new(HashMap::new()));

/// LLVM Code Generator for functions and expressions
struct LLVMCodeGen<'ctx> {
	context: &'ctx Context,
	module: Module<'ctx>,
	builder: Builder<'ctx>,
	execution_engine: ExecutionEngine<'ctx>,
	float_type: FloatType<'ctx>,
}

impl<'ctx> LLVMCodeGen<'ctx> {
	fn new(context: &'ctx Context) -> Result<Self, Box<dyn Error>> {
		let module = context.create_module("fcalc");
		let execution_engine = module.create_jit_execution_engine(OptimizationLevel::Aggressive)?;
		let builder = context.create_builder();
		let float_type = context.f64_type();

		Ok(LLVMCodeGen {
			context,
			module,
			builder,
			execution_engine,
			float_type,
		})
	}

	/// Initialize LLVM targets for binary generation
	fn initialize_targets() {
		Target::initialize_all(&InitializationConfig::default());
	}

	/// Create a new instance specifically for binary generation (without JIT engine)
	fn new_for_binary_gen(context: &'ctx Context) -> Result<Self, Box<dyn Error>> {
		let module = context.create_module("fcalc_binary");
		let builder = context.create_builder();
		let float_type = context.f64_type();

		// Create a dummy execution engine for compatibility, but we won't use it
		let execution_engine = module.create_jit_execution_engine(OptimizationLevel::Aggressive)?;

		Ok(LLVMCodeGen {
			context,
			module,
			builder,
			execution_engine,
			float_type,
		})
	}

	/// Generate an executable binary from the current module
	fn generate_executable(&self, output_path: &str) -> Result<(), Box<dyn Error>> {
		// Initialize targets
		Self::initialize_targets();

		// Get the native target triple
		let target_triple = TargetMachine::get_default_triple();
		let target = Target::from_triple(&target_triple)
			.map_err(|e| format!("Failed to get target from triple: {}", e))?;

		// Create target machine
		let target_machine = target
			.create_target_machine(
				&target_triple,
				&TargetMachine::get_host_cpu_name().to_string(),
				&TargetMachine::get_host_cpu_features().to_string(),
				inkwell::OptimizationLevel::Aggressive,
				RelocMode::Default,
				CodeModel::Default,
			)
			.ok_or("Failed to create target machine")?;

		// Set the target triple and data layout for the module
		self.module.set_triple(&target_triple);
		self.module
			.set_data_layout(&target_machine.get_target_data().get_data_layout());

		// Generate object file
		let object_path = format!("{}.o", output_path);
		target_machine
			.write_to_file(&self.module, FileType::Object, Path::new(&object_path))
			.map_err(|e| format!("Failed to write object file: {}", e))?;

		// Link the object file to create executable
		#[cfg(target_os = "macos")]
		let link_command = format!("clang -o {} {} -lm", output_path, object_path);

		#[cfg(target_os = "linux")]
		let link_command = format!("gcc -o {} {} -lm", output_path, object_path);

		#[cfg(target_os = "windows")]
		let link_command = format!("clang -o {}.exe {} -lm", output_path, object_path);

		// Execute the link command
		let output = std::process::Command::new("sh")
			.arg("-c")
			.arg(&link_command)
			.output();

		match output {
			Ok(result) => {
				if result.status.success() {
					// Clean up object file
					let _ = fs::remove_file(&object_path);
					println!("Successfully created executable: {}", output_path);
					Ok(())
				} else {
					let error_msg = String::from_utf8_lossy(&result.stderr);
					Err(format!("Linking failed: {}", error_msg).into())
				}
			}
			Err(e) => Err(format!("Failed to execute linker: {}", e).into()),
		}
	}

	/// Create a main function that calls a user-defined function
	fn create_main_function(
		&mut self,
		function_name: &str,
		args: &[f64],
	) -> Result<(), Box<dyn Error>> {
		// Create main function type: int main()
		let i32_type = self.context.i32_type();
		let main_fn_type = i32_type.fn_type(&[], false);
		let main_function = self.module.add_function("main", main_fn_type, None);

		let basic_block = self.context.append_basic_block(main_function, "entry");
		self.builder.position_at_end(basic_block);

		// Get the user function
		if let Some(user_function) = self.module.get_function(function_name) {
			// Prepare arguments
			let mut llvm_args = Vec::new();
			for &arg in args {
				llvm_args.push(self.float_type.const_float(arg).into());
			}

			// Call the user function
			let call_result = self
				.builder
				.build_call(user_function, &llvm_args, "call_user_func")
				.unwrap();

			// Print the result (simplified - in real implementation you'd need printf)
			// For now, just return 0
			let return_val = i32_type.const_int(0, false);
			self.builder.build_return(Some(&return_val)).unwrap();
		} else {
			// Function not found, return error code
			let return_val = i32_type.const_int(1, false);
			self.builder.build_return(Some(&return_val)).unwrap();
		}

		Ok(())
	}

	/// Compile a function definition to LLVM IR
	fn compile_function(
		&mut self,
		name: &str,
		function: &parse::LangFunction,
	) -> Result<FunctionValue<'ctx>, Box<dyn Error>> {
		// Create function type: all parameters and return value are f64
		let param_types: Vec<_> = (0..function.parameters.len())
			.map(|_| self.float_type.into())
			.collect();
		let fn_type = self.float_type.fn_type(&param_types, false);

		// Create the function
		let llvm_function = self.module.add_function(name, fn_type, None);
		let basic_block = self.context.append_basic_block(llvm_function, "entry");
		self.builder.position_at_end(basic_block);

		// Create parameter bindings
		let mut param_values = HashMap::new();
		for (i, param_name) in function.parameters.iter().enumerate() {
			let param_value = llvm_function
				.get_nth_param(i as u32)
				.ok_or(format!("Missing parameter {}", i))?
				.into_float_value();
			param_values.insert(param_name.clone(), param_value);
		}

		// Compile the function body
		let result = self.compile_block(&function.body, &param_values)?;

		// Return the result
		self.builder.build_return(Some(&result)).unwrap();

		Ok(llvm_function)
	}

	/// Compile a named function definition to LLVM IR
	fn compile_named_function(
		&mut self,
		named_function: &parse::LangNamedFunction,
	) -> Result<FunctionValue<'ctx>, Box<dyn Error>> {
		// Create function type: all parameters and return value are f64
		let param_types: Vec<_> = (0..named_function.parameters.len())
			.map(|_| self.float_type.into())
			.collect();
		let fn_type = self.float_type.fn_type(&param_types, false);

		// Create the function
		let llvm_function = self
			.module
			.add_function(&named_function.name, fn_type, None);
		let basic_block = self.context.append_basic_block(llvm_function, "entry");
		self.builder.position_at_end(basic_block);

		// Create parameter bindings
		let mut param_values = HashMap::new();
		for (i, param_name) in named_function.parameters.iter().enumerate() {
			let param_value = llvm_function
				.get_nth_param(i as u32)
				.ok_or(format!("Missing parameter {}", i))?
				.into_float_value();
			param_values.insert(param_name.clone(), param_value);
		}

		// Compile the function body
		let result = self.compile_block(&named_function.body, &param_values)?;

		// Return the result
		self.builder.build_return(Some(&result)).unwrap();

		Ok(llvm_function)
	}

	/// Compile a block of statements
	fn compile_block(
		&mut self,
		block: &parse::LangBlock,
		variables: &HashMap<String, FloatValue<'ctx>>,
	) -> Result<FloatValue<'ctx>, Box<dyn Error>> {
		let mut last_result = self.float_type.const_float(0.0);

		for item in &block.items {
			match item {
				parse::LangBlockItem::Line(line) => {
					// Check if the line contains function calls to other user-defined functions
					if self.contains_user_function_calls(line) {
						// Fall back to runtime evaluation for lines with function calls
						return Err(
							"Function contains calls to other functions - use runtime evaluation"
								.into(),
						);
					}
					last_result = self.compile_line(line, variables)?;
				}
				parse::LangBlockItem::Block(nested_block) => {
					last_result = self.compile_block(nested_block, variables)?;
				}
				parse::LangBlockItem::Function(_) => {
					// Nested functions not supported for now
					return Err("Nested functions not supported".into());
				}
				parse::LangBlockItem::NamedFunction(_) => {
					// Nested named functions not supported for now
					return Err("Nested named functions not supported".into());
				}
				parse::LangBlockItem::FunctionCall(call) => {
					// Check if this is a call to a user-defined function
					if self.is_user_defined_function(&call.name) {
						return Err(
							"Function contains calls to other functions - use runtime evaluation"
								.into(),
						);
					}
					last_result = self.compile_function_call(call, variables)?;
				}
			}
		}

		Ok(last_result)
	}

	/// Check if a line contains calls to user-defined functions
	fn contains_user_function_calls(&self, line: &parse::LangLine) -> bool {
		// Look for function call patterns in the tokens
		let mut i = 0;
		while i + 1 < line.tokens.len() {
			if let (Token::Symbol(name), Token::Operator(op)) =
				(&line.tokens[i], &line.tokens[i + 1])
			{
				if op.value == "(" && self.is_user_defined_function(&name.value) {
					return true;
				}
			}
			i += 1;
		}
		false
	}

	/// Check if a function name refers to a user-defined function
	fn is_user_defined_function(&self, name: &str) -> bool {
		match FUNCTIONS.lock() {
			Ok(functions) => functions.contains_key(name),
			Err(poisoned) => {
				let functions = poisoned.into_inner();
				functions.contains_key(name)
			}
		}
	}

	/// Compile a line (expression) to LLVM IR
	fn compile_line(
		&mut self,
		line: &parse::LangLine,
		variables: &HashMap<String, FloatValue<'ctx>>,
	) -> Result<FloatValue<'ctx>, Box<dyn Error>> {
		// Convert infix to postfix
		let postfix_tokens = infix_to_postfix(&line.tokens);
		self.compile_postfix_expression(&postfix_tokens, variables)
	}

	/// Compile a postfix expression to LLVM IR
	fn compile_postfix_expression(
		&mut self,
		tokens: &[Token],
		variables: &HashMap<String, FloatValue<'ctx>>,
	) -> Result<FloatValue<'ctx>, Box<dyn Error>> {
		let mut value_stack: Vec<FloatValue<'ctx>> = Vec::new();

		for token in tokens {
			match token {
				Token::Number(lex::LangNumber::Integer(int_val)) => {
					let value = self.float_type.const_float(int_val.value as f64);
					value_stack.push(value);
				}
				Token::Number(lex::LangNumber::RealNumber(real_val)) => {
					let value = self.float_type.const_float(real_val.value);
					value_stack.push(value);
				}
				Token::Symbol(symbol) => {
					// Look up variable value
					if let Some(&value) = variables.get(&symbol.value) {
						value_stack.push(value);
					} else {
						// Try to get global variable value
						let global_value = match VARIABLES.lock() {
							Ok(vars) => vars.get(&symbol.value).copied(),
							Err(poisoned) => {
								let vars = poisoned.into_inner();
								vars.get(&symbol.value).copied()
							}
						};

						let value = self.float_type.const_float(global_value.unwrap_or(0.0));
						value_stack.push(value);
					}
				}
				Token::Operator(op) => match op.value.as_str() {
					"+" => {
						if value_stack.len() >= 2 {
							let b = value_stack.pop().unwrap();
							let a = value_stack.pop().unwrap();
							let result = self.builder.build_float_add(a, b, "add").unwrap();
							value_stack.push(result);
						}
					}
					"-" => {
						if value_stack.len() >= 2 {
							let b = value_stack.pop().unwrap();
							let a = value_stack.pop().unwrap();
							let result = self.builder.build_float_sub(a, b, "sub").unwrap();
							value_stack.push(result);
						}
					}
					"*" => {
						if value_stack.len() >= 2 {
							let b = value_stack.pop().unwrap();
							let a = value_stack.pop().unwrap();
							let result = self.builder.build_float_mul(a, b, "mul").unwrap();
							value_stack.push(result);
						}
					}
					"/" => {
						if value_stack.len() >= 2 {
							let b = value_stack.pop().unwrap();
							let a = value_stack.pop().unwrap();

							// Check for division by zero by comparing to 0.0
							let zero = self.float_type.const_float(0.0);
							let is_zero = self
								.builder
								.build_float_compare(
									inkwell::FloatPredicate::OEQ,
									b,
									zero,
									"is_zero",
								)
								.unwrap();

							// Create basic blocks for division and error cases
							let function = self
								.builder
								.get_insert_block()
								.unwrap()
								.get_parent()
								.unwrap();
							let div_bb = self.context.append_basic_block(function, "div");
							let error_bb = self.context.append_basic_block(function, "error");
							let continue_bb = self.context.append_basic_block(function, "continue");

							// Branch based on zero check
							self.builder
								.build_conditional_branch(is_zero, error_bb, div_bb)
								.unwrap();

							// Division block
							self.builder.position_at_end(div_bb);
							let result = self.builder.build_float_div(a, b, "div").unwrap();
							self.builder
								.build_unconditional_branch(continue_bb)
								.unwrap();

							// Error block - return NaN to indicate error
							self.builder.position_at_end(error_bb);
							let nan = self.float_type.const_float(f64::NAN);
							self.builder
								.build_unconditional_branch(continue_bb)
								.unwrap();

							// Continue block - phi node to get the result
							self.builder.position_at_end(continue_bb);
							let phi = self
								.builder
								.build_phi(self.float_type, "div_result")
								.unwrap();
							phi.add_incoming(&[(&result, div_bb), (&nan, error_bb)]);

							value_stack.push(phi.as_basic_value().into_float_value());
						}
					}
					_ => {
						return Err(format!("Unsupported operator: {}", op.value).into());
					}
				},
				_ => {
					return Err("Unsupported token type in expression".into());
				}
			}
		}

		value_stack.last().copied().ok_or("Empty expression".into())
	}

	/// Compile a function call
	fn compile_function_call(
		&mut self,
		call: &parse::LangFunctionCall,
		variables: &HashMap<String, FloatValue<'ctx>>,
	) -> Result<FloatValue<'ctx>, Box<dyn Error>> {
		// Get the function from the module
		let function = self
			.module
			.get_function(&call.name)
			.ok_or(format!("Function '{}' not found", call.name))?;

		// Compile arguments
		let mut arg_values = Vec::new();
		for arg_tokens in &call.arguments {
			let postfix = infix_to_postfix(arg_tokens);
			let arg_value = self.compile_postfix_expression(&postfix, variables)?;
			arg_values.push(arg_value.into());
		}

		// Call the function
		let call_site = self
			.builder
			.build_call(function, &arg_values, "call")
			.unwrap();
		Ok(call_site
			.try_as_basic_value()
			.left()
			.unwrap()
			.into_float_value())
	}
}

/// Check if a function contains calls to other user-defined functions
fn function_contains_user_function_calls(function: &parse::LangFunction) -> bool {
	contains_user_function_calls_in_block(&function.body)
}

/// Check if a block contains calls to user-defined functions
fn contains_user_function_calls_in_block(block: &parse::LangBlock) -> bool {
	for item in &block.items {
		match item {
			parse::LangBlockItem::Line(line) => {
				if contains_user_function_calls_in_line(line) {
					return true;
				}
			}
			parse::LangBlockItem::Block(nested_block) => {
				if contains_user_function_calls_in_block(nested_block) {
					return true;
				}
			}
			parse::LangBlockItem::FunctionCall(call) => {
				if is_user_defined_function_global(&call.name) {
					return true;
				}
			}
			_ => {}
		}
	}
	false
}

/// Check if a line contains calls to user-defined functions
fn contains_user_function_calls_in_line(line: &parse::LangLine) -> bool {
	let mut i = 0;
	while i + 1 < line.tokens.len() {
		if let (Token::Symbol(name), Token::Operator(op)) = (&line.tokens[i], &line.tokens[i + 1]) {
			if op.value == "(" && is_user_defined_function_global(&name.value) {
				return true;
			}
		}
		i += 1;
	}
	false
}

/// Check if a function name refers to a user-defined function (global version)
fn is_user_defined_function_global(name: &str) -> bool {
	match FUNCTIONS.lock() {
		Ok(functions) => functions.contains_key(name),
		Err(poisoned) => {
			let functions = poisoned.into_inner();
			functions.contains_key(name)
		}
	}
}

/// Evaluate a function at runtime using the interpreter
fn evaluate_function_at_runtime(
	function: &parse::LangFunction,
	arg_values: &[f64],
) -> Result<f64, Box<dyn Error>> {
	// Create a temporary variable map with the function parameters
	let original_variables = {
		match VARIABLES.lock() {
			Ok(vars) => vars.clone(),
			Err(poisoned) => poisoned.into_inner().clone(),
		}
	};

	// Set up parameter bindings
	{
		match VARIABLES.lock() {
			Ok(mut vars) => {
				for (i, param_name) in function.parameters.iter().enumerate() {
					if i < arg_values.len() {
						vars.insert(param_name.clone(), arg_values[i]);
					}
				}
			}
			Err(poisoned) => {
				let mut vars = poisoned.into_inner();
				for (i, param_name) in function.parameters.iter().enumerate() {
					if i < arg_values.len() {
						vars.insert(param_name.clone(), arg_values[i]);
					}
				}
			}
		}
	}

	// Evaluate the function body with function call preprocessing
	// We need to manually process each line to ensure function calls are handled
	let result = eval_block_with_function_preprocessing(&function.body);

	// Restore original variables
	{
		match VARIABLES.lock() {
			Ok(mut vars) => *vars = original_variables,
			Err(poisoned) => {
				let mut vars = poisoned.into_inner();
				*vars = original_variables;
			}
		}
	}

	match result {
		Some(value) => Ok(value),
		None => Err("Function evaluation returned no result".into()),
	}
}

/// Evaluate a block with proper function call preprocessing
fn eval_block_with_function_preprocessing(block: &parse::LangBlock) -> Option<f64> {
	let mut last_result = None;
	let mut has_function_definition = false;

	for item in &block.items {
		match item {
			parse::LangBlockItem::Line(line) => {
				let result = eval_line(line); // eval_line already does function call preprocessing
				if result.is_some() {
					last_result = result;
				}
			}
			parse::LangBlockItem::Block(nested_block) => {
				let result = eval_block_with_function_preprocessing(nested_block);
				if result.is_some() {
					last_result = result;
				}
			}
			parse::LangBlockItem::Function(_) => {
				has_function_definition = true;
			}
			parse::LangBlockItem::NamedFunction(_) => {
				has_function_definition = true;
			}
			parse::LangBlockItem::FunctionCall(call) => match execute_function_call(call) {
				Ok(value) => {
					last_result = Some(value);
				}
				Err(e) => {
					println!("Error executing function call: {}", e);
					return None;
				}
			},
		}
	}

	// If there's a function definition in the block, return None
	if has_function_definition {
		None
	} else {
		last_result
	}
}

/// Compile and store a named function using LLVM
fn compile_and_store_named_function(
	named_function: &parse::LangNamedFunction,
) -> Result<(), Box<dyn Error>> {
	// Convert to LangFunction for storage
	let function = parse::LangFunction {
		parameters: named_function.parameters.clone(),
		body: named_function.body.clone(),
	};

	// Check if this function contains calls to other functions
	if function_contains_user_function_calls(&function) {
		// Store the function for runtime evaluation, skip LLVM compilation
		match FUNCTIONS.lock() {
			Ok(mut functions) => {
				functions.insert(named_function.name.clone(), function);
			}
			Err(poisoned) => {
				let mut functions = poisoned.into_inner();
				functions.insert(named_function.name.clone(), function);
			}
		}
		return Ok(());
	}

	// Try LLVM compilation for simple functions
	let context = Context::create();
	let mut codegen = LLVMCodeGen::new(&context)?;

	// Try to compile the named function
	match codegen.compile_named_function(named_function) {
		Ok(_) => {
			// Successfully compiled with LLVM, store the function
			match FUNCTIONS.lock() {
				Ok(mut functions) => {
					functions.insert(named_function.name.clone(), function);
				}
				Err(poisoned) => {
					let mut functions = poisoned.into_inner();
					functions.insert(named_function.name.clone(), function);
				}
			}
		}
		Err(e) if e.to_string().contains("use runtime evaluation") => {
			// Failed due to function calls, store for runtime evaluation
			match FUNCTIONS.lock() {
				Ok(mut functions) => {
					functions.insert(named_function.name.clone(), function);
				}
				Err(poisoned) => {
					let mut functions = poisoned.into_inner();
					functions.insert(named_function.name.clone(), function);
				}
			}
		}
		Err(e) => return Err(e),
	}

	// println!(
	// 	"Successfully compiled named function '{}' with LLVM",
	// 	named_function.name
	// );
	Ok(())
}

/// Execute a function call using LLVM
fn execute_function_call(call: &parse::LangFunctionCall) -> Result<f64, Box<dyn Error>> {
	// Check if function exists in our store
	let function_opt = match FUNCTIONS.lock() {
		Ok(functions) => functions.get(&call.name).cloned(),
		Err(poisoned) => {
			let functions = poisoned.into_inner();
			functions.get(&call.name).cloned()
		}
	};

	if let Some(function) = function_opt {
		// Evaluate argument expressions to get actual values
		let mut arg_values = Vec::new();
		for arg_tokens in &call.arguments {
			let unary_processed = preprocess_unary_minus(arg_tokens);
			let postfix = infix_to_postfix(&unary_processed);
			match execute_postfix_tokens(&postfix)? {
				Some(value) => arg_values.push(value),
				None => return Err("Argument expression evaluation failed".into()),
			}
		}

		// Check argument count matches function parameters
		if arg_values.len() != function.parameters.len() {
			return Err(format!(
				"Function '{}' expects {} arguments, got {}",
				call.name,
				function.parameters.len(),
				arg_values.len()
			)
			.into());
		}

		// Check if this function contains calls to other functions
		if function_contains_user_function_calls(&function) {
			// Use runtime evaluation instead of LLVM compilation
			return evaluate_function_at_runtime(&function, &arg_values);
		}

		// Create a new LLVM context and compile the function for execution
		let context = Context::create();
		let mut codegen = LLVMCodeGen::new(&context)?;

		// Try to compile the function - if it fails due to function calls, fall back to runtime
		let _llvm_function = match codegen.compile_function(&call.name, &function) {
			Ok(f) => f,
			Err(e) if e.to_string().contains("use runtime evaluation") => {
				// Fall back to runtime evaluation
				return evaluate_function_at_runtime(&function, &arg_values);
			}
			Err(e) => return Err(e),
		};

		// Get JIT function pointer and execute based on argument count
		unsafe {
			match arg_values.len() {
				0 => {
					type Func0 = unsafe extern "C" fn() -> f64;
					let jit_fn: inkwell::execution_engine::JitFunction<Func0> =
						codegen.execution_engine.get_function(&call.name)?;
					let result = jit_fn.call();
					if result.is_nan() {
						Err("Division by zero".into())
					} else {
						Ok(result)
					}
				}
				1 => {
					type Func1 = unsafe extern "C" fn(f64) -> f64;
					let jit_fn: inkwell::execution_engine::JitFunction<Func1> =
						codegen.execution_engine.get_function(&call.name)?;
					let result = jit_fn.call(arg_values[0]);
					if result.is_nan() {
						Err("Division by zero".into())
					} else {
						Ok(result)
					}
				}
				2 => {
					type Func2 = unsafe extern "C" fn(f64, f64) -> f64;
					let jit_fn: inkwell::execution_engine::JitFunction<Func2> =
						codegen.execution_engine.get_function(&call.name)?;
					let result = jit_fn.call(arg_values[0], arg_values[1]);
					if result.is_nan() {
						Err("Division by zero".into())
					} else {
						Ok(result)
					}
				}
				3 => {
					type Func3 = unsafe extern "C" fn(f64, f64, f64) -> f64;
					let jit_fn: inkwell::execution_engine::JitFunction<Func3> =
						codegen.execution_engine.get_function(&call.name)?;
					let result = jit_fn.call(arg_values[0], arg_values[1], arg_values[2]);
					if result.is_nan() {
						Err("Division by zero".into())
					} else {
						Ok(result)
					}
				}
				4 => {
					type Func4 = unsafe extern "C" fn(f64, f64, f64, f64) -> f64;
					let jit_fn: inkwell::execution_engine::JitFunction<Func4> =
						codegen.execution_engine.get_function(&call.name)?;
					let result =
						jit_fn.call(arg_values[0], arg_values[1], arg_values[2], arg_values[3]);
					if result.is_nan() {
						Err("Division by zero".into())
					} else {
						Ok(result)
					}
				}
				5 => {
					type Func5 = unsafe extern "C" fn(f64, f64, f64, f64, f64) -> f64;
					let jit_fn: inkwell::execution_engine::JitFunction<Func5> =
						codegen.execution_engine.get_function(&call.name)?;
					let result = jit_fn.call(
						arg_values[0],
						arg_values[1],
						arg_values[2],
						arg_values[3],
						arg_values[4],
					);
					if result.is_nan() {
						Err("Division by zero".into())
					} else {
						Ok(result)
					}
				}
				_ => Err(format!(
					"Functions with {} parameters not supported yet (max 5)",
					arg_values.len()
				)
				.into()),
			}
		}
	} else {
		Err(format!("Function '{}' not found", call.name).into())
	}
}

/// Preprocess tokens to handle function calls in expressions
fn preprocess_tokens_for_function_calls(tokens: &[Token]) -> Result<Vec<Token>, Box<dyn Error>> {
	let mut result = Vec::new();
	let mut i = 0;

	while i < tokens.len() {
		if i + 1 < tokens.len() {
			// Check for function call pattern: Symbol followed by (
			if let (Token::Symbol(func_name), Token::Operator(op)) = (&tokens[i], &tokens[i + 1]) {
				if op.value == "(" {
					// Found a function call pattern, parse arguments
					let mut j = i + 2; // Start after the opening parenthesis
					let mut paren_count = 1;
					let mut arg_tokens = Vec::new();
					let mut current_arg = Vec::new();

					while j < tokens.len() && paren_count > 0 {
						match &tokens[j] {
							Token::Operator(op) if op.value == "(" => {
								paren_count += 1;
								current_arg.push(tokens[j].clone());
							}
							Token::Operator(op) if op.value == ")" => {
								paren_count -= 1;
								if paren_count == 0 {
									// End of function call
									if !current_arg.is_empty() {
										arg_tokens.push(current_arg.clone());
									}
								} else {
									current_arg.push(tokens[j].clone());
								}
							}
							Token::Operator(op) if op.value == "," && paren_count == 1 => {
								// Argument separator at top level
								if !current_arg.is_empty() {
									arg_tokens.push(current_arg.clone());
									current_arg.clear();
								}
							}
							_ => {
								current_arg.push(tokens[j].clone());
							}
						}
						j += 1;
					}

					// Recursively preprocess arguments for nested function calls
					let mut processed_arg_tokens = Vec::new();
					for arg in arg_tokens {
						match preprocess_tokens_for_function_calls(&arg) {
							Ok(processed_arg) => processed_arg_tokens.push(processed_arg),
							Err(e) => {
								return Err(format!(
									"Error preprocessing nested function call: {}",
									e
								)
								.into());
							}
						}
					}

					// Execute the function call and replace with the result
					let function_call = parse::LangFunctionCall {
						name: func_name.value.clone(),
						arguments: processed_arg_tokens,
					};

					match execute_function_call(&function_call) {
						Ok(result_value) => {
							// Replace the function call with its result as a number token
							result.push(Token::Number(lex::LangNumber::RealNumber(
								lex::LangRealNumber {
									value: result_value,
								},
							)));
						}
						Err(e) => {
							return Err(format!("Function call error: {}", e).into());
						}
					}

					i = j; // Skip past the function call tokens
					continue;
				}
			}
		}

		// Not a function call, add the token as-is
		result.push(tokens[i].clone());
		i += 1;
	}

	Ok(result)
}

fn execute_postfix_tokens(tokens: &[Token]) -> Result<Option<f64>, Box<dyn Error>> {
	// For assignment operations, we need to handle them at runtime rather than compile time
	// So we'll evaluate the postfix expression directly without LLVM for now
	let mut value_stack: Vec<f64> = Vec::new();
	let mut variable_stack: Vec<String> = Vec::new(); // For tracking variable names in assignment

	for token in tokens {
		match token {
			Token::Number(lex::LangNumber::Integer(int_val)) => {
				value_stack.push(int_val.value as f64);
				variable_stack.push(String::new()); // Empty string for non-variables
			}
			Token::Number(lex::LangNumber::RealNumber(real_val)) => {
				value_stack.push(real_val.value);
				variable_stack.push(String::new()); // Empty string for non-variables
			}
			Token::Symbol(symbol) => {
				// Always track the symbol name for potential assignment
				variable_stack.push(symbol.value.clone());

				// Check if this symbol is a variable, if so push its value
				let value = match VARIABLES.lock() {
					Ok(variables) => variables.get(&symbol.value).copied(),
					Err(poisoned) => {
						let variables = poisoned.into_inner();
						variables.get(&symbol.value).copied()
					}
				};

				if let Some(value) = value {
					value_stack.push(value);
				} else {
					// For new variables, push 0 as placeholder
					value_stack.push(0.0);
				}
			}
			Token::Operator(op) => match op.value.as_str() {
				"=" => {
					if value_stack.len() >= 2 && variable_stack.len() >= 2 {
						let value = value_stack.pop().unwrap();
						let _var_placeholder = value_stack.pop().unwrap(); // Remove placeholder

						// Pop variable names (value operand first, then variable name)
						variable_stack.pop(); // Pop the variable name for the value
						let var_name = variable_stack.pop().unwrap(); // Pop the variable name for assignment target

						if !var_name.is_empty() {
							// Assign value to variable
							match VARIABLES.lock() {
								Ok(mut variables) => {
									variables.insert(var_name.clone(), value);
								}
								Err(poisoned) => {
									let mut variables = poisoned.into_inner();
									variables.insert(var_name.clone(), value);
								}
							}
							// Push the assigned value back for potential chaining
							value_stack.push(value);
							variable_stack.push(String::new()); // Push placeholder for result
						} else {
							return Err("Assignment requires a variable name".into());
						}
					} else {
						return Err("Assignment requires two operands".into());
					}
				}
				"+" => {
					if value_stack.len() >= 2 {
						let b = value_stack.pop().unwrap();
						let a = value_stack.pop().unwrap();
						let result = a + b;
						value_stack.push(result);
						// Clean up variable_stack for the two operands consumed and push placeholder for result
						if variable_stack.len() >= 2 {
							variable_stack.pop();
							variable_stack.pop();
							variable_stack.push(String::new()); // Placeholder for result
						}
					}
				}
				"-" => {
					if value_stack.len() >= 2 {
						let b = value_stack.pop().unwrap();
						let a = value_stack.pop().unwrap();
						let result = a - b;
						value_stack.push(result);
						// Clean up variable_stack for the two operands consumed and push placeholder for result
						if variable_stack.len() >= 2 {
							variable_stack.pop();
							variable_stack.pop();
							variable_stack.push(String::new()); // Placeholder for result
						}
					}
				}
				"*" => {
					if value_stack.len() >= 2 {
						let b = value_stack.pop().unwrap();
						let a = value_stack.pop().unwrap();
						let result = a * b;
						value_stack.push(result);
						// Clean up variable_stack for the two operands consumed and push placeholder for result
						if variable_stack.len() >= 2 {
							variable_stack.pop();
							variable_stack.pop();
							variable_stack.push(String::new()); // Placeholder for result
						}
					}
				}
				"/" => {
					if value_stack.len() >= 2 {
						let b = value_stack.pop().unwrap();
						let a = value_stack.pop().unwrap();
						if b != 0.0 {
							let result = a / b;
							value_stack.push(result);
						} else {
							return Err("Division by zero".into());
						}
						// Clean up variable_stack for the two operands consumed and push placeholder for result
						if variable_stack.len() >= 2 {
							variable_stack.pop();
							variable_stack.pop();
							variable_stack.push(String::new()); // Placeholder for result
						}
					}
				}
				"," => {
					// Commas should be handled in function call preprocessing,
					// but if they reach here, just ignore them
					continue;
				}
				_ => {
					println!("Warning: Operator '{}' not supported yet", op.value);
				}
			},
			Token::String(_) => {
				println!("Warning: Strings not supported in arithmetic evaluation");
			}
		}
	}

	// Return the final result if it's not an assignment
	if let Some(result) = value_stack.last() {
		if !tokens
			.iter()
			.any(|t| matches!(t, Token::Operator(op) if op.value == "="))
		{
			// println!("{}", result);
			Ok(Some(*result))
		} else {
			Ok(Some(*result))
		}
	} else {
		Ok(None)
	}
}

/// Preprocess tokens to handle unary minus by converting patterns like "- number" to "0 - number"
fn preprocess_unary_minus(tokens: &[Token]) -> Vec<Token> {
	let mut result = Vec::new();
	let mut i = 0;

	while i < tokens.len() {
		if let Token::Operator(op) = &tokens[i] {
			if op.value == "-" {
				// Check if this is a unary minus
				let is_unary = if i == 0 {
					// Minus at the beginning is unary
					true
				} else {
					// Check if previous token indicates this should be unary
					match &tokens[i - 1] {
						Token::Operator(prev_op) if prev_op.value == "(" => true,
						Token::Operator(prev_op) if prev_op.value == "," => true,
						Token::Operator(prev_op) if prev_op.value == "=" => true,
						Token::Operator(prev_op) if prev_op.value == "+" => true,
						Token::Operator(prev_op) if prev_op.value == "-" => true,
						Token::Operator(prev_op) if prev_op.value == "*" => true,
						Token::Operator(prev_op) if prev_op.value == "/" => true,
						_ => false,
					}
				};

				if is_unary {
					// Convert unary minus to "0 - number"
					result.push(Token::Number(lex::LangNumber::Integer(lex::LangInteger {
						value: 0,
					})));
					result.push(tokens[i].clone()); // The minus operator
				} else {
					// Regular binary minus
					result.push(tokens[i].clone());
				}
			} else {
				result.push(tokens[i].clone());
			}
		} else {
			result.push(tokens[i].clone());
		}
		i += 1;
	}

	result
}

fn eval_line(line: &LangLine) -> Option<f64> {
	// println!("Evaluating line:");

	// First preprocess tokens to handle function calls
	let processed_tokens = match preprocess_tokens_for_function_calls(&line.tokens) {
		Ok(tokens) => tokens,
		Err(e) => {
			println!("Error preprocessing function calls: {}", e);
			return None;
		}
	};

	// Preprocess tokens to handle unary minus
	let unary_processed_tokens = preprocess_unary_minus(&processed_tokens);

	// Debug output
	// println!("Original tokens: {:?}", line.tokens);
	// println!("Processed tokens: {:?}", processed_tokens);

	// Convert infix to postfix using Shunting Yard algorithm
	let postfix_tokens = infix_to_postfix(&unary_processed_tokens);

	// println!("Original tokens: {:?}", line.tokens);
	// println!("Postfix tokens: {:?}", postfix_tokens);

	match execute_postfix_tokens(&postfix_tokens) {
		Ok(result) => result,
		Err(e) => {
			println!("Error: {}", e);
			None
		}
	}
}

fn infix_to_postfix(tokens: &[Token]) -> Vec<Token> {
	let mut output: Vec<Token> = Vec::new();
	let mut operator_stack: Vec<Token> = Vec::new();

	for token in tokens {
		match token {
			Token::Number(_) | Token::Symbol(_) | Token::String(_) => {
				// Operands go directly to output
				output.push(token.clone());
			}
			Token::Operator(op) => {
				match op.value.as_str() {
					"=" => {
						// Assignment has lowest precedence, right associative
						while let Some(Token::Operator(stack_op)) = operator_stack.last() {
							if get_precedence(&stack_op.value) > get_precedence("=") {
								output.push(operator_stack.pop().unwrap());
							} else {
								break;
							}
						}
						operator_stack.push(token.clone());
					}
					"+" | "-" => {
						// Left associative, precedence 1
						while let Some(Token::Operator(stack_op)) = operator_stack.last() {
							if get_precedence(&stack_op.value) >= get_precedence(&op.value) {
								output.push(operator_stack.pop().unwrap());
							} else {
								break;
							}
						}
						operator_stack.push(token.clone());
					}
					"*" | "/" => {
						// Left associative, precedence 2
						while let Some(Token::Operator(stack_op)) = operator_stack.last() {
							if get_precedence(&stack_op.value) >= get_precedence(&op.value) {
								output.push(operator_stack.pop().unwrap());
							} else {
								break;
							}
						}
						operator_stack.push(token.clone());
					}
					"(" => {
						operator_stack.push(token.clone());
					}
					")" => {
						// Pop operators until we find the opening parenthesis
						while let Some(stack_token) = operator_stack.pop() {
							if let Token::Operator(stack_op) = &stack_token {
								if stack_op.value == "(" {
									break;
								}
							}
							output.push(stack_token);
						}
					}
					_ => {
						// For any other operators, treat as normal operators
						output.push(token.clone());
					}
				}
			}
		}
	}

	// Pop remaining operators from stack
	while let Some(op) = operator_stack.pop() {
		output.push(op);
	}

	output
}

fn get_precedence(op: &str) -> i32 {
	match op {
		"=" => 0,       // Assignment (lowest precedence)
		"+" | "-" => 1, // Addition and subtraction
		"*" | "/" => 2, // Multiplication and division (highest precedence)
		_ => -1,        // Unknown operators
	}
}

fn eval_block(block: &LangBlock) -> Option<f64> {
	// println!("Evaluating block:");

	let mut last_result = None;
	let mut has_function_definitions = false;

	for item in &block.items {
		match item {
			parse::LangBlockItem::Line(line) => {
				let result = eval_line(line);

				// Print result for non-assignment expressions
				if let Some(value) = result {
					// Check if this line contains an assignment operator
					let has_assignment = line
						.tokens
						.iter()
						.any(|t| matches!(t, Token::Operator(op) if op.value == "="));

					if !has_assignment {
						println!("{}", value);
					}
				}

				last_result = result;
			}
			parse::LangBlockItem::Block(nested_block) => {
				let result = eval_block(nested_block);
				last_result = result;
			}
			parse::LangBlockItem::Function(function) => {
				// Store the function definition and compile with LLVM
				let func_name = match FUNCTIONS.lock() {
					Ok(functions) => {
						format!("func_{}_{}", function.parameters.len(), functions.len())
					}
					Err(poisoned) => {
						let functions = poisoned.into_inner();
						format!("func_{}_{}", function.parameters.len(), functions.len())
					}
				};

				// Convert to named function for storage
				let named_function = parse::LangNamedFunction {
					name: func_name.clone(),
					parameters: function.parameters.clone(),
					body: function.body.clone(),
				};

				// Try to compile the function with LLVM, but store it regardless
				match compile_and_store_named_function(&named_function) {
					Ok(_) => {
						// Function was successfully compiled and stored
						match FUNCTIONS.lock() {
							Ok(mut functions) => {
								functions.insert(func_name.clone(), function.clone());
							}
							Err(poisoned) => {
								let mut functions = poisoned.into_inner();
								functions.insert(func_name.clone(), function.clone());
							}
						}
						// println!(
						// 	"Function defined: {} ({}) => {{ ... }}",
						// 	func_name,
						// 	function.parameters.join(", ")
						// );
					}
					Err(e) => {
						// Compilation failed, but still store function for runtime evaluation
						match FUNCTIONS.lock() {
							Ok(mut functions) => {
								functions.insert(func_name.clone(), function.clone());
							}
							Err(poisoned) => {
								let mut functions = poisoned.into_inner();
								functions.insert(func_name.clone(), function.clone());
							}
						}
						// println!("Error compiling function: {}", e);
						// println!("Function stored for runtime evaluation");
					}
				}
				has_function_definitions = true;
				last_result = None;
			}
			parse::LangBlockItem::NamedFunction(named_function) => {
				// Store the named function definition and compile with LLVM

				// Convert to LangFunction for storage compatibility
				let function = parse::LangFunction {
					parameters: named_function.parameters.clone(),
					body: named_function.body.clone(),
				};

				// Try to compile the function with LLVM, but store it regardless
				match compile_and_store_named_function(named_function) {
					Ok(_) => {
						// Function was successfully compiled and stored
						// Note: compile_and_store_named_function already stored it
						// println!(
						// 	"Function defined: {} ({}) => {{ ... }}",
						// 	named_function.name,
						// 	named_function.parameters.join(", ")
						// );
					}
					Err(e) => {
						// Compilation failed, but still store function for runtime evaluation
						match FUNCTIONS.lock() {
							Ok(mut functions) => {
								functions.insert(named_function.name.clone(), function);
							}
							Err(poisoned) => {
								let mut functions = poisoned.into_inner();
								functions.insert(named_function.name.clone(), function);
							}
						}
						// println!("Error compiling function: {}", e);
						// println!("Function stored for runtime evaluation");
					}
				}
				has_function_definitions = true;
				last_result = None;
			}
			parse::LangBlockItem::FunctionCall(call) => {
				// Execute function call using LLVM
				match execute_function_call(call) {
					Ok(result) => {
						println!("{}", result);
						last_result = Some(result);
					}
					Err(e) => {
						println!("Error calling function: {}", e);
						last_result = None;
					}
				}
			}
		}
	}

	// If there were function definitions in this block, return None
	if has_function_definitions {
		None
	} else {
		last_result
	}
}

fn run(line: &str) -> Option<f64> {
	// println!("Tokenizing: {}", line);
	let tokens = lex(line);

	// Parse tokens into a LangBlock with support for nested blocks
	let mut token_iter = tokens.into_iter().peekable();
	let block = parse_block(&mut token_iter);

	// println!("Parsed block:\n{}", block);

	eval_block(&block)
}

/// Create an executable binary from a user-defined function
fn create_executable_from_function(
	function_name: &str,
	output_name: &str,
	args: &[f64],
) -> Result<(), Box<dyn Error>> {
	// Get the function from storage
	let function_opt = match FUNCTIONS.lock() {
		Ok(functions) => functions.get(function_name).cloned(),
		Err(poisoned) => {
			let functions = poisoned.into_inner();
			functions.get(function_name).cloned()
		}
	};

	let function = function_opt.ok_or(format!("Function '{}' not found", function_name))?;

	// Create LLVM context and code generator for binary generation
	let context = Context::create();
	let mut codegen = LLVMCodeGen::new_for_binary_gen(&context)?;

	// Compile the user function to LLVM IR
	codegen.compile_function(function_name, &function)?;

	// Create a main function that calls the user function
	codegen.create_main_function(function_name, args)?;

	// Generate the executable
	codegen.generate_executable(output_name)?;

	Ok(())
}

/// Create a simple executable that evaluates an expression
fn create_executable_from_expression(
	expression: &str,
	output_name: &str,
) -> Result<(), Box<dyn Error>> {
	// Parse the expression
	let tokens = lex(expression);
	let mut token_iter = tokens.into_iter().peekable();
	let block = parse_block(&mut token_iter);

	// Create LLVM context and code generator
	let context = Context::create();
	let mut codegen = LLVMCodeGen::new_for_binary_gen(&context)?;

	// Create main function that evaluates the expression and returns the result
	let i32_type = context.i32_type();
	let main_fn_type = i32_type.fn_type(&[], false);
	let main_function = codegen.module.add_function("main", main_fn_type, None);

	let basic_block = context.append_basic_block(main_function, "entry");
	codegen.builder.position_at_end(basic_block);

	// Try to compile the expression
	let empty_vars = HashMap::new();
	match codegen.compile_block(&block, &empty_vars) {
		Ok(_result) => {
			// Expression compiled successfully
			let return_val = i32_type.const_int(0, false);
			codegen.builder.build_return(Some(&return_val)).unwrap();
		}
		Err(_) => {
			// Expression too complex, return error
			let return_val = i32_type.const_int(1, false);
			codegen.builder.build_return(Some(&return_val)).unwrap();
		}
	}

	// Generate the executable
	codegen.generate_executable(output_name)?;

	Ok(())
}

fn main() {
	println!("Fast Calculator");
	println!("===============");
	println!("Features:");
	println!("  • Basic arithmetic: 2 + 3 * 4");
	println!("  • Variables: x = 5; y = x * 2");
	println!("  • Functions: fn increment(x) {{ x + 1 }}");
	println!("  • Function calls: increment(5)");
	println!("  • Binary generation: :compile <function_name> <output_name> [args...]");
	println!("  • Expression compilation: :compile_expr <expression> <output_name>");
	println!("");

	let _ = repl();
}

#[allow(dead_code)]
fn repl() -> rustyline::Result<()> {
	let mut rl = rustyline::DefaultEditor::new()?;
	let _ = rl.load_history("repl_history.txt").is_err();
	loop {
		let readline = rl.readline(">> ");
		match readline {
			Ok(line) => {
				let _ = rl.add_history_entry(line.as_str());

				// Check for special commands
				if line.starts_with(":compile_expr ") {
					// Parse command: :compile_expr <expression> <output_name>
					let parts: Vec<&str> = line[14..].splitn(2, ' ').collect();
					if parts.len() == 2 {
						let expression = parts[0];
						let output_name = parts[1];
						match create_executable_from_expression(expression, output_name) {
							Ok(_) => println!("✓ Executable created successfully"),
							Err(e) => println!("✗ Error creating executable: {}", e),
						}
					} else {
						println!("Usage: :compile_expr <expression> <output_name>");
					}
				} else if line.starts_with(":compile ") {
					// Parse command: :compile <function_name> <output_name> [args...]
					let parts: Vec<&str> = line[9..].split_whitespace().collect();
					if parts.len() >= 2 {
						let function_name = parts[0];
						let output_name = parts[1];
						let args: Result<Vec<f64>, _> =
							parts[2..].iter().map(|s| s.parse()).collect();

						match args {
							Ok(arg_values) => {
								match create_executable_from_function(
									function_name,
									output_name,
									&arg_values,
								) {
									Ok(_) => println!("✓ Executable created successfully"),
									Err(e) => println!("✗ Error creating executable: {}", e),
								}
							}
							Err(_) => {
								println!(
									"Error: Invalid argument values. All arguments must be numbers."
								);
							}
						}
					} else {
						println!("Usage: :compile <function_name> <output_name> [args...]");
					}
				} else if line.starts_with(":help") {
					println!("Available commands:");
					println!(
						"  :compile <function_name> <output_name> [args...]  - Compile function to executable"
					);
					println!(
						"  :compile_expr <expression> <output_name>         - Compile expression to executable"
					);
					println!("  :help                                            - Show this help");
					println!("  :quit                                            - Exit the REPL");
				} else if line.starts_with(":quit") {
					break;
				} else {
					// Regular expression evaluation
					let _result = run(line.as_str());
				}
			}
			Err(_) => {
				break;
			}
		}
	}
	let _ = rl.save_history("repl_history.txt");
	Ok(())
}

#[cfg(test)]
mod tests;
