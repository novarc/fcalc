use rustyline;

#[derive(Debug)]
struct WhiteSpace {
    whitespace: String
}

#[derive(Debug)]
struct Integer {
    value: i64
}

#[derive(Debug)]
struct RealNumber {
    value: f64
}

#[derive(Debug)]
enum Number {
    Integer(Integer),
    RealNumber(RealNumber)
}

#[derive(Debug)]
struct Symbol {
    value: String
}

#[derive(Debug)]
enum Token {
    Whitespace(WhiteSpace),
    Number(Number),
    Symbol(Symbol)
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
            TokenChars::String(vec![])  // Start with empty vector, don't include opening quote
        } else if ch == '/' {
            TokenChars::Comment(vec![ch])  // Start comment with '/'
        } else if ch.is_alphabetic() {
            TokenChars::Symbol(vec![ch])
        } else if ch.is_numeric() {
            TokenChars::Number(vec![ch])
        } else if ch.is_whitespace() {
            TokenChars::Whitespace(vec![ch])
        } else if is_operator_char(ch) {
            TokenChars::Operator(vec![ch])
        } else {
            todo!("Unknown token kind char: {}", ch)
        }
    }

    fn is_operator_char(ch: char) -> bool {
        matches!(ch, '+' | '-' | '*' | '/' | '=' | '<' | '>' | '!' | '&' | '|' | '^' | '%' | '~')
    }

    for ch in line.chars() {
        

        match current_token_chars {
            None => {
                current_token_chars = Some(determine_token_kind(ch));
            }
            Some(ref mut token_chars) => {
                match token_chars {
                    TokenChars::Whitespace(chars) => {
                        if !ch.is_whitespace() {
                            // End of whitespace token
                            token_chars_collection.push(std::mem::replace(&mut current_token_chars, Some(determine_token_kind(ch))).unwrap());
                        } else {
                            chars.push(ch);
                        }
                    }
                    TokenChars::Number(chars) => {
                        if !ch.is_numeric() && ch != '.' {
                            // End of number token
                            token_chars_collection.push(std::mem::replace(&mut current_token_chars, Some(determine_token_kind(ch))).unwrap());
                        } else {
                            chars.push(ch);
                        }
                    }
                    TokenChars::Symbol(chars) => {
                        if !ch.is_alphanumeric() && ch != '_' {
                            // End of symbol token
                            token_chars_collection.push(std::mem::replace(&mut current_token_chars, Some(determine_token_kind(ch))).unwrap());
                        } else {
                            chars.push(ch);
                        }
                    }
                    TokenChars::String(chars) => {
                        if ch == '"' {
                            // End of string token - don't add the closing quote, just finalize
                            token_chars_collection.push(std::mem::replace(&mut current_token_chars, None).unwrap());
                        } else {
                            chars.push(ch);
                        }
                    }
                    TokenChars::Operator(chars) => {
                        if !is_operator_char(ch) {
                            // End of operator token
                            token_chars_collection.push(std::mem::replace(&mut current_token_chars, Some(determine_token_kind(ch))).unwrap());
                        } else {
                            chars.push(ch);
                        }
                    }
                    TokenChars::Comment(chars) => {
                        // Handle C++ style comments
                        if chars.len() == 1 && chars[0] == '/' {
                            // First '/' encountered, check for second '/' or '*'
                            if ch == '/' || ch == '*' {
                                chars.push(ch);
                            } else {
                                // Not a comment, treat the '/' as an operator and start new token
                                let slash_token = TokenChars::Operator(vec!['/']);
                                token_chars_collection.push(slash_token);
                                current_token_chars = Some(determine_token_kind(ch));
                            }
                        } else if chars.len() == 2 {
                            if chars[0] == '/' && chars[1] == '/' {
                                // Single-line comment: continue until newline
                                if ch == '\n' {
                                    // End of single-line comment
                                    token_chars_collection.push(std::mem::replace(&mut current_token_chars, None).unwrap());
                                } else {
                                    chars.push(ch);
                                }
                            } else if chars[0] == '/' && chars[1] == '*' {
                                // Multi-line comment: continue until */
                                chars.push(ch);
                                if chars.len() >= 3 && chars[chars.len() - 2] == '*' && chars[chars.len() - 1] == '/' {
                                    // End of multi-line comment
                                    token_chars_collection.push(std::mem::replace(&mut current_token_chars, None).unwrap());
                                }
                            }
                        } else {
                            // Continue building comment
                            chars.push(ch);
                            // Check for end of multi-line comment
                            if chars.len() >= 3 && chars[chars.len() - 2] == '*' && chars[chars.len() - 1] == '/' {
                                // End of multi-line comment
                                token_chars_collection.push(std::mem::replace(&mut current_token_chars, None).unwrap());
                            }
                        }
                    }
                }
            }
        }
        
        
    }
    
    if let Some(token_chars) = current_token_chars {    
        token_chars_collection.push(token_chars);
    }

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
    run("2 + 4 / 5 - 3 + \"hello\" /* yea */");

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
