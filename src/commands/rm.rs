use super::{describe, parse_flags};
use std::fs;
use std::io;
use std::path::Path;

pub fn run(args: &[String]) -> i32 {
    let parsed = match parse_flags("rm", args, "rRf") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    let recursive = parsed.has('r') || parsed.has('R');
    let force = parsed.has('f');

    if parsed.operands.is_empty() {
        if force {
            return 0;
        }
        eprintln!("rm: missing operand");
        return 1;
    }

    let mut status = 0;
    for operand in &parsed.operands {
        let path = Path::new(operand);
        if path.file_name().is_none() {
            if path == Path::new("/") {
                eprintln!("rm: it is dangerous to operate recursively on '/'");
            } else {
                eprintln!("rm: refusing to remove '.' or '..' directory: skipping '{operand}'");
            }
            status = 1;
            continue;
        }
        match remove(path, recursive) {
            Ok(()) => {}
            Err(e) if force && e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => {
                eprintln!("rm: cannot remove '{operand}': {}", describe(&e));
                status = 1;
            }
        }
    }
    status
}

fn remove(path: &Path, recursive: bool) -> io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.is_dir() {
        if !recursive {
            return Err(io::Error::new(io::ErrorKind::Other, "Is a directory"));
        }
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}
