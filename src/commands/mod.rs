//! Built-in command dispatch and shared helpers.

mod cat;
mod cd;
mod cp;
mod echo;
mod ls;
mod mkdir;
mod mv;
mod pwd;
mod rm;

use std::collections::HashSet;
use std::io;

/// What the shell loop should do after a command.
pub enum Flow {
    Continue,
    Exit(i32),
}

/// Runs the command described by `args` (name followed by its arguments).
pub fn execute(args: &[String]) -> Flow {
    let (name, rest) = match args.split_first() {
        Some(split) => split,
        None => return Flow::Continue,
    };
    let _status = match name.as_str() {
        "echo" => echo::run(rest),
        "cd" => cd::run(rest),
        "ls" => ls::run(rest),
        "pwd" => pwd::run(rest),
        "cat" => cat::run(rest),
        "cp" => cp::run(rest),
        "rm" => rm::run(rest),
        "mv" => mv::run(rest),
        "mkdir" => mkdir::run(rest),
        "help" => help(),
        "exit" => return exit(rest),
        other => {
            eprintln!("Command '{other}' not found");
            127
        }
    };
    Flow::Continue
}

fn exit(args: &[String]) -> Flow {
    match args {
        [] => Flow::Exit(0),
        [code] => match code.parse::<i32>() {
            Ok(n) => Flow::Exit(n & 0xff),
            Err(_) => {
                eprintln!("exit: {code}: numeric argument required");
                Flow::Exit(2)
            }
        },
        _ => {
            eprintln!("exit: too many arguments");
            Flow::Continue
        }
    }
}

fn help() -> i32 {
    println!(
        "0-shell built-ins:\n\
         \x20 echo [-n] [args...]      print arguments\n\
         \x20 cd [dir | -]             change directory (no arg: $HOME)\n\
         \x20 ls [-laF] [path...]      list directory contents\n\
         \x20 pwd                      print working directory\n\
         \x20 cat [file...]            print files\n\
         \x20 cp [-r] src... dest      copy files and directories\n\
         \x20 rm [-rf] path...         remove files and directories\n\
         \x20 mv src... dest           move / rename\n\
         \x20 mkdir [-p] dir...        create directories\n\
         \x20 exit [code]              leave the shell\n\
         Also: `;` chaining, $VAR expansion, quotes, Ctrl+C / Ctrl+D."
    );
    0
}

/// Result of option parsing for a command.
pub struct Parsed {
    flags: HashSet<char>,
    pub operands: Vec<String>,
}

impl Parsed {
    pub fn has(&self, flag: char) -> bool {
        self.flags.contains(&flag)
    }
}

/// Parses short flags (`-la`, `-r -f`, `--` terminator). `allowed` lists valid flags.
pub fn parse_flags(cmd: &str, args: &[String], allowed: &str) -> Result<Parsed, String> {
    let mut flags = HashSet::new();
    let mut operands = Vec::new();
    let mut only_operands = false;
    for arg in args {
        if only_operands || arg == "-" || !arg.starts_with('-') {
            operands.push(arg.clone());
        } else if arg == "--" {
            only_operands = true;
        } else {
            for c in arg.chars().skip(1) {
                if allowed.contains(c) {
                    flags.insert(c);
                } else {
                    return Err(format!("{cmd}: invalid option -- '{c}'"));
                }
            }
        }
    }
    Ok(Parsed { flags, operands })
}

/// Human readable I/O error without Rust's " (os error N)" suffix.
pub fn describe(err: &io::Error) -> String {
    let text = err.to_string();
    match text.find(" (os error") {
        Some(idx) => text[..idx].to_string(),
        None => text,
    }
}
