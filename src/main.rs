use rustyline;

mod lex;
use lex::lex;

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

fn run(line: &str) {
	println!("Tokenizing: {}", line);
	let tokens = lex(line);

	// Parse tokens into a LangBlock based on '\n' operator
	let mut block_items: Vec<LangBlockItem> = Vec::new();
	let mut current_line_tokens: Vec<lex::Token> = Vec::new();

	for token in tokens {
		match &token {
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

	// Don't forget the last line if it doesn't end with newline
	if !current_line_tokens.is_empty() {
		let lang_line = LangLine {
			tokens: current_line_tokens,
		};
		block_items.push(LangBlockItem::Line(lang_line));
	}

	let block = LangBlock { items: block_items };

	println!("Parsed into {} lines:", block.items.len());
	for (i, item) in block.items.iter().enumerate() {
		match item {
			LangBlockItem::Line(line) => {
				println!("Line {}: {:?}", i + 1, line.tokens);
			}
			LangBlockItem::Block(_) => {
				println!("Line {}: [nested block]", i + 1);
			}
		}
	}
}

fn main() {
	println!("\n\n");
	run("2 + 4 /! 5 - 3 + \"hello\" /* yea */ \n 123");
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
