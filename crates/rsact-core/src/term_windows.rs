use windows_sys::Win32::System::Console;

// term_windows.rs — #[cfg(windows)]
pub(crate) struct RawModeGuard {
    stdin_mode: (std::os::windows::raw::HANDLE, u32),
    stdout_mode: (std::os::windows::raw::HANDLE, u32),
}
impl RawModeGuard {
    // GetConsoleMode/SetConsoleMode on stdin: clear ENABLE_ECHO_INPUT |
    // ENABLE_LINE_INPUT | ENABLE_PROCESSED_INPUT (so Ctrl-C arrives as byte
    // 0x03 instead of a CTRL_C_EVENT, matching Unix's ISIG-cleared raw mode),
    // set ENABLE_VIRTUAL_TERMINAL_INPUT (so arrow keys etc. arrive as the
    // same `ESC [ A`-style bytes InputReader already parses on Unix). On
    // stdout: set ENABLE_VIRTUAL_TERMINAL_PROCESSING. Restores both original
    // modes on Drop.
    pub fn enable() -> std::io::Result<Self> {
        unsafe {
            let hconsoleinput = Console::GetStdHandle(Console::STD_INPUT_HANDLE);
            let mut stdin_mode: Console::CONSOLE_MODE = 0;
            if Console::GetConsoleMode(hconsoleinput, &mut stdin_mode) == 0 {
                return Err(std::io::Error::last_os_error());
            }

            let hconsoleoutput = Console::GetStdHandle(Console::STD_OUTPUT_HANDLE);
            let mut stdout_mode: Console::CONSOLE_MODE = 0;
            if Console::GetConsoleMode(hconsoleoutput, &mut stdout_mode) == 0 {
                return Err(std::io::Error::last_os_error());
            }

            let raw_stdin_mode = (stdin_mode
                & !(Console::ENABLE_ECHO_INPUT
                    | Console::ENABLE_LINE_INPUT
                    | Console::ENABLE_PROCESSED_INPUT))
                | Console::ENABLE_VIRTUAL_TERMINAL_INPUT;
            if Console::SetConsoleMode(hconsoleinput, raw_stdin_mode) == 0 {
                return Err(std::io::Error::last_os_error());
            }

            let raw_stdout_mode = stdout_mode | Console::ENABLE_VIRTUAL_TERMINAL_PROCESSING;
            if Console::SetConsoleMode(hconsoleoutput, raw_stdout_mode) == 0 {
                return Err(std::io::Error::last_os_error());
            }

            Ok(Self {
                stdin_mode: (hconsoleinput, stdin_mode),
                stdout_mode: (hconsoleoutput, stdout_mode),
            })
        }
    }
}
impl Drop for RawModeGuard {
    fn drop(&mut self) {
        unsafe {
            Console::SetConsoleMode(self.stdin_mode.0, self.stdin_mode.1);
            Console::SetConsoleMode(self.stdout_mode.0, self.stdout_mode.1);
        }
    }
}
// GetConsoleScreenBufferInfo
pub(crate) fn terminal_size() -> std::io::Result<(u16, u16)> {
    let mut lpconsolescreenbufferinfo =
        unsafe { std::mem::zeroed::<Console::CONSOLE_SCREEN_BUFFER_INFO>() };
    let hconsoleoutput = unsafe { Console::GetStdHandle(Console::STD_OUTPUT_HANDLE) };
    unsafe {
        if Console::GetConsoleScreenBufferInfo(hconsoleoutput, &mut lpconsolescreenbufferinfo) == 0
        // WTF is wrong with this error code MS??
        {
            Err(std::io::Error::last_os_error())
        } else {
            let rows = lpconsolescreenbufferinfo.srWindow.Right
                - lpconsolescreenbufferinfo.srWindow.Left
                + 1;
            let cols = lpconsolescreenbufferinfo.srWindow.Bottom
                - lpconsolescreenbufferinfo.srWindow.Top
                + 1;
            Ok((i16_to_u16_relu(rows), i16_to_u16_relu(cols)))
        }
    }
}

fn i16_to_u16_relu(num: i16) -> u16 {
    if num <= 0 { 0 } else { num.try_into().unwrap() }
}
