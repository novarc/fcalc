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

// Global variable storage for the REPL session
static VARIABLES: LazyLock<Mutex<HashMap<String, f64>>> =
	LazyLock::new(|| Mutex::new(HashMap::new()));

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
				let variables = VARIABLES.lock().unwrap();
				if let Some(&value) = variables.get(&symbol.value) {
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
							let mut variables = VARIABLES.lock().unwrap();
							variables.insert(var_name.clone(), value);
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
	// println!("\n--- Simple arithmetic ---");
	// run("2 + 3 * 4");

	// println!("\n--- Assignment with arithmetic ---");
	// run("x = a + b * c");

	// println!("\n--- More complex expression ---");
	// run("a + b * c - d / e");

	// println!("\n--- Original complex test ---");
	// run("2 + 4 /! 5 - 3 + \"hello\" /* yea */ \n 123");

	// println!("\n--- Testing with blocks ---");
	// run("if x > 0 { \n  y = x + 1; \n  z = y * 2 \n} else { \n  y = 0 \n}");

	// let _ = inkwell_example();

	// run("x = 10 ; y = 20 ; z = x + y ; z * 7");

	// run("x = 10 ; y = 20 ; z = x + y ; z * 7");

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
		let mut variables = VARIABLES.lock().unwrap();
		variables.clear();
	}

	// Helper function to get a variable value
	fn get_variable(name: &str) -> Option<f64> {
		let variables = VARIABLES.lock().unwrap();
		variables.get(name).copied()
	}

	// Helper function to set a variable value
	fn set_variable(name: &str, value: f64) {
		let mut variables = VARIABLES.lock().unwrap();
		variables.insert(name.to_string(), value);
	}

	#[test]
	fn test_simple_arithmetic() {
		let _guard = TEST_MUTEX.lock().unwrap();
		clear_variables();

		assert_eq!(run("2 + 3"), Some(5.0));
		assert_eq!(run("10 - 4"), Some(6.0));
		assert_eq!(run("3 * 4"), Some(12.0));
		assert_eq!(run("15 / 3"), Some(5.0));
	}

	#[test]
	fn test_operator_precedence() {
		let _guard = TEST_MUTEX.lock().unwrap();
		clear_variables();

		assert_eq!(run("2 + 3 * 4"), Some(14.0)); // Should be 2 + (3 * 4) = 14
		assert_eq!(run("10 - 6 / 2"), Some(7.0)); // Should be 10 - (6 / 2) = 7
		assert_eq!(run("2 * 3 + 4"), Some(10.0)); // Should be (2 * 3) + 4 = 10
		assert_eq!(run("20 / 4 - 2"), Some(3.0)); // Should be (20 / 4) - 2 = 3
	}

	#[test]
	fn test_parentheses() {
		let _guard = TEST_MUTEX.lock().unwrap();
		clear_variables();

		assert_eq!(run("(2 + 3) * 4"), Some(20.0));
		assert_eq!(run("2 * (3 + 4)"), Some(14.0));
		assert_eq!(run("(10 - 6) / 2"), Some(2.0));
		assert_eq!(run("20 / (4 - 2)"), Some(10.0));
	}

	#[test]
	fn test_variable_assignment() {
		let _guard = TEST_MUTEX.lock().unwrap();
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
		let _guard = TEST_MUTEX.lock().unwrap();
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
		let _guard = TEST_MUTEX.lock().unwrap();
		clear_variables();

		assert_eq!(run("x = 2"), Some(2.0));
		assert_eq!(run("y = 3"), Some(3.0));
		assert_eq!(run("z = x * y + 1"), Some(7.0));
		assert_eq!(get_variable("z"), Some(7.0));

		assert_eq!(run("result = (x + y) * z"), Some(35.0));
		assert_eq!(get_variable("result"), Some(35.0));
	}

	#[test]
	fn test_floating_point_numbers() {
		let _guard = TEST_MUTEX.lock().unwrap();
		clear_variables();

		assert_eq!(run("3.14 + 2.86"), Some(6.0));
		assert_eq!(run("5.5 * 2"), Some(11.0));
		assert_eq!(run("x = 3.14159"), Some(3.14159));
		assert_eq!(get_variable("x"), Some(3.14159));
	}

	#[test]
	fn test_division_by_zero() {
		let _guard = TEST_MUTEX.lock().unwrap();
		clear_variables();

		// Division by zero should return None (error)
		assert_eq!(run("5 / 0"), None);
		assert_eq!(run("x = 10 / 0"), None);
	}

	#[test]
	fn test_undefined_variables() {
		let _guard = TEST_MUTEX.lock().unwrap();
		clear_variables();

		// Using undefined variables should work (they default to 0)
		assert_eq!(run("undefined_var + 5"), Some(5.0));
		assert_eq!(run("x = undefined_var * 2"), Some(0.0));
	}

	#[test]
	fn test_multiple_statements() {
		let _guard = TEST_MUTEX.lock().unwrap();
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
		let _guard = TEST_MUTEX.lock().unwrap();
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
		let _guard = TEST_MUTEX.lock().unwrap();
		clear_variables();

		// Test that assignment returns the assigned value for chaining
		assert_eq!(run("x = y = 5"), Some(5.0));
		assert_eq!(get_variable("x"), Some(5.0));
		assert_eq!(get_variable("y"), Some(5.0));
	}

	#[test]
	fn test_assignment_with_expression() {
		let _guard = TEST_MUTEX.lock().unwrap();
		clear_variables();

		set_variable("a", 10.0);
		set_variable("b", 5.0);

		assert_eq!(run("result = a * 2 + b"), Some(25.0));
		assert_eq!(get_variable("result"), Some(25.0));
	}

	#[test]
	fn test_empty_input() {
		let _guard = TEST_MUTEX.lock().unwrap();
		clear_variables();

		assert_eq!(run(""), None);
		assert_eq!(run("   "), None);
	}

	#[test]
	fn test_whitespace_handling() {
		let _guard = TEST_MUTEX.lock().unwrap();
		clear_variables();

		assert_eq!(run("  2   +   3  "), Some(5.0));
		assert_eq!(run(" x = 5 "), Some(5.0));
		assert_eq!(get_variable("x"), Some(5.0));
	}

	#[test]
	fn test_variable_names() {
		let _guard = TEST_MUTEX.lock().unwrap();
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
		let _guard = TEST_MUTEX.lock().unwrap();
		clear_variables();

		assert_eq!(run("1000000 + 1"), Some(1000001.0));
		assert_eq!(run("x = 999999999"), Some(999999999.0));
		assert_eq!(get_variable("x"), Some(999999999.0));
	}

	#[test]
	fn test_negative_results() {
		let _guard = TEST_MUTEX.lock().unwrap();
		clear_variables();

		assert_eq!(run("3 - 8"), Some(-5.0));
		assert_eq!(run("x = 2 - 10"), Some(-8.0));
		assert_eq!(get_variable("x"), Some(-8.0));
	}

	#[test]
	fn test_fractional_results() {
		let _guard = TEST_MUTEX.lock().unwrap();
		clear_variables();

		assert_eq!(run("7 / 2"), Some(3.5));
		assert_eq!(run("1 / 3"), Some(1.0 / 3.0));
	}
}
