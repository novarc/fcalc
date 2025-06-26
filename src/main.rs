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

use std::error::Error;

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

fn execute_postfix_tokens(tokens: &[Token]) -> Result<(), Box<dyn Error>> {
	let context = Context::create();
	let module = context.create_module("postfix_eval");
	let execution_engine = module.create_jit_execution_engine(OptimizationLevel::None)?;
	let builder = context.create_builder();

	// Create function type: () -> i64
	let f64_type = context.f64_type();
	let fn_type = f64_type.fn_type(&[], false);

	// Create function
	let function = module.add_function("eval_postfix", fn_type, None);
	let basic_block = context.append_basic_block(function, "entry");
	builder.position_at_end(basic_block);

	// Stack to hold values during evaluation
	let mut value_stack: Vec<inkwell::values::FloatValue> = Vec::new();

	for token in tokens {
		match token {
			Token::Number(lex::LangNumber::Integer(int_val)) => {
				// Push integer constant onto stack
				let const_val = f64_type.const_float(int_val.value as f64);
				value_stack.push(const_val);
			}
			Token::Number(lex::LangNumber::RealNumber(real_val)) => {
				let const_val = f64_type.const_float(real_val.value as f64);
				value_stack.push(const_val);
			}
			Token::Operator(op) => match op.value.as_str() {
				"+" => {
					if value_stack.len() >= 2 {
						let b = value_stack.pop().unwrap();
						let a = value_stack.pop().unwrap();
						let result = builder.build_float_add(a, b, "add").unwrap();
						value_stack.push(result);
					}
				}
				"-" => {
					if value_stack.len() >= 2 {
						let b = value_stack.pop().unwrap();
						let a = value_stack.pop().unwrap();
						let result = builder.build_float_sub(a, b, "sub").unwrap();
						value_stack.push(result);
					}
				}
				"*" => {
					if value_stack.len() >= 2 {
						let b = value_stack.pop().unwrap();
						let a = value_stack.pop().unwrap();
						let result = builder.build_float_mul(a, b, "mul").unwrap();
						value_stack.push(result);
					}
				}
				"/" => {
					if value_stack.len() >= 2 {
						let b = value_stack.pop().unwrap();
						let a = value_stack.pop().unwrap();
						let result = builder.build_float_div(a, b, "div").unwrap();
						value_stack.push(result);
					}
				}
				_ => {
					println!("Warning: Operator '{}' not supported yet", op.value);
				}
			},
			Token::Symbol(_) | Token::String(_) => {
				println!("Warning: Symbols and strings not supported in arithmetic evaluation");
			}
		}
	}

	// Return the final result (top of stack) or 0 if stack is empty
	let result = if let Some(final_value) = value_stack.pop() {
		final_value
	} else {
		f64_type.const_float(0.0)
	};

	builder.build_return(Some(&result)).unwrap();

	// JIT compile and call
	type EvalFunc = unsafe extern "C" fn() -> f64;
	let eval_fn: JitFunction<'_, EvalFunc> =
		unsafe { execution_engine.get_function("eval_postfix").unwrap() };

	unsafe {
		let result = eval_fn.call();
		println!("Evaluation result: {}", result);
	}

	Ok(())
}

fn eval_line(line: &LangLine) {
	// println!("Evaluating line:");

	// Convert infix to postfix using Shunting Yard algorithm
	let postfix_tokens = infix_to_postfix(&line.tokens);

	// println!("Original tokens: {:?}", line.tokens);
	// println!("Postfix tokens: {:?}", postfix_tokens);

	let _ = execute_postfix_tokens(&postfix_tokens);
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

fn eval_block(block: &LangBlock) {
	// println!("Evaluating block:");

	for item in &block.items {
		match item {
			parse::LangBlockItem::Line(line) => {
				eval_line(line);
			}
			parse::LangBlockItem::Block(nested_block) => {
				eval_block(nested_block);
			}
		}
	}
}

fn run(line: &str) {
	// println!("Tokenizing: {}", line);
	let tokens = lex(line);

	// Parse tokens into a LangBlock with support for nested blocks
	let mut token_iter = tokens.into_iter().peekable();
	let block = parse_block(&mut token_iter);

	// println!("Parsed block:");
	// print!("{}", block);

	eval_block(&block);
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
				run(line.as_str());
			}
			Err(_) => {
				break;
			}
		}
	}
	let _ = rl.save_history("repl_history.txt");
	Ok(())
}
