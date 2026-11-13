use super::describe;
use std::fs::File;
use std::io::{self, Write};

pub fn run(args: &[String]) -> i32 {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut status = 0;

    if args.is_empty() {
        if let Err(e) = io::copy(&mut io::stdin().lock(), &mut out) {
            eprintln!("cat: {}", describe(&e));
            status = 1;
        }
        let _ = out.flush();
        return status;
    }

    for arg in args {
        let result = if arg == "-" {
            io::copy(&mut io::stdin().lock(), &mut out)
        } else {
            File::open(arg).and_then(|mut file| io::copy(&mut file, &mut out))
        };
        if let Err(e) = result {
            let _ = out.flush();
            eprintln!("cat: {arg}: {}", describe(&e));
            status = 1;
        }
    }
    let _ = out.flush();
    status
}
