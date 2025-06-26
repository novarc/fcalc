use rustyline;

mod lex;
mod parse;
use lex::{Token, lex};
use parse::{LangBlock, LangLine, parse_block};

fn eval_line(line: &LangLine) {
	println!("Evaluating line:");

	// Convert infix to postfix using Shunting Yard algorithm
	let postfix_tokens = infix_to_postfix(&line.tokens);

	println!("Original tokens: {:?}", line.tokens);
	println!("Postfix tokens: {:?}", postfix_tokens);
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
	println!("Evaluating block:");

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
	println!("Tokenizing: {}", line);
	let tokens = lex(line);

	// Parse tokens into a LangBlock with support for nested blocks
	let mut token_iter = tokens.into_iter().peekable();
	let block = parse_block(&mut token_iter);

	println!("Parsed block:");
	print!("{}", block);

	eval_block(&block);
}

fn main() {
	println!("\n=== Testing Infix to Postfix Conversion ===");

	println!("\n--- Simple arithmetic ---");
	run("2 + 3 * 4");

	// println!("\n--- Assignment with arithmetic ---");
	// run("x = a + b * c");

	println!("\n--- More complex expression ---");
	run("a + b * c - d / e");

	// println!("\n--- Original complex test ---");
	// run("2 + 4 /! 5 - 3 + \"hello\" /* yea */ \n 123");

	// println!("\n--- Testing with blocks ---");
	// run("if x > 0 { \n  y = x + 1; \n  z = y * 2 \n} else { \n  y = 0 \n}");

	// let _ = repl();
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
