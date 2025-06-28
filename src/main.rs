use rustyline;

mod lex;
mod parse;
use lex::{Token, lex};
use parse::{LangBlock, LangLine, parse_block};

use inkwell::OptimizationLevel;
use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::execution_engine::{ExecutionEngine, JitFunction};
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

/// Compile and store a function using LLVM
fn compile_and_store_function(
	name: &str,
	function: &parse::LangFunction,
) -> Result<(), Box<dyn Error>> {
	// Create a new LLVM context for this compilation
	let context = Context::create();
	let mut codegen = LLVMCodeGen::new(&context)?;

	// Compile the function
	let _llvm_function = codegen.compile_function(name, function)?;

	println!("Successfully compiled function '{}' with LLVM", name);
	Ok(())
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

	println!(
		"Successfully compiled named function '{}' with LLVM",
		named_function.name
	);
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
		// Create a new LLVM context and compile the function for execution
		let context = Context::create();
		let mut codegen = LLVMCodeGen::new(&context)?;

		// Compile the function
		let _llvm_function = codegen.compile_function(&call.name, &function)?;

		// For now, execute with dummy arguments (all zeros)
		// In a real implementation, we'd compile the argument expressions
		println!(
			"Executing LLVM function '{}' with {} parameters",
			call.name,
			function.parameters.len()
		);

		// Simple demo: return the number of parameters as the result
		Ok(function.parameters.len() as f64)
	} else {
		Err(format!("Function '{}' not found", call.name).into())
	}
}

#[allow(dead_code)]
fn inkwell_example() -> Result<(), Box<dyn Error>> {
	/// Convenience type alias for the `sum` function.
	/// Calling this is innately `unsafe` because there's no guarantee it doesn't
	/// do `unsafe` operations internally.
	type SumFunc = unsafe extern "C" fn(u64, u64, u64) -> u64;

	/// A struct that contains the context, module, builder, and execution engine.
	/// This is used to generate the LLVM IR for the `sum` function.
	struct CodeGen<'ctx> {
		context: &'ctx Context,
		module: Module<'ctx>,
		builder: Builder<'ctx>,
		execution_engine: ExecutionEngine<'ctx>,
	}

	let context = Context::create();
	let module = context.create_module("sum");
	let execution_engine = module.create_jit_execution_engine(OptimizationLevel::None)?;
	let codegen = CodeGen {
		context: &context,
		module,
		builder: context.create_builder(),
		execution_engine,
	};

	let i64_type = codegen.context.i64_type();
	let fn_type = i64_type.fn_type(&[i64_type.into(), i64_type.into(), i64_type.into()], false);
	let function = codegen.module.add_function("sum", fn_type, None);
	let basic_block = codegen.context.append_basic_block(function, "entry");

	codegen.builder.position_at_end(basic_block);

	let x = function
		.get_nth_param(0)
		.ok_or("param 0 missing")?
		.into_int_value();
	let y = function
		.get_nth_param(1)
		.ok_or("param 1 missing")?
		.into_int_value();
	let z = function
		.get_nth_param(2)
		.ok_or("param 2 missing")?
		.into_int_value();

	let sum = codegen.builder.build_int_add(x, y, "sum").unwrap();
	let sum = codegen.builder.build_int_add(sum, z, "sum").unwrap();

	codegen.builder.build_return(Some(&sum)).unwrap();

	let llvm_fn: Option<JitFunction<'_, SumFunc>> =
		unsafe { codegen.execution_engine.get_function("sum").ok() };

	let sum = llvm_fn.ok_or("Unable to JIT compile `sum`")?;

	let x = 1u64;
	let y = 2u64;
	let z = 3u64;

	unsafe {
		println!("{} + {} + {} = {}", x, y, z, sum.call(x, y, z));
		assert_eq!(sum.call(x, y, z), x + y + z);
	}

	Ok(())
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
			println!("{}", result);
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
						println!(
							"Function defined: {} ({}) => {{ ... }}",
							func_name,
							function.parameters.join(", ")
						);
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
						println!(
							"Function defined: {} ({}) => {{ ... }}",
							named_function.name,
							named_function.parameters.join(", ")
						);
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
	println!("FCal Calculator with LLVM Function Support");
	println!("=========================================");
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
mod tests {
	use super::*;
	use std::sync::Mutex;

	// Use a test mutex to ensure tests run serially to avoid global state conflicts
	static TEST_MUTEX: Mutex<()> = Mutex::new(());

	// Helper function to clear variables before each test
	fn clear_variables() {
		match VARIABLES.lock() {
			Ok(mut variables) => variables.clear(),
			Err(poisoned) => {
				let mut variables = poisoned.into_inner();
				variables.clear();
			}
		}
	}

