// term.rs — public API, cfg-dispatches to a platform backend module
#[cfg(unix)]
use crate::term_unix as imp;
#[cfg(windows)]
use crate::term_windows as imp;

pub struct RawModeGuard(#[allow(dead_code)] imp::RawModeGuard);
impl RawModeGuard {
    /// Puts the controlling terminal/console into raw mode (no echo, no line
    /// buffering, no signal chars) and — on Windows — enables VT100 escape
    /// processing on stdout so `renderer.rs` needs no platform branch at all.
    /// Restores the original settings on Drop.
    pub fn enable() -> std::io::Result<Self> {
        imp::RawModeGuard::enable().map(Self)
    }
}

/// returns size of row, col of terminal or something.
pub fn terminal_size() -> std::io::Result<(u16, u16)> {
    imp::terminal_size()
}
