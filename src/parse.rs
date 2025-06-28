#![allow(warnings)]

use crate::lex;
use std::fmt;
use std::iter::Peekable;
use std::vec::IntoIter;

pub struct LangLine {
	pub tokens: Vec<lex::Token>,
}

pub struct LangBlock {
	pub items: Vec<LangBlockItem>,
}

pub struct LangFunction {
	pub parameters: Vec<String>,
	pub body: LangBlock,
}

pub enum LangBlockItem {
	Line(LangLine),
	Block(LangBlock),
	Function(LangFunction),
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
