use rustyline;

mod lex;
mod parse;
use lex::lex;
use parse::{LangBlock, parse_block};

fn eval_line(line: &LangLine) {
	println!("Evaluating line:");
	print!("{}", line);
}

fn eval_block(block: &LangBlock) {
	println!("Evaluating block:");
	print!("{}", block);
}

fn run(line: &str) {
	println!("Tokenizing: {}", line);
	let tokens = lex(line);

	// Parse tokens into a LangBlock with support for nested blocks
	let mut token_iter = tokens.into_iter().peekable();
	let block = parse_block(&mut token_iter);

	println!("Parsed block:");
	print!("{}", block);
}

fn main() {
	println!("\n\n");
	run("2 + 4 /! 5 - 3 + \"hello\" /* yea */ \n 123");
	println!("\n--- Testing with blocks ---");
	run("if x > 0 { \n  y = x + 1; \n  z = y * 2 \n} else { \n  y = 0 \n}");
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
