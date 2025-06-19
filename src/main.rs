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
    Vec::new()
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