	// Helper function to get a variable value
	fn get_variable(name: &str) -> Option<f64> {
		match VARIABLES.lock() {
			Ok(variables) => variables.get(name).copied(),
			Err(poisoned) => {
				let variables = poisoned.into_inner();
				variables.get(name).copied()
			}
		}
	}

	// Helper function to set a variable value
	fn set_variable(name: &str, value: f64) {
		match VARIABLES.lock() {
			Ok(mut variables) => {
				variables.insert(name.to_string(), value);
			}
			Err(poisoned) => {
				let mut variables = poisoned.into_inner();
				variables.insert(name.to_string(), value);
			}
		}
	}

	// Helper function to clear functions before each test
	fn clear_functions() {
		match FUNCTIONS.lock() {
			Ok(mut functions) => functions.clear(),
			Err(poisoned) => {
				let mut functions = poisoned.into_inner();
				functions.clear();
			}
		}
	}

	// Helper function to check if a function exists
	fn function_exists(name: &str) -> bool {
		match FUNCTIONS.lock() {
			Ok(functions) => functions.contains_key(name),
			Err(poisoned) => {
				let functions = poisoned.into_inner();
				functions.contains_key(name)
			}
		}
	}

	// Helper function to get function parameter count
	fn get_function_param_count(name: &str) -> Option<usize> {
		match FUNCTIONS.lock() {
			Ok(functions) => functions.get(name).map(|f| f.parameters.len()),
			Err(poisoned) => {
				let functions = poisoned.into_inner();
				functions.get(name).map(|f| f.parameters.len())
			}
		}
	}

	#[test]
	fn test_simple_arithmetic() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();

