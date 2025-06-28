#![allow(warnings)]

use crate::lex;
use std::fmt;
use std::iter::Peekable;
use std::vec::IntoIter;

#[derive(Clone)]
pub struct LangLine {
	pub tokens: Vec<lex::Token>,
}

#[derive(Clone)]
pub struct LangBlock {
	pub items: Vec<LangBlockItem>,
}

#[derive(Clone)]
pub struct LangFunction {
	pub parameters: Vec<String>,
	pub body: LangBlock,
}

#[derive(Clone)]
pub struct LangNamedFunction {
	pub name: String,
	pub parameters: Vec<String>,
	pub body: LangBlock,
}

#[derive(Clone)]
pub struct LangFunctionCall {
	pub name: String,
	pub arguments: Vec<Vec<lex::Token>>, // Each argument is a list of tokens forming an expression
}

#[derive(Clone)]
pub enum LangBlockItem {
	Line(LangLine),
	Block(LangBlock),
	Function(LangFunction),
	NamedFunction(LangNamedFunction),
	FunctionCall(LangFunctionCall),
}

pub struct DisplayBlock<'a> {
	block: &'a LangBlock,
	indent_level: usize,
}

pub fn parse_block(tokens: &mut Peekable<IntoIter<lex::Token>>) -> LangBlock {
	let mut block_items: Vec<LangBlockItem> = Vec::new();
	let mut current_line_tokens: Vec<lex::Token> = Vec::new();

	while let Some(token) = tokens.next() {
		match &token {
			lex::Token::Symbol(symbol) => {
				// Check if this is the 'fn' keyword for function definition
				if symbol.value == "fn" {
					// Parse function definition: fn name(params) { body }
					if let Some(lex::Token::Symbol(name_symbol)) = tokens.next() {
						let function_name = name_symbol.value.clone();

						// Expect opening parenthesis
						if let Some(lex::Token::Operator(op)) = tokens.next() {
							if op.value == "(" {
								// Parse parameters
								let parameters = parse_function_parameters_until_paren(tokens);

								// Expect opening brace
								if let Some(lex::Token::Operator(brace)) = tokens.next() {
									if brace.value == "{" {
										let body = parse_block(tokens);

										if !current_line_tokens.is_empty() {
											let lang_line = LangLine {
												tokens: current_line_tokens,
											};
											block_items.push(LangBlockItem::Line(lang_line));
											current_line_tokens = Vec::new();
										}

										// Create a named function
										let named_function = LangNamedFunction {
											name: function_name,
											parameters,
											body,
										};
										block_items
											.push(LangBlockItem::NamedFunction(named_function));
										continue;
									}
								}
							}
						}
					}
					// If we get here, it wasn't a valid function, treat as regular token
					current_line_tokens.push(token);
				}
				// Check if this is a function assignment: symbol = (params) => { body }
				else if let Some(lex::Token::Operator(op)) = tokens.peek() {
					if op.value == "=" {
						// Look ahead to see if this is a function assignment
						let mut lookahead_tokens = Vec::new();
						let mut temp_iter = tokens.clone();
						temp_iter.next(); // consume '='

						// Collect tokens until we find enough to determine if this is a function
						let mut paren_count = 0;
						let mut found_opening_paren = false;
						let mut found_arrow = false;

						while let Some(t) = temp_iter.next() {
							lookahead_tokens.push(t.clone());

							match t {
								lex::Token::Operator(op) if op.value == "(" => {
									found_opening_paren = true;
									paren_count += 1;
								}
								lex::Token::Operator(op) if op.value == ")" => {
									paren_count -= 1;
									if paren_count == 0 && found_opening_paren {
										// Check if next token is '=>'
										if let Some(next_t) = temp_iter.next() {
											lookahead_tokens.push(next_t.clone());
											if let lex::Token::Operator(arrow_op) = next_t {
												if arrow_op.value == "=>" {
													found_arrow = true;
												}
											}
										}
										break;
									}
								}
								_ => {}
							}

							// Limit lookahead to prevent infinite loops
							if lookahead_tokens.len() > 20 {
								break;
							}
						}

						// Check if it starts with '(' and has '=>' pattern (indicating function parameters)
						let is_function_assignment = found_opening_paren && found_arrow;

						if is_function_assignment {
							// This is a function assignment: name = (params) => { body }
							tokens.next(); // consume '='
							tokens.next(); // consume '('

							// Parse function parameters and body
							let parameters = parse_function_parameters_until_paren(tokens);

							// Expect '=>'
							if let Some(lex::Token::Operator(arrow)) = tokens.next() {
								if arrow.value == "=>" {
									// Expect '{'
									if let Some(lex::Token::Operator(brace)) = tokens.next() {
										if brace.value == "{" {
											let body = parse_block(tokens);

											if !current_line_tokens.is_empty() {
												let lang_line = LangLine {
													tokens: current_line_tokens,
												};
												block_items.push(LangBlockItem::Line(lang_line));
												current_line_tokens = Vec::new();
											}

											// Create a named function
											let named_function = LangNamedFunction {
												name: symbol.value.clone(),
												parameters,
												body,
											};
											block_items
												.push(LangBlockItem::NamedFunction(named_function));
											continue;
										}
									}
								}
							}
							// If we get here, it wasn't a valid function, treat as regular tokens
							current_line_tokens.push(token);
							current_line_tokens.push(lex::Token::Operator(lex::LangOperator {
								value: "=".to_string(),
							}));
						} else {
							// Regular assignment - add symbol and let normal flow handle the rest
							current_line_tokens.push(token);
						}
					} else if op.value == "(" {
						// This is a function call
						tokens.next(); // consume the '('

						let arguments = parse_function_arguments(tokens);

						if !current_line_tokens.is_empty() {
							let lang_line = LangLine {
								tokens: current_line_tokens,
							};
							block_items.push(LangBlockItem::Line(lang_line));
							current_line_tokens = Vec::new();
						}

						block_items.push(LangBlockItem::FunctionCall(LangFunctionCall {
							name: symbol.value.clone(),
							arguments,
						}));
					} else {
						// Regular symbol, add to current line
						current_line_tokens.push(token);
					}
				} else {
					// Regular symbol, add to current line
					current_line_tokens.push(token);
				}
			}
			lex::Token::Operator(op) if op.value == "(" => {
				// Try to parse as function by collecting all tokens first and checking for function pattern
				let mut lookahead_tokens = vec![token.clone()];
				let mut paren_count = 1;
				let mut found_arrow = false;

				// Collect tokens until we either find '=>' after balanced parens or hit something else
				let mut temp_tokens = Vec::new();
				while let Some(peek_token) = tokens.peek() {
					match peek_token {
						lex::Token::Operator(op) if op.value == "(" => {
							paren_count += 1;
							temp_tokens.push(tokens.next().unwrap());
						}
						lex::Token::Operator(op) if op.value == ")" => {
							paren_count -= 1;
							temp_tokens.push(tokens.next().unwrap());
							if paren_count == 0 {
								// Check if next token is '=>'
								if let Some(lex::Token::Operator(op)) = tokens.peek() {
									if op.value == "=>" {
										found_arrow = true;
										temp_tokens.push(tokens.next().unwrap()); // consume '=>'
									}
								}
								break;
							}
						}
						_ => {
							temp_tokens.push(tokens.next().unwrap());
						}
					}
				}

				if found_arrow {
					// This is a function - parse it
					lookahead_tokens.extend(temp_tokens.clone());
					let parameters =
						parse_parameters(&lookahead_tokens[1..lookahead_tokens.len() - 2]); // exclude parens and arrow

					// Parse the function body (expect a '{' followed by a block)
					if let Some(lex::Token::Operator(op)) = tokens.peek() {
						if op.value == "{" {
							tokens.next(); // consume the '{'
							let body = parse_block(tokens);

							if !current_line_tokens.is_empty() {
								let lang_line = LangLine {
									tokens: current_line_tokens,
								};
								block_items.push(LangBlockItem::Line(lang_line));
								current_line_tokens = Vec::new();
							}
							block_items
								.push(LangBlockItem::Function(LangFunction { parameters, body }));
						} else {
							// No function body, treat as regular tokens
							current_line_tokens.push(token);
							for t in temp_tokens {
								current_line_tokens.push(t);
							}
						}
					} else {
						// No more tokens, treat as regular tokens
						current_line_tokens.push(token);
						for t in temp_tokens {
							current_line_tokens.push(t);
						}
					}
				} else {
					// Not a function, put tokens back and treat as regular token
					current_line_tokens.push(token);
					for t in temp_tokens {
						current_line_tokens.push(t);
					}
				}
			}
			lex::Token::Operator(op) if op.value == "{" => {
				// Start of nested block - first finish current line if any
				if !current_line_tokens.is_empty() {
					let lang_line = LangLine {
						tokens: current_line_tokens,
					};
					block_items.push(LangBlockItem::Line(lang_line));
					current_line_tokens = Vec::new();
				}

				// Parse nested block recursively
				let nested_block = parse_block(tokens);
				block_items.push(LangBlockItem::Block(nested_block));
			}
			lex::Token::Operator(op) if op.value == "}" => {
				// End of current block - finish current line if any and return
				if !current_line_tokens.is_empty() {
					let lang_line = LangLine {
						tokens: current_line_tokens,
					};
					block_items.push(LangBlockItem::Line(lang_line));
				}
				return LangBlock { items: block_items };
			}
			lex::Token::Operator(op) if op.value == "\n" || op.value == ";" => {
				// End of line - create LangLine and add to block
				if !current_line_tokens.is_empty() {
					let lang_line = LangLine {
						tokens: current_line_tokens,
					};
					block_items.push(LangBlockItem::Line(lang_line));
					current_line_tokens = Vec::new();
				}
			}
			_ => {
				// Add token to current line
				current_line_tokens.push(token);
			}
		}
	}

	// Handle any remaining tokens at end of input
	if !current_line_tokens.is_empty() {
		let lang_line = LangLine {
			tokens: current_line_tokens,
		};
		block_items.push(LangBlockItem::Line(lang_line));
	}

	LangBlock { items: block_items }
}

