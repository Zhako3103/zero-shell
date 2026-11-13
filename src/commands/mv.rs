use super::cp::{base_name, copy_tree, is_inside};
use super::{describe, parse_flags};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

/// errno for "cross-device link" on Linux.
const EXDEV: i32 = 18;

pub fn run(args: &[String]) -> i32 {
    let parsed = match parse_flags("mv", args, "f") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    let operands = &parsed.operands;
    if operands.is_empty() {
        eprintln!("mv: missing file operand");
        return 1;
    }
    if operands.len() == 1 {
        eprintln!("mv: missing destination file operand after '{}'", operands[0]);
        return 1;
    }
    let (dest, sources) = match operands.split_last() {
        Some(split) => split,
        None => return 1,
    };
    let dest_path = Path::new(dest);
    let dest_is_dir = dest_path.is_dir();
    if sources.len() > 1 && !dest_is_dir {
        eprintln!("mv: target '{dest}' is not a directory");
        return 1;
    }

    let mut status = 0;
    for src in sources {
        let target = if dest_is_dir {
            match base_name(Path::new(src)) {
                Some(name) => dest_path.join(name),
                None => {
                    eprintln!("mv: cannot move '{src}'");
                    status = 1;
                    continue;
                }
            }
        } else {
            dest_path.to_path_buf()
        };
        if let Err(msg) = move_one(Path::new(src), &target) {
            eprintln!("mv: {msg}");
            status = 1;
        }
    }
    status
}

fn move_one(src: &Path, target: &Path) -> Result<(), String> {
    let meta = fs::symlink_metadata(src)
        .map_err(|e| format!("cannot stat '{}': {}", src.display(), describe(&e)))?;

    if let Ok(target_meta) = fs::symlink_metadata(target) {
        if target_meta.dev() == meta.dev() && target_meta.ino() == meta.ino() {
            return Err(format!(
                "'{}' and '{}' are the same file",
                src.display(),
                target.display()
            ));
        }
    }
    if meta.is_dir() && is_inside(src, target) {
        return Err(format!(
            "cannot move '{}' to a subdirectory of itself, '{}'",
            src.display(),
            target.display()
        ));
    }

    match fs::rename(src, target) {
        Ok(()) => Ok(()),
        Err(e) if e.raw_os_error() == Some(EXDEV) => {
            copy_tree(src, target)?;
            let removed = if meta.is_dir() {
                fs::remove_dir_all(src)
            } else {
                fs::remove_file(src)
            };
            removed.map_err(|e| format!("cannot remove '{}': {}", src.display(), describe(&e)))
        }
        Err(e) => Err(format!(
            "cannot move '{}' to '{}': {}",
            src.display(),
            target.display(),
            describe(&e)
        )),
    }
}
