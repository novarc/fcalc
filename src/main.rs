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

use std::collections::HashMap;
use std::error::Error;
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
		let execution_engine = module.create_jit_execution_engine(OptimizationLevel::None)?;
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
					last_result = self.compile_function_call(call, variables)?;
				}
			}
		}

		Ok(last_result)
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
						// Use global variable or default to 0
						let value = self.float_type.const_float(0.0);
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
							let result = self.builder.build_float_div(a, b, "div").unwrap();
							value_stack.push(result);
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

/// Compile and store a named function using LLVM
fn compile_and_store_named_function(
	named_function: &parse::LangNamedFunction,
) -> Result<(), Box<dyn Error>> {
	// Create a new LLVM context for this compilation
	let context = Context::create();
	let mut codegen = LLVMCodeGen::new(&context)?;

	// Compile the named function
	let _llvm_function = codegen.compile_named_function(named_function)?;

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
			let postfix = infix_to_postfix(arg_tokens);
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

		// Create a new LLVM context and compile the function for execution
		let context = Context::create();
		let mut codegen = LLVMCodeGen::new(&context)?;

		// Compile the function
		let _llvm_function = codegen.compile_function(&call.name, &function)?;

		// Get JIT function pointer and execute based on argument count
		unsafe {
			match arg_values.len() {
				0 => {
					type Func0 = unsafe extern "C" fn() -> f64;
					let jit_fn: inkwell::execution_engine::JitFunction<Func0> =
						codegen.execution_engine.get_function(&call.name)?;
					Ok(jit_fn.call())
				}
				1 => {
					type Func1 = unsafe extern "C" fn(f64) -> f64;
					let jit_fn: inkwell::execution_engine::JitFunction<Func1> =
						codegen.execution_engine.get_function(&call.name)?;
					Ok(jit_fn.call(arg_values[0]))
				}
				2 => {
					type Func2 = unsafe extern "C" fn(f64, f64) -> f64;
					let jit_fn: inkwell::execution_engine::JitFunction<Func2> =
						codegen.execution_engine.get_function(&call.name)?;
					Ok(jit_fn.call(arg_values[0], arg_values[1]))
				}
				3 => {
					type Func3 = unsafe extern "C" fn(f64, f64, f64) -> f64;
					let jit_fn: inkwell::execution_engine::JitFunction<Func3> =
						codegen.execution_engine.get_function(&call.name)?;
					Ok(jit_fn.call(arg_values[0], arg_values[1], arg_values[2]))
				}
				4 => {
					type Func4 = unsafe extern "C" fn(f64, f64, f64, f64) -> f64;
					let jit_fn: inkwell::execution_engine::JitFunction<Func4> =
						codegen.execution_engine.get_function(&call.name)?;
					Ok(jit_fn.call(arg_values[0], arg_values[1], arg_values[2], arg_values[3]))
				}
				5 => {
					type Func5 = unsafe extern "C" fn(f64, f64, f64, f64, f64) -> f64;
					let jit_fn: inkwell::execution_engine::JitFunction<Func5> =
						codegen.execution_engine.get_function(&call.name)?;
					Ok(jit_fn.call(
						arg_values[0],
						arg_values[1],
						arg_values[2],
						arg_values[3],
						arg_values[4],
					))
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

fn eval_line(line: &LangLine) -> Option<f64> {
	// println!("Evaluating line:");

	// Convert infix to postfix using Shunting Yard algorithm
	let postfix_tokens = infix_to_postfix(&line.tokens);

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

	for item in &block.items {
		match item {
			parse::LangBlockItem::Line(line) => {
				let result = eval_line(line);
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

				// Compile the function with LLVM
				match compile_and_store_named_function(&named_function) {
					Ok(_) => {
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
						println!("Error compiling function: {}", e);
					}
				}
				last_result = None;
			}
			parse::LangBlockItem::NamedFunction(named_function) => {
				// Store the named function definition and compile with LLVM

				// Convert to LangFunction for storage compatibility
				let function = parse::LangFunction {
					parameters: named_function.parameters.clone(),
					body: named_function.body.clone(),
				};

				// Compile the function with LLVM
				match compile_and_store_named_function(named_function) {
					Ok(_) => {
						match FUNCTIONS.lock() {
							Ok(mut functions) => {
								functions.insert(named_function.name.clone(), function);
							}
							Err(poisoned) => {
								let mut functions = poisoned.into_inner();
								functions.insert(named_function.name.clone(), function);
							}
						}
						// println!(
						// 	"Function defined: {} ({}) => {{ ... }}",
						// 	named_function.name,
						// 	named_function.parameters.join(", ")
						// );
					}
					Err(e) => {
						println!("Error compiling function: {}", e);
					}
				}
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

	last_result
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

fn main() {
	println!("Fast Calculator");
	println!("===============");
	println!("Features:");
	println!("  • Basic arithmetic: 2 + 3 * 4");
	println!("  • Variables: x = 5; y = x * 2");
	println!("  • Functions: (x) => {{ x + 1 }}");
	println!("  • Named functions: sum = (a, b) => {{ a + b }}");
	println!("  • Function calls: sum(5, 3)");
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
				let _result = run(line.as_str());
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
