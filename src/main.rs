use rustyline;

mod lex;
use lex::lex;

fn run(line: &str) {
	println!("Tokenizing: {}", line);
	let tokens = lex(line);

	// Parse tokens into lines based on '\n' operator
	let mut lines: Vec<Vec<lex::Token>> = Vec::new();
	let mut current_line: Vec<lex::Token> = Vec::new();

	for token in tokens {
		match &token {
			lex::Token::Operator(op) if op.value == "\n" || op.value == ";" => {
				// End of line - push current line and start new one
				lines.push(current_line);
				current_line = Vec::new();
			}
			_ => {
				// Add token to current line
				current_line.push(token);
			}
		}
	}

	// Don't forget the last line if it doesn't end with newline
	if !current_line.is_empty() {
		lines.push(current_line);
	}

	println!("Parsed into {} lines:", lines.len());
	for (i, line) in lines.iter().enumerate() {
		println!("Line {}: {:?}", i + 1, line);
	}
}

fn main() {
	println!("\n\n");
	run("2 + 4 /! 5 - 3 + \"hello\" /* yea */ \n 123");
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
