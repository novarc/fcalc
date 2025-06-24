use rustyline;

#[derive(Debug)]
struct WhiteSpace {
	whitespace: String,
}

#[derive(Debug)]
struct Integer {
	value: i64,
}

#[derive(Debug)]
struct RealNumber {
	value: f64,
}

#[derive(Debug)]
enum Number {
	Integer(Integer),
	RealNumber(RealNumber),
}

#[derive(Debug)]
struct Symbol {
	value: String,
}

#[derive(Debug)]
enum Token {
	Whitespace(WhiteSpace),
	Number(Number),
	Symbol(Symbol),
}

fn tokenize(line: &str) -> Vec<Token> {
	let mut tokens: Vec<Token> = Vec::new();

	#[derive(Debug)]
	enum TokenChars {
		Whitespace(Vec<char>),
		Number(Vec<char>),
		Symbol(Vec<char>),
		String(Vec<char>),
		Operator(Vec<char>),
		Comment(Vec<char>),
	}

	let mut token_chars_collection: Vec<TokenChars> = Vec::new();
	let mut current_token_chars: Option<TokenChars> = None;

	fn determine_token_kind(ch: char) -> TokenChars {
		if ch == '"' {
			TokenChars::String(vec![]) // Start with empty vector, don't include opening quote
		} else if ch.is_alphabetic() || ch == '_' {
			TokenChars::Symbol(vec![ch])
		} else if ch.is_numeric() {
			TokenChars::Number(vec![ch])
		} else if ch.is_whitespace() {
			TokenChars::Whitespace(vec![ch])
		} else {
			TokenChars::Operator(vec![ch])
		}
	}

	for ch in line.chars() {
		println!("{}", ch);

		if ch == '\r' {
			continue;
		} else if ch == '\n' {
			// Finish the current token
			token_chars_collection.push(std::mem::replace(&mut current_token_chars, None).unwrap());
			continue;
		}

		// Check if we need to convert an operator to a comment
		if let Some(TokenChars::Operator(chars)) = &mut current_token_chars {
			if chars.len() == 1 && chars[0] == '/' && (ch == '/' || ch == '*') {
				current_token_chars = Some(TokenChars::Comment(vec![chars[0], ch]));
				continue;
			}
		}

		match current_token_chars {
			None => {
				current_token_chars = Some(determine_token_kind(ch));
			}
			Some(ref mut token_chars) => {
				match token_chars {
					TokenChars::Whitespace(chars) => {
						if !ch.is_whitespace() {
							// End of whitespace token
							token_chars_collection.push(
								std::mem::replace(
									&mut current_token_chars,
									Some(determine_token_kind(ch)),
								)
								.unwrap(),
							);
						} else {
							chars.push(ch);
						}
					}
					TokenChars::Number(chars) => {
						if !ch.is_numeric() && ch != '.' {
							// End of number token
							token_chars_collection.push(
								std::mem::replace(
									&mut current_token_chars,
									Some(determine_token_kind(ch)),
								)
								.unwrap(),
							);
						} else {
							chars.push(ch);
						}
					}
					TokenChars::Symbol(chars) => {
						if !ch.is_alphanumeric() && ch != '_' {
							// End of symbol token
							token_chars_collection.push(
								std::mem::replace(
									&mut current_token_chars,
									Some(determine_token_kind(ch)),
								)
								.unwrap(),
							);
						} else {
							chars.push(ch);
						}
					}
					TokenChars::String(chars) => {
						if ch == '"' {
							// End of string token - don't add the closing quote, just finalize
							token_chars_collection
								.push(std::mem::replace(&mut current_token_chars, None).unwrap());
						} else {
							chars.push(ch);
						}
					}
					TokenChars::Operator(chars) => {
						if ch == '\n' || ch == '"' || ch.is_whitespace() || ch.is_alphanumeric() {
							// End of operator token
							token_chars_collection.push(
								std::mem::replace(
									&mut current_token_chars,
									Some(determine_token_kind(ch)),
								)
								.unwrap(),
							);
						} else {
							chars.push(ch);
						}
					}
					TokenChars::Comment(chars) => {
						chars.push(ch);

						let l = chars.len();
						let mut end_of_comment = false;
						if chars[0] == '/' && chars[1] == '*' {
							if chars[l - 1] == '*' && chars[l - 2] == '/' {
								end_of_comment = true;
							}
						} else if chars[0] == '/' && chars[1] == '/' {
							if chars[l - 1] == '\n' {
								end_of_comment = true;
							}
						}

						if end_of_comment {
							// End of comment token
							token_chars_collection
								.push(std::mem::replace(&mut current_token_chars, None).unwrap());
						}
					}
				}
			}
		}
	}

	if let Some(token_chars) = current_token_chars {
		token_chars_collection.push(token_chars);
	}

	// Remove Whitespace tokens from token_chars_collection
	token_chars_collection.retain(|token| match token {
		TokenChars::Whitespace(_) => false,
		_ => true,
	});

	println!("{:?}", token_chars_collection);

	tokens
}

fn run(line: &str) {
	println!("{}", line);
	let tokens = tokenize(line);
	println!("{:?}", tokens);
}

fn main() {
	println!("");
	println!("");
	run("2 + 4 /! 5 - 3 + \"hello\" /* yea */");

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
