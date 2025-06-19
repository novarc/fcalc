use rustyline;



fn run(line: &str) {
    println!("{}", line);
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
