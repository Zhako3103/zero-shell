use super::{describe, parse_flags};
use crate::terminal;
use std::collections::HashMap;
use std::fs::{self, Metadata};
use std::io::{self, IsTerminal, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
/// Roughly six months, the cut-off GNU ls uses to show a year instead of a time.
const SIX_MONTHS_SECS: i64 = 15_778_476;

struct Options {
    long: bool,
    all: bool,
    classify: bool,
}

struct Entry {
    name: String,
    meta: Metadata,
    link_target: Option<String>,
}

impl Entry {
    fn new(name: String, path: &Path, meta: Metadata) -> Self {
        let link_target = if meta.file_type().is_symlink() {
            fs::read_link(path)
                .ok()
                .map(|t| t.to_string_lossy().into_owned())
        } else {
            None
        };
        Entry { name, meta, link_target }
    }
}

pub fn run(args: &[String]) -> i32 {
    let parsed = match parse_flags("ls", args, "laF") {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            return 2;
        }
    };
    let opts = Options {
        long: parsed.has('l'),
        all: parsed.has('a'),
        classify: parsed.has('F'),
    };
    let operands = if parsed.operands.is_empty() {
        vec![".".to_string()]
    } else {
        parsed.operands.clone()
    };

    let mut status = 0;
    let mut files: Vec<Entry> = Vec::new();
    let mut dirs: Vec<String> = Vec::new();
    for operand in &operands {
        let path = Path::new(operand);
        match stat_operand(path, &opts) {
            Ok(meta) if meta.is_dir() => dirs.push(operand.clone()),
            Ok(meta) => files.push(Entry::new(operand.clone(), path, meta)),
            Err(e) => {
                eprintln!("ls: cannot access '{operand}': {}", describe(&e));
                status = 2;
            }
        }
    }
    dirs.sort();

    let mut sections: Vec<String> = Vec::new();
    if !files.is_empty() {
        sections.push(render(&mut files, &opts, false));
    }
    let show_headers = operands.len() > 1;
    for dir in &dirs {
        match list_dir(Path::new(dir), &opts) {
            Ok(mut entries) => {
                let mut section = String::new();
                if show_headers {
                    section.push_str(&format!("{dir}:\n"));
                }
                section.push_str(&render(&mut entries, &opts, true));
                sections.push(section);
            }
            Err(e) => {
                eprintln!("ls: cannot open directory '{dir}': {}", describe(&e));
                status = 2;
            }
        }
    }

    let mut out = io::stdout().lock();
    let _ = out.write_all(sections.join("\n").as_bytes());
    let _ = out.flush();
    status
}

/// Stats a command-line operand (follows symlinks unless `-l`).
fn stat_operand(path: &Path, opts: &Options) -> io::Result<Metadata> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() && !opts.long {
        Ok(fs::metadata(path).unwrap_or(meta))
    } else {
        Ok(meta)
    }
}

fn list_dir(path: &Path, opts: &Options) -> io::Result<Vec<Entry>> {
    let mut entries = Vec::new();
    if opts.all {
        for special in [".", ".."] {
            let p = path.join(special);
            if let Ok(meta) = fs::symlink_metadata(&p) {
                entries.push(Entry::new(special.to_string(), &p, meta));
            }
        }
    }
    for item in fs::read_dir(path)? {
        let item = item?;
        let name = item.file_name().to_string_lossy().into_owned();
        if !opts.all && name.starts_with('.') {
            continue;
        }
        let p = item.path();
        match fs::symlink_metadata(&p) {
            Ok(meta) => entries.push(Entry::new(name, &p, meta)),
            Err(e) => eprintln!("ls: cannot access '{}': {}", p.display(), describe(&e)),
        }
    }
    Ok(entries)
}

fn sort_key(name: &str) -> (String, String) {
    (name.trim_start_matches('.').to_lowercase(), name.to_string())
}

fn render(entries: &mut [Entry], opts: &Options, with_total: bool) -> String {
    entries.sort_by_key(|e| sort_key(&e.name));
    if opts.long {
        return long_listing(entries, opts, with_total);
    }
    let names: Vec<String> = entries.iter().map(|e| display_name(e, opts)).collect();
    if io::stdout().is_terminal() {
        format_columns(&names, terminal::width())
    } else {
        names.iter().map(|n| format!("{n}\n")).collect()
    }
}

fn indicator(meta: &Metadata) -> Option<char> {
    let ft = meta.file_type();
    if ft.is_dir() {
        Some('/')
    } else if ft.is_symlink() {
        Some('@')
    } else if ft.is_fifo() {
        Some('|')
    } else if ft.is_socket() {
        Some('=')
    } else if ft.is_file() && meta.mode() & 0o111 != 0 {
        Some('*')
    } else {
        None
    }
}

fn display_name(entry: &Entry, opts: &Options) -> String {
    let mut name = entry.name.clone();
    let skip = opts.long && entry.link_target.is_some();
    if opts.classify && !skip {
        if let Some(c) = indicator(&entry.meta) {
            name.push(c);
        }
    }
    name
}