fn parse_parameters(tokens: &[lex::Token]) -> Vec<String> {
	let mut parameters = Vec::new();

	for token in tokens {
		if let lex::Token::Symbol(symbol) = token {
			parameters.push(symbol.value.clone());
		}
	}

	parameters
}

fn parse_function_arguments(tokens: &mut Peekable<IntoIter<lex::Token>>) -> Vec<Vec<lex::Token>> {
	let mut arguments = Vec::new();
	let mut current_arg_tokens = Vec::new();
	let mut paren_depth = 0;

	while let Some(token) = tokens.next() {
		match &token {
			lex::Token::Operator(op) if op.value == ")" && paren_depth == 0 => {
				// End of function arguments
				if !current_arg_tokens.is_empty() {
					arguments.push(current_arg_tokens);
				}
				break;
			}
			lex::Token::Operator(op) if op.value == "(" => {
				paren_depth += 1;
				current_arg_tokens.push(token);
			}
			lex::Token::Operator(op) if op.value == ")" => {
				paren_depth -= 1;
				current_arg_tokens.push(token);
			}
			lex::Token::Operator(op) if op.value == "," && paren_depth == 0 => {
				// End of current argument
				if !current_arg_tokens.is_empty() {
					arguments.push(current_arg_tokens);
					current_arg_tokens = Vec::new();
				}
			}
			_ => {
				current_arg_tokens.push(token);
			}
		}
	}

	arguments
}