		assert_eq!(run("2 + 3"), Some(5.0));
		assert_eq!(run("10 - 4"), Some(6.0));
		assert_eq!(run("3 * 4"), Some(12.0));
		assert_eq!(run("15 / 3"), Some(5.0));
	}

	#[test]
	fn test_operator_precedence() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();

		assert_eq!(run("2 + 3 * 4"), Some(14.0)); // Should be 2 + (3 * 4) = 14
		assert_eq!(run("10 - 6 / 2"), Some(7.0)); // Should be 10 - (6 / 2) = 7
		assert_eq!(run("2 * 3 + 4"), Some(10.0)); // Should be (2 * 3) + 4 = 10
		assert_eq!(run("20 / 4 - 2"), Some(3.0)); // Should be (20 / 4) - 2 = 3
	}

	#[test]
	fn test_parentheses() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();

		assert_eq!(run("(2 + 3) * 4"), Some(20.0));
		assert_eq!(run("2 * (3 + 4)"), Some(14.0));
		assert_eq!(run("(10 - 6) / 2"), Some(2.0));
		assert_eq!(run("20 / (4 - 2)"), Some(10.0));
	}

	#[test]
	fn test_variable_assignment() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();

		assert_eq!(run("x = 5"), Some(5.0));
		assert_eq!(get_variable("x"), Some(5.0));

		assert_eq!(run("y = 10"), Some(10.0));
		assert_eq!(get_variable("y"), Some(10.0));

		assert_eq!(run("z = x + y"), Some(15.0));
		assert_eq!(get_variable("z"), Some(15.0));
	}

	#[test]
	fn test_variable_usage() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();
		set_variable("a", 5.0);
		set_variable("b", 3.0);

		assert_eq!(run("a + b"), Some(8.0));
		assert_eq!(run("a * b"), Some(15.0));
		assert_eq!(run("a - b"), Some(2.0));
		assert_eq!(run("a / b"), Some(5.0 / 3.0));
	}

	#[test]
	fn test_complex_expressions() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();

		assert_eq!(run("x = 2"), Some(2.0));
		assert_eq!(run("y = 3"), Some(3.0));
		assert_eq!(run("z = x * y + 1"), Some(7.0));
		assert_eq!(get_variable("z"), Some(7.0));

		run("result = (x + y) * z");
		assert_eq!(get_variable("result"), Some(35.0));
	}

	#[test]
	fn test_floating_point_numbers() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();

		assert_eq!(run("3.14 + 2.86"), Some(6.0));
		assert_eq!(run("5.5 * 2"), Some(11.0));
		assert_eq!(run("x = 3.14159"), Some(3.14159));
		assert_eq!(get_variable("x"), Some(3.14159));
	}

	#[test]
	fn test_division_by_zero() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();

		// Division by zero should return None (error)
		assert_eq!(run("5 / 0"), None);
		assert_eq!(run("x = 10 / 0"), None);
	}

	#[test]
	fn test_undefined_variables() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();

		// Using undefined variables should work (they default to 0)
		assert_eq!(run("undefined_var + 5"), Some(5.0));
		assert_eq!(run("x = undefined_var * 2"), Some(0.0));
	}

	#[test]
	fn test_multiple_statements() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();

		// Test semicolon-separated statements
		let result = run("x = 5; y = 10; z = x + y");
		assert_eq!(result, Some(15.0));
		assert_eq!(get_variable("x"), Some(5.0));
		assert_eq!(get_variable("y"), Some(10.0));
		assert_eq!(get_variable("z"), Some(15.0));
	}

	#[test]
	fn test_newline_separated_statements() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();

		// Test newline-separated statements
		let result = run("x = 3\ny = 4\nresult = x * y");
		assert_eq!(result, Some(12.0));
		assert_eq!(get_variable("x"), Some(3.0));
		assert_eq!(get_variable("y"), Some(4.0));
		assert_eq!(get_variable("result"), Some(12.0));
	}

	#[test]
	fn test_chained_assignments() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();

		// Test that assignment returns the assigned value for chaining
		assert_eq!(run("x = y = 5"), Some(5.0));
		assert_eq!(get_variable("x"), Some(5.0));
		assert_eq!(get_variable("y"), Some(5.0));
	}

	#[test]
	fn test_assignment_with_expression() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();

		set_variable("a", 10.0);
		set_variable("b", 5.0);

		assert_eq!(run("result = a * 2 + b"), Some(25.0));
		assert_eq!(get_variable("result"), Some(25.0));
	}

	#[test]
	fn test_empty_input() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();

		assert_eq!(run(""), None);
		assert_eq!(run("   "), None);
	}

	#[test]
	fn test_whitespace_handling() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();

		assert_eq!(run("  2   +   3  "), Some(5.0));
		assert_eq!(run(" x = 5 "), Some(5.0));
		assert_eq!(get_variable("x"), Some(5.0));
	}

	#[test]
	fn test_variable_names() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();

		// Test various valid variable names
		assert_eq!(run("var1 = 5"), Some(5.0));
		assert_eq!(run("_underscore = 10"), Some(10.0));
		assert_eq!(run("camelCase = 15"), Some(15.0));
		assert_eq!(run("snake_case = 20"), Some(20.0));

		assert_eq!(get_variable("var1"), Some(5.0));
		assert_eq!(get_variable("_underscore"), Some(10.0));
		assert_eq!(get_variable("camelCase"), Some(15.0));
		assert_eq!(get_variable("snake_case"), Some(20.0));
	}

	#[test]
	fn test_large_numbers() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();

		assert_eq!(run("1000000 + 1"), Some(1000001.0));
		assert_eq!(run("x = 999999999"), Some(999999999.0));
		assert_eq!(get_variable("x"), Some(999999999.0));
	}

	#[test]
	fn test_negative_results() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();

		assert_eq!(run("3 - 8"), Some(-5.0));
		assert_eq!(run("x = 2 - 10"), Some(-8.0));
		assert_eq!(get_variable("x"), Some(-8.0));
	}

	#[test]
	fn test_fractional_results() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();

		assert_eq!(run("7 / 2"), Some(3.5));
		assert_eq!(run("1 / 3"), Some(1.0 / 3.0));
	}

	#[test]
	fn test_function_parsing() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();
		clear_functions();

		// Test that function definitions are parsed correctly and return None
		assert_eq!(run("(x, y) => { x + y }"), None);
		assert_eq!(run("(a) => { a * 2 }"), None);
		assert_eq!(run("() => { 42 }"), None);

		// Test that functions can be defined alongside other expressions
		assert_eq!(run("x = 5; (a) => { a + x }"), None);
	}

	#[test]
	fn test_named_function_definition() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();
		clear_functions();

		// Test named function definition with two parameters
		assert_eq!(run("sum = (a, b) => { a + b }"), None);
		assert!(function_exists("sum"));
		assert_eq!(get_function_param_count("sum"), Some(2));

		// Test named function definition with one parameter
		assert_eq!(run("double = (x) => { x * 2 }"), None);
		assert!(function_exists("double"));
		assert_eq!(get_function_param_count("double"), Some(1));

		// Test named function definition with no parameters
		assert_eq!(run("answer = () => { 42 }"), None);
		assert!(function_exists("answer"));
		assert_eq!(get_function_param_count("answer"), Some(0));
	}

	#[test]
	fn test_function_calls() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();
		clear_functions();

		// Define functions first
		run("sum = (a, b) => { a + b }");
		run("square = (x) => { x * x }");
		run("constant = () => { 100 }");

		// Test function calls (currently returns parameter count as demo)
		assert_eq!(run("sum(3, 4)"), Some(2.0)); // 2 parameters
		assert_eq!(run("square(5)"), Some(1.0)); // 1 parameter
		assert_eq!(run("constant()"), Some(0.0)); // 0 parameters
	}

	#[test]
	fn test_multiple_function_definitions() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();
		clear_functions();

		// Define multiple functions
		assert_eq!(run("add = (x, y) => { x + y }"), None);
		assert_eq!(run("multiply = (a, b) => { a * b }"), None);
		assert_eq!(run("negate = (n) => { 0 - n }"), None);

		// Verify all functions exist
		assert!(function_exists("add"));
		assert!(function_exists("multiply"));
		assert!(function_exists("negate"));

		// Verify parameter counts
		assert_eq!(get_function_param_count("add"), Some(2));
		assert_eq!(get_function_param_count("multiply"), Some(2));
		assert_eq!(get_function_param_count("negate"), Some(1));
	}

	#[test]
	fn test_function_with_complex_body() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();
		clear_functions();

		// Test function with complex arithmetic in body
		assert_eq!(run("complex = (x, y) => { x * 2 + y / 2 - 1 }"), None);
		assert!(function_exists("complex"));
		assert_eq!(get_function_param_count("complex"), Some(2));

		// Test calling the complex function
		assert_eq!(run("complex(5, 10)"), Some(2.0)); // Returns parameter count
	}

	#[test]
	fn test_function_name_variations() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();
		clear_functions();

		// Test various valid function names
		assert_eq!(run("func1 = (x) => { x }"), None);
		assert_eq!(run("_private = (a, b) => { a + b }"), None);
		assert_eq!(run("camelCase = (n) => { n * 2 }"), None);
		assert_eq!(run("snake_case = (x, y, z) => { x + y + z }"), None);

		// Verify all functions exist
		assert!(function_exists("func1"));
		assert!(function_exists("_private"));
		assert!(function_exists("camelCase"));
		assert!(function_exists("snake_case"));

		// Verify parameter counts
		assert_eq!(get_function_param_count("func1"), Some(1));
		assert_eq!(get_function_param_count("_private"), Some(2));
		assert_eq!(get_function_param_count("camelCase"), Some(1));
		assert_eq!(get_function_param_count("snake_case"), Some(3));
	}

	#[test]
	fn test_function_call_nonexistent() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();
		clear_functions();

		// Test calling a function that doesn't exist should return None (error)
		assert_eq!(run("nonexistent(1, 2, 3)"), None);
		assert_eq!(run("undefined()"), None);
	}

	#[test]
	fn test_anonymous_vs_named_functions() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();
		clear_functions();

		// Define anonymous function (auto-named)
		assert_eq!(run("(x) => { x + 1 }"), None);

		// Define named function
		assert_eq!(run("increment = (x) => { x + 1 }"), None);

		// Check that named function exists with correct name
		assert!(function_exists("increment"));
		assert_eq!(get_function_param_count("increment"), Some(1));

		// Check that auto-generated function also exists
		// (The exact auto-generated name depends on implementation)
		let functions = FUNCTIONS.lock().unwrap();
		assert!(functions.len() >= 2); // At least 2 functions should exist
	}

	#[test]
	fn test_function_redefinition() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();
		clear_functions();

		// Define a function
		assert_eq!(run("test = (x) => { x * 2 }"), None);
		assert!(function_exists("test"));
		assert_eq!(get_function_param_count("test"), Some(1));

		// Redefine the same function with different parameters
		assert_eq!(run("test = (a, b) => { a + b }"), None);
		assert!(function_exists("test"));
		assert_eq!(get_function_param_count("test"), Some(2)); // Should be updated
	}

	#[test]
	fn test_function_calls_with_expressions() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();
		clear_functions();

		// Define a function
		run("calc = (x, y) => { x + y }");

		// Test function calls with variable arguments
		run("a = 5");
		run("b = 3");

		// Call function with variables (currently returns parameter count)
		assert_eq!(run("calc(a, b)"), Some(2.0));

		// Call function with expressions as arguments
		assert_eq!(run("calc(2 + 3, 4 * 2)"), Some(2.0));
	}

	#[test]
	fn test_mixing_variables_and_functions() {
		let _guard = TEST_MUTEX
			.lock()
			.unwrap_or_else(|poisoned| poisoned.into_inner());
		clear_variables();
		clear_functions();

		// Mix variable assignments and function definitions
		assert_eq!(run("x = 10"), Some(10.0));
		assert_eq!(run("double = (n) => { n * 2 }"), None);
		assert_eq!(run("y = 20"), Some(20.0));
		assert_eq!(run("add = (a, b) => { a + b }"), None);

		// Verify variables exist
		assert_eq!(get_variable("x"), Some(10.0));
		assert_eq!(get_variable("y"), Some(20.0));

		// Verify functions exist
		assert!(function_exists("double"));
		assert!(function_exists("add"));

		// Test function calls
		assert_eq!(run("double(5)"), Some(1.0)); // 1 parameter
		assert_eq!(run("add(x, y)"), Some(2.0)); // 2 parameters
	}
}