/// Column-major layout like `ls` does on a terminal.
fn format_columns(names: &[String], width: usize) -> String {
    let n = names.len();
    if n == 0 {
        return String::new();
    }
    let lens: Vec<usize> = names.iter().map(|s| s.chars().count()).collect();
    let mut cols_used = 1;
    let mut col_widths = vec![lens.iter().copied().max().unwrap_or(0)];
    for cols in (1..=n).rev() {
        let rows = (n + cols - 1) / cols;
        if (n + rows - 1) / rows != cols {
            continue;
        }
        let widths: Vec<usize> = (0..cols)
            .map(|c| {
                lens[c * rows..((c + 1) * rows).min(n)]
                    .iter()
                    .copied()
                    .max()
                    .unwrap_or(0)
            })
            .collect();
        let total: usize = widths.iter().sum::<usize>() + 2 * (cols - 1);
        if total <= width || cols == 1 {
            cols_used = cols;
            col_widths = widths;
            break;
        }
    }

    let rows = (n + cols_used - 1) / cols_used;
    let mut out = String::new();
    for r in 0..rows {
        for c in 0..cols_used {
            let idx = c * rows + r;
            if idx >= n {
                break;
            }
            out.push_str(&names[idx]);
            if (c + 1) * rows + r < n {
                let pad = col_widths[c] - lens[idx] + 2;
                out.push_str(&" ".repeat(pad));
            }
        }
        out.push('\n');
    }
    out
}

fn long_listing(entries: &[Entry], opts: &Options, with_total: bool) -> String {
    let users = load_names("/etc/passwd");
    let groups = load_names("/etc/group");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    let mut rows: Vec<[String; 7]> = Vec::new();
    let mut blocks: u64 = 0;
    for e in entries {
        let meta = &e.meta;
        blocks += meta.blocks();
        let ft = meta.file_type();
        let size = if ft.is_char_device() || ft.is_block_device() {
            let rdev = meta.rdev();
            format!("{}, {}", dev_major(rdev), dev_minor(rdev))
        } else {
            meta.len().to_string()
        };
        let mut name = display_name(e, opts);
        if let Some(target) = &e.link_target {
            name.push_str(" -> ");
            name.push_str(target);
        }
        rows.push([
            mode_string(meta.mode()),
            meta.nlink().to_string(),
            users
                .get(&meta.uid())
                .cloned()
                .unwrap_or_else(|| meta.uid().to_string()),
            groups
                .get(&meta.gid())
                .cloned()
                .unwrap_or_else(|| meta.gid().to_string()),
            size,
            format_time(meta.mtime(), now),
            name,
        ]);
    }

    let width = |col: usize| rows.iter().map(|r| r[col].chars().count()).max().unwrap_or(0);
    let (w_links, w_owner, w_group, w_size) = (width(1), width(2), width(3), width(4));

    let mut out = String::new();
    if with_total {
        out.push_str(&format!("total {}\n", (blocks + 1) / 2));
    }
    for r in &rows {
        out.push_str(&format!(
            "{} {:>w_links$} {:<w_owner$} {:<w_group$} {:>w_size$} {} {}\n",
            r[0], r[1], r[2], r[3], r[4], r[5], r[6],
        ));
    }
    out
}

fn mode_string(mode: u32) -> String {
    let file_type = match mode & 0o170000 {
        0o040000 => 'd',
        0o120000 => 'l',
        0o020000 => 'c',
        0o060000 => 'b',
        0o010000 => 'p',
        0o140000 => 's',
        _ => '-',
    };
    let bits = [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ];
    let mut chars: Vec<char> = vec![file_type];
    chars.extend(bits.iter().map(|&(bit, ch)| if mode & bit != 0 { ch } else { '-' }));
    // setuid / setgid / sticky bits replace the execute column
    for (special, idx, lower, upper) in [
        (0o4000, 3, 's', 'S'),
        (0o2000, 6, 's', 'S'),
        (0o1000, 9, 't', 'T'),
    ] {
        if mode & special != 0 {
            chars[idx] = if chars[idx] == 'x' { lower } else { upper };
        }
    }
    chars.into_iter().collect()
}

fn dev_major(dev: u64) -> u64 {
    ((dev >> 8) & 0xfff) | ((dev >> 32) & !0xfff)
}

fn dev_minor(dev: u64) -> u64 {
    (dev & 0xff) | ((dev >> 12) & !0xff)
}

/// Parses `/etc/passwd` or `/etc/group` into an id -> name map.
fn load_names(path: &str) -> HashMap<u32, String> {
    let mut map = HashMap::new();
    if let Ok(text) = fs::read_to_string(path) {
        for line in text.lines() {
            let mut fields = line.split(':');
            if let (Some(name), Some(_), Some(id)) = (fields.next(), fields.next(), fields.next()) {
                if let Ok(id) = id.parse::<u32>() {
                    map.entry(id).or_insert_with(|| name.to_string());
                }
            }
        }
    }
    map
}

fn format_time(secs: i64, now: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let mon = MONTHS[(month - 1) as usize];
    if (now - secs).abs() < SIX_MONTHS_SECS {
        format!("{mon} {day:>2} {:02}:{:02}", rem / 3600, (rem % 3600) / 60)
    } else {
        format!("{mon} {day:>2}  {year}")
    }
}

/// Days since 1970-01-01 -> (year, month, day) in the proleptic Gregorian calendar.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_723), (2024, 1, 1));
    }

    #[test]
    fn modes() {
        assert_eq!(mode_string(0o100644), "-rw-r--r--");
        assert_eq!(mode_string(0o041777), "drwxrwxrwt");
        assert_eq!(mode_string(0o104755), "-rwsr-xr-x");
    }
}
