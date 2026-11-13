//! 0-shell: a minimalist Unix-like shell.

mod commands;
mod parser;
mod terminal;

use commands::Flow;
use parser::ParseError;
use std::io::{self, BufRead, Write};

const PROMPT: &str = "$ ";
const CONTINUATION_PROMPT: &str = "> ";

fn show_prompt(prompt: &str) {
    print!("{prompt}");
    let _ = io::stdout().flush();
}

/// Reads one logical input (continues reading while a quote is unterminated).
/// Returns `None` on EOF with no pending input.
fn read_commands() -> Option<Vec<Vec<String>>> {
    let stdin = io::stdin();
    let mut buffer = String::new();
    loop {
        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => {
                if buffer.is_empty() {
                    return None;
                }
                eprintln!("0-shell: unexpected EOF while looking for matching quote");
                return Some(Vec::new());
            }
            Ok(_) => {
                buffer.push_str(&line);
                match parser::parse(&buffer) {
                    Ok(commands) => return Some(commands),
                    Err(ParseError::UnterminatedQuote) => show_prompt(CONTINUATION_PROMPT),
                }
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => {
                eprintln!("0-shell: input error: {e}");
                return None;
            }
        }
    }
}

fn main() {
    terminal::install_sigint_handler();
    loop {
        show_prompt(PROMPT);
        let commands = match read_commands() {
            Some(commands) => commands,
            None => {
                println!();
                break;
            }
        };
        for args in commands {
            if let Flow::Exit(code) = commands::execute(&args) {
                std::process::exit(code);
            }
        }
    }
}
