use super::{describe, parse_flags};
use std::ffi::OsString;
use std::fs;
use std::io;
use std::os::unix::fs::{symlink, MetadataExt};
use std::path::{Path, PathBuf};

pub fn run(args: &[String]) -> i32 {
    let parsed = match parse_flags("cp", args, "rR") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    let recursive = parsed.has('r') || parsed.has('R');
    let operands = &parsed.operands;
    if operands.is_empty() {
        eprintln!("cp: missing file operand");
        return 1;
    }
    if operands.len() == 1 {
        eprintln!("cp: missing destination file operand after '{}'", operands[0]);
        return 1;
    }
    let (dest, sources) = match operands.split_last() {
        Some(split) => split,
        None => return 1,
    };
    let dest_path = Path::new(dest);
    let dest_is_dir = dest_path.is_dir();
    if sources.len() > 1 && !dest_is_dir {
        eprintln!("cp: target '{dest}' is not a directory");
        return 1;
    }

    let mut status = 0;
    for src in sources {
        let target = if dest_is_dir {
            match base_name(Path::new(src)) {
                Some(name) => dest_path.join(name),
                None => {
                    eprintln!("cp: cannot copy '{src}'");
                    status = 1;
                    continue;
                }
            }
        } else {
            dest_path.to_path_buf()
        };
        if let Err(msg) = copy_operand(Path::new(src), &target, recursive) {
            eprintln!("cp: {msg}");
            status = 1;
        }
    }
    status
}

/// Final path component, resolving `.`/`..` when needed.
pub fn base_name(path: &Path) -> Option<OsString> {
    path.file_name().map(|n| n.to_os_string()).or_else(|| {
        fs::canonicalize(path)
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_os_string()))
    })
}

/// True if `target` is `src` itself or lies inside it.
pub fn is_inside(src: &Path, target: &Path) -> bool {
    let src_abs = match fs::canonicalize(src) {
        Ok(p) => p,
        Err(_) => return false,
    };
    let target_abs = match fs::canonicalize(target) {
        Ok(p) => p,
        Err(_) => {
            let parent = match target.parent() {
                Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
                _ => PathBuf::from("."),
            };
            match (fs::canonicalize(parent), target.file_name()) {
                (Ok(p), Some(name)) => p.join(name),
                _ => return false,
            }
        }
    };
    target_abs.starts_with(&src_abs)
}

fn copy_operand(src: &Path, target: &Path, recursive: bool) -> Result<(), String> {
    let meta = fs::metadata(src)
        .map_err(|e| format!("cannot stat '{}': {}", src.display(), describe(&e)))?;

    if meta.is_dir() {
        if !recursive {
            return Err(format!("-r not specified; omitting directory '{}'", src.display()));
        }
        if is_inside(src, target) {
            return Err(format!(
                "cannot copy a directory, '{}', into itself, '{}'",
                src.display(),
                target.display()
            ));
        }
        return copy_tree(src, target);
    }

    if let Ok(target_meta) = fs::metadata(target) {
        if target_meta.dev() == meta.dev() && target_meta.ino() == meta.ino() {
            return Err(format!(
                "'{}' and '{}' are the same file",
                src.display(),
                target.display()
            ));
        }
        if target_meta.is_dir() {
            return Err(format!(
                "cannot overwrite directory '{}' with non-directory",
                target.display()
            ));
        }
    }
    fs::copy(src, target).map(|_| ()).map_err(|e| {
        format!("cannot create regular file '{}': {}", target.display(), describe(&e))
    })
}

/// Recursively copies a file, symlink or directory tree.
pub fn copy_tree(src: &Path, dst: &Path) -> Result<(), String> {
    let fail = |e: io::Error| -> String {
        format!("cannot copy '{}' to '{}': {}", src.display(), dst.display(), describe(&e))
    };
    let meta = fs::symlink_metadata(src).map_err(&fail)?;
    let file_type = meta.file_type();

    if file_type.is_symlink() {
        let link_target = fs::read_link(src).map_err(&fail)?;
        let _ = fs::remove_file(dst);
        symlink(link_target, dst).map_err(&fail)
    } else if file_type.is_dir() {
        if let Err(e) = fs::create_dir(dst) {
            if !dst.is_dir() {
                return Err(fail(e));
            }
        }
        for entry in fs::read_dir(src).map_err(&fail)? {
            let entry = entry.map_err(&fail)?;
            copy_tree(&entry.path(), &dst.join(entry.file_name()))?;
        }
        fs::set_permissions(dst, meta.permissions()).map_err(&fail)
    } else {
        fs::copy(src, dst).map(|_| ()).map_err(&fail)
    }
}
