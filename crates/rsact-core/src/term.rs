use crate::renderer::Renderer;
// term.rs — public API, cfg-dispatches to a platform backend module
#[cfg(unix)]
use crate::term_unix as imp;
#[cfg(windows)]
use crate::term_windows as imp;
use std::io::Write;

pub struct RawModeGuard {
    _rmg: imp::RawModeGuard,
    terminal: Option<(TerminalSize, Box<dyn Write>)>,
}
impl RawModeGuard {
    /// Puts the controlling terminal/console into raw mode (no echo, no line
    /// buffering, no signal chars) and — on Windows — enables VT100 escape
    /// processing on stdout so `renderer.rs` needs no platform branch at all.
    /// Restores the original settings on Drop.
    pub fn enable() -> std::io::Result<Self> {
        imp::RawModeGuard::enable().map(|_rmg| Self {
            _rmg,
            terminal: None,
        })
    }
    /// # Parameters
    /// - terminal : size of `Write` and object to write.
    ///
    /// It guarantees valid quit of raw mode (No cut-off in the middle).
    pub fn enable_safe_exit(
        terminal_size: TerminalSize,
        out: impl Write + 'static,
    ) -> std::io::Result<Self> {
        imp::RawModeGuard::enable().map(|_rmg| Self {
            _rmg,
            terminal: Some((terminal_size, Box::new(out))),
        })
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let Some((terminal_size, out)) = &mut self.terminal else {
            return;
        };
        let mut rend = Renderer::new(out);
        let _ = rend.move_cursor(terminal_size.row, terminal_size.col);
        let _ = rend.down_cursor();
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TerminalSize {
    pub row: u16,
    pub col: u16,
}

/// returns size of row, col of terminal or something.
pub fn terminal_size() -> std::io::Result<TerminalSize> {
    let (row, col) = imp::terminal_size()?;
    Ok(TerminalSize { row, col })
}
