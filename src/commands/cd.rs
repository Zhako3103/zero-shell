use super::describe;
use std::env;

pub fn run(args: &[String]) -> i32 {
    if args.len() > 1 {
        eprintln!("cd: too many arguments");
        return 1;
    }
    let mut print_target = false;
    let target = match args.first().map(String::as_str) {
        None => match env::var("HOME") {
            Ok(home) => home,
            Err(_) => {
                eprintln!("cd: HOME not set");
                return 1;
            }
        },
        Some("-") => match env::var("OLDPWD") {
            Ok(old) => {
                print_target = true;
                old
            }
            Err(_) => {
                eprintln!("cd: OLDPWD not set");
                return 1;
            }
        },
        Some(path) => path.to_string(),
    };

    let previous = env::current_dir().ok();
    if let Err(e) = env::set_current_dir(&target) {
        eprintln!("cd: {target}: {}", describe(&e));
        return 1;
    }
    if let Some(prev) = previous {
        env::set_var("OLDPWD", prev);
    }
    if let Ok(now) = env::current_dir() {
        if print_target {
            println!("{}", now.display());
        }
        env::set_var("PWD", now);
    }
    0
}
