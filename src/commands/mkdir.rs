use super::{describe, parse_flags};
use std::fs;

pub fn run(args: &[String]) -> i32 {
    let parsed = match parse_flags("mkdir", args, "p") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    if parsed.operands.is_empty() {
        eprintln!("mkdir: missing operand");
        return 1;
    }
    let mut status = 0;
    for dir in &parsed.operands {
        let result = if parsed.has('p') {
            fs::create_dir_all(dir)
        } else {
            fs::create_dir(dir)
        };
        if let Err(e) = result {
            eprintln!("mkdir: cannot create directory '{dir}': {}", describe(&e));
            status = 1;
        }
    }
    status
}
