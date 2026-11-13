# 0-shell

Minimalist Unix shell in Rust. Uses only `std` (plus three tiny libc FFI calls for
SIGINT handling and terminal width). No external binaries are ever spawned.

## Build & run
    cargo build --release
    ./target/release/zero-shell        # (optionally: cp it to ./0-shell)

## Built-ins
echo, cd, ls (-l -a -F), pwd, cat, cp (-r), rm (-r -f), mv, mkdir (-p), exit, help

## Bonus
* Ctrl+C does not kill the shell, Ctrl+D exits
* `;` command chaining
* `$VAR` / `${VAR}` expansion, `~` expansion
* single/double quotes, backslash escapes, multi-line quoted input
* `help` command

Unknown command prints: `Command '<name>' not found`.
Note: `ls -l` prints times in UTC.
