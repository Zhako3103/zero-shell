//! Command line parser: quotes, escapes, `$VAR`, `~` and `;` chaining.

use std::env;
use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug)]
pub enum ParseError {
    UnterminatedQuote,
}

/// Splits a line into commands (separated by unquoted `;`), each a list of words.
pub fn parse(line: &str) -> Result<Vec<Vec<String>>, ParseError> {
    let mut commands: Vec<Vec<String>> = Vec::new();
    let mut words: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_word = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                in_word = true;
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(ch) => current.push(ch),
                        None => return Err(ParseError::UnterminatedQuote),
                    }
                }
            }
            '"' => {
                in_word = true;
                parse_double_quoted(&mut chars, &mut current)?;
            }
            '\\' => {
                in_word = true;
                match chars.next() {
                    Some('\n') => {}
                    Some(next) => current.push(next),
                    None => current.push('\\'),
                }
            }
            '$' => {
                expand_variable(&mut chars, &mut current);
                if !current.is_empty() {
                    in_word = true;
                }
            }
            '~' if !in_word && tilde_is_home(chars.peek().copied()) => {
                in_word = true;
                current.push_str(&env::var("HOME").unwrap_or_else(|_| "~".to_string()));
            }
            ';' => {
                flush_word(&mut words, &mut current, &mut in_word);
                if !words.is_empty() {
                    commands.push(std::mem::take(&mut words));
                }
            }
            c if c.is_whitespace() => flush_word(&mut words, &mut current, &mut in_word),
            c => {
                in_word = true;
                current.push(c);
            }
        }
    }
    flush_word(&mut words, &mut current, &mut in_word);
    if !words.is_empty() {
        commands.push(words);
    }
    Ok(commands)
}

fn flush_word(words: &mut Vec<String>, current: &mut String, in_word: &mut bool) {
    if *in_word {
        words.push(std::mem::take(current));
        *in_word = false;
    }
}

fn tilde_is_home(next: Option<char>) -> bool {
    match next {
        None => true,
        Some(c) => c == '/' || c == ';' || c.is_whitespace(),
    }
}

fn parse_double_quoted(chars: &mut Peekable<Chars>, current: &mut String) -> Result<(), ParseError> {
    loop {
        match chars.next() {
            Some('"') => return Ok(()),
            Some('\\') => match chars.peek().copied() {
                Some(next) if next == '"' || next == '\\' || next == '$' => {
                    current.push(next);
                    chars.next();
                }
                Some('\n') => {
                    chars.next();
                }
                _ => current.push('\\'),
            },
            Some('$') => expand_variable(chars, current),
            Some(ch) => current.push(ch),
            None => return Err(ParseError::UnterminatedQuote),
        }
    }
}

/// Called right after a `$` was consumed; appends the variable's value.
fn expand_variable(chars: &mut Peekable<Chars>, out: &mut String) {
    let mut name = String::new();
    if chars.peek() == Some(&'{') {
        chars.next();
        let mut closed = false;
        for c in chars.by_ref() {
            if c == '}' {
                closed = true;
                break;
            }
            name.push(c);
        }
        if !closed {
            out.push_str("${");
            out.push_str(&name);
            return;
        }
    } else {
        while let Some(&c) = chars.peek() {
            if c.is_alphanumeric() || c == '_' {
                name.push(c);
                chars.next();
            } else {
                break;
            }
        }
        if name.is_empty() {
            out.push('$');
            return;
        }
    }
    out.push_str(&env::var(&name).unwrap_or_default());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Vec<Vec<String>> {
        parse(s).unwrap()
    }

    #[test]
    fn quotes_and_chaining() {
        assert_eq!(p("echo \"Hello   There\""), vec![vec!["echo", "Hello   There"]]);
        assert_eq!(p("a b; c 'd e'"), vec![vec!["a", "b"], vec!["c", "d e"]]);
        assert_eq!(p("echo \"\""), vec![vec!["echo", ""]]);
    }

    #[test]
    fn unterminated() {
        assert!(parse("echo \"abc").is_err());
    }
}
