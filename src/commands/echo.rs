use std::io::Write;

pub fn run(args: &[String]) -> i32 {
    let (newline, rest) = match args.first() {
        Some(first) if first == "-n" => (false, &args[1..]),
        _ => (true, args),
    };
    let mut text = rest.join(" ");
    if newline {
        text.push('\n');
    }
    let mut out = std::io::stdout().lock();
    let _ = out.write_all(text.as_bytes());
    let _ = out.flush();
    0
}
