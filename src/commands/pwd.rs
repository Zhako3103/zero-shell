use super::describe;
use std::env;

pub fn run(_args: &[String]) -> i32 {
    match env::current_dir() {
        Ok(path) => {
            println!("{}", path.display());
            0
        }
        Err(e) => {
            eprintln!("pwd: {}", describe(&e));
            1
        }
    }
}
