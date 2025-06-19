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
        // String(Vec<char>),
        // Comment(Vec<char>),
    }

    let mut token_chars_collection: Vec<TokenChars> = Vec::new();
    let mut current_token_chars: Option<TokenChars> = None;

    fn determine_token_kind(ch: char) -> TokenChars {
        if ch.is_alphabetic() {
            TokenChars::Symbol(vec![ch])
        } else if ch.is_numeric() {
            TokenChars::Number(vec![ch])
        } else if ch.is_whitespace() {
            TokenChars::Whitespace(vec![ch])
        } else {
            todo!("Unknown token kind char: {}", ch)
        }
    }

    for ch in line.chars() {
        

        match current_token_chars {
            None => {
                current_token_chars = Some(determine_token_kind(ch));
            }
            Some(ref mut token_chars) => {
                match token_chars {
                    TokenChars::Whitespace(chars) => {
                        chars.push(ch);
                    }
                    TokenChars::Number(chars) => {
                        chars.push(ch);
                    }
                    TokenChars::Symbol(chars) => {
                        chars.push(ch);
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
    run("2 + 3");

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
