#[derive(Debug)]
pub struct LangInteger {
	pub value: i64,
}

#[derive(Debug)]
pub struct LangRealNumber {
	pub value: f64,
}

#[derive(Debug)]
pub enum LangNumber {
	Integer(LangInteger),
	RealNumber(LangRealNumber),
}

#[derive(Debug)]
pub struct LangSymbol {
	pub value: String,
}

#[derive(Debug)]
pub struct LangString {
	pub value: String,
}

#[derive(Debug)]
pub struct LangOperator {
	pub value: String,
}

#[derive(Debug)]
pub enum Token {
	Number(LangNumber),
	Symbol(LangSymbol),
	String(LangString),
	Operator(LangOperator),
}

pub fn lex(line: &str) -> Vec<Token> {
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
		if ch == '\r' {
			continue;
		} else if ch == '\n' {
			// Finish the current token
			token_chars_collection.push(std::mem::replace(&mut current_token_chars, None).unwrap());
			token_chars_collection.push(TokenChars::Operator(vec!['\n']));
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
		TokenChars::Comment(_) => false,
		_ => true,
	});

	// Transform TokenChars into Tokens
	for token_chars in token_chars_collection {
		match token_chars {
			TokenChars::Whitespace(chars) => {
				// skip
			}
			TokenChars::Number(chars) => {
				let num_str: String = chars.into_iter().collect();
				if num_str.contains('.') {
					tokens.push(Token::Number(LangNumber::RealNumber(LangRealNumber {
						value: num_str.parse().unwrap_or(0.0),
					})));
				} else {
					tokens.push(Token::Number(LangNumber::Integer(LangInteger {
						value: num_str.parse().unwrap_or(0),
					})));
				}
			}
			TokenChars::Symbol(chars) => {
				tokens.push(Token::Symbol(LangSymbol {
					value: chars.into_iter().collect(),
				}));
			}
			TokenChars::String(chars) => {
				tokens.push(Token::String(LangString {
					value: chars.into_iter().collect(),
				}));
			}
			TokenChars::Operator(chars) => {
				tokens.push(Token::Operator(LangOperator {
					value: chars.into_iter().collect(),
				}));
			}
			TokenChars::Comment(chars) => {
				// skip
			}
		}
	}

	tokens
}
