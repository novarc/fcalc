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

pub enum LangBlockItem {
	Line(LangLine),
	Block(LangBlock),
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
