// term_unix.rs — #[cfg(unix)]
use libc::{ECHO, ICANON, IXON, TCSANOW, tcgetattr, tcsetattr, termios};
use std::io;
use std::os::fd::AsRawFd;

pub(crate) struct RawModeGuard {
    original: libc::termios,
}
impl RawModeGuard {
    pub fn enable() -> std::io::Result<Self> {
        let stdin_fd = io::stdin().as_raw_fd();
        let mut original_orig = unsafe { std::mem::zeroed::<termios>() };
        unsafe {
            if tcgetattr(stdin_fd, &mut original_orig) != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        let mut raw = original_orig;
        raw.c_lflag &= !(ICANON | ECHO | libc::ISIG);
        raw.c_iflag &= !IXON;
        unsafe {
            if tcsetattr(stdin_fd, TCSANOW, &raw) != 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(RawModeGuard {
            original: original_orig,
        })
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let stdin_fd = io::stdin().as_raw_fd();
        unsafe { tcsetattr(stdin_fd, TCSANOW, &self.original) };
    }
}

/// returns terminal row, col size using ioctl(TIOCGWINSZ).
/// Calls libc raw ioctl.
pub(crate) fn terminal_size() -> std::io::Result<(u16, u16)> {
    let mut ws: libc::winsize = libc::winsize {
        ws_col: 0,
        ws_row: 0,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    let fd = std::io::stdout().as_raw_fd();
    unsafe {
        if libc::ioctl(fd, libc::TIOCGWINSZ, &mut ws) == 0 {
            Ok((ws.ws_row, ws.ws_col))
        } else {
            Err(std::io::Error::last_os_error())
        }
    }
}
