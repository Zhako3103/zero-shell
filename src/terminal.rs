//! Tiny libc FFI helpers: SIGINT handling and terminal width.

use std::ffi::c_int;

extern "C" {
    fn signal(signum: c_int, handler: usize) -> usize;
    fn write(fd: c_int, buf: *const u8, count: usize) -> isize;
}

const SIGINT: c_int = 2;

/// Async-signal-safe handler: just redraws a fresh prompt.
extern "C" fn on_sigint(_signum: c_int) {
    const MSG: &[u8] = b"\n$ ";
    // SAFETY: write(2) is async-signal-safe and MSG is a valid buffer.
    unsafe {
        write(1, MSG.as_ptr(), MSG.len());
    }
}

/// Makes Ctrl+C redraw the prompt instead of killing the shell.
pub fn install_sigint_handler() {
    // SAFETY: installing a handler that only calls async-signal-safe functions.
    unsafe {
        signal(SIGINT, on_sigint as usize);
    }
}

/// Terminal width in columns (80 if unknown).
pub fn width() -> usize {
    ioctl_width()
        .or_else(|| std::env::var("COLUMNS").ok().and_then(|c| c.parse().ok()))
        .filter(|w| *w > 0)
        .unwrap_or(80)
}

#[cfg(target_os = "linux")]
fn ioctl_width() -> Option<usize> {
    use std::ffi::c_ulong;

    extern "C" {
        fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
    }
    #[repr(C)]
    struct WinSize {
        rows: u16,
        cols: u16,
        xpixel: u16,
        ypixel: u16,
    }
    const TIOCGWINSZ: c_ulong = 0x5413;

    let mut ws = WinSize { rows: 0, cols: 0, xpixel: 0, ypixel: 0 };
    // SAFETY: `ws` is a valid, correctly laid out winsize struct.
    let rc = unsafe { ioctl(1, TIOCGWINSZ, &mut ws as *mut WinSize) };
    if rc == 0 && ws.cols > 0 {
        Some(ws.cols as usize)
    } else {
        None
    }
}

#[cfg(not(target_os = "linux"))]
fn ioctl_width() -> Option<usize> {
    None
}