fn parse_function_parameters_until_paren(
	tokens: &mut Peekable<IntoIter<lex::Token>>,
) -> Vec<String> {
	let mut parameters = Vec::new();

	while let Some(token) = tokens.next() {
		match &token {
			lex::Token::Operator(op) if op.value == ")" => {
				// End of parameters
				break;
			}
			lex::Token::Symbol(symbol) => {
				parameters.push(symbol.value.clone());
			}
			lex::Token::Operator(op) if op.value == "," => {
				// Parameter separator, continue
			}
			_ => {
				// Ignore other tokens in parameter list
			}
		}
	}

	parameters
}

impl<'a> DisplayBlock<'a> {
	pub fn new(block: &'a LangBlock, indent_level: usize) -> Self {
		DisplayBlock {
			block,
			indent_level,
		}
	}
}

impl<'a> fmt::Display for DisplayBlock<'a> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let indent = "  ".repeat(self.indent_level);
		for (i, item) in self.block.items.iter().enumerate() {
			match item {
				LangBlockItem::Line(line) => {
					writeln!(f, "{}Line {}: {:?}", indent, i + 1, line.tokens)?;
				}
				LangBlockItem::Block(nested_block) => {
					writeln!(f, "{}Block {}:", indent, i + 1)?;
					write!(
						f,
						"{}",
						DisplayBlock::new(nested_block, self.indent_level + 1)
					)?;
				}
				LangBlockItem::Function(function) => {
					writeln!(
						f,
						"{}Function {}: ({}) => {{",
						indent,
						i + 1,
						function.parameters.join(", ")
					)?;
					write!(
						f,
						"{}",
						DisplayBlock::new(&function.body, self.indent_level + 1)
					)?;
					writeln!(f, "{}}}", indent)?;
				}
				LangBlockItem::NamedFunction(named_function) => {
					writeln!(
						f,
						"{}Named Function {}: {} ({}) => {{",
						indent,
						i + 1,
						named_function.name,
						named_function.parameters.join(", ")
					)?;
					write!(
						f,
						"{}",
						DisplayBlock::new(&named_function.body, self.indent_level + 1)
					)?;
					writeln!(f, "{}}}", indent)?;
				}
				LangBlockItem::FunctionCall(call) => {
					let args: Vec<String> = call
						.arguments
						.iter()
						.map(|tokens| {
							tokens
								.iter()
								.map(|t| match t {
									lex::Token::Number(lex::LangNumber::Integer(n)) => {
										n.value.to_string()
									}
									lex::Token::Number(lex::LangNumber::RealNumber(n)) => {
										n.value.to_string()
									}
									lex::Token::Symbol(s) => s.value.clone(),
									lex::Token::String(s) => format!("\"{}\"", s.value),
									lex::Token::Operator(o) => o.value.clone(),
								})
								.collect::<Vec<_>>()
								.join(" ")
						})
						.collect();
					writeln!(
						f,
						"{}Function Call {}: {}({})",
						indent,
						i + 1,
						call.name,
						args.join(", ")
					)?;
				}
			}
		}
		Ok(())
	}
}

impl fmt::Display for LangBlock {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", DisplayBlock::new(self, 0))
	}
}
