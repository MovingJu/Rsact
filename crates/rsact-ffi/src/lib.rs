use rsact_core::{
    buffer::Buffer,
    cell::{Cell, Color, Style},
    diff,
    input::{InputReader, Key},
    renderer::Renderer,
    term::{self, RawModeGuard, TerminalSize},
};

#[repr(C)]
pub struct RsactHandle {
    _private: [u8; 0],
}

struct RsactHandleInner {
    _rmg: RawModeGuard,
    virtual_dom: Buffer,
    real_dom: Buffer,
    renderer: Renderer<std::io::Stdout>,
    reader: InputReader<std::io::Stdin>,
}

/// # Safety
/// No `rsact_destroy` while using handles from this function.
///
/// And there shouldn't exist **multiple** `&mut RsactHandleInner`.
unsafe fn get_handle<'a>(handle: *mut RsactHandle) -> Option<&'a mut RsactHandleInner> {
    if handle.is_null() {
        None
    } else {
        Some(unsafe { &mut *(handle as *mut RsactHandleInner) })
    }
}

/// Creates a handle for a `width`×`height` virtual terminal buffer and
/// enters raw mode. Returns a null pointer if raw mode couldn't be enabled
/// (for example, when stdin/stdout isn't a real terminal).
///
/// # Safety
/// No preconditions on the arguments themselves. Call this only from a
/// single thread at a time — v0.1 has no synchronization around the
/// process's stdin/stdout raw-mode state.
///
/// # Examples
/// ```rust,no_run
/// use rsact_ffi::*;
///
/// unsafe {
///     let handle = rsact_create(80, 24);
///     assert!(!handle.is_null());
///     rsact_destroy(handle);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rsact_create(width: u16, height: u16) -> *mut RsactHandle {
    let terminal_size = TerminalSize {
        row: height,
        col: width,
    };
    let rmg = match RawModeGuard::enable_safe_exit(terminal_size, std::io::stdout()) {
        Ok(rmg) => rmg,
        Err(_) => return std::ptr::null_mut(),
    };
    let handle_inner = Box::new(RsactHandleInner {
        _rmg: rmg,
        virtual_dom: Buffer::new(terminal_size.col, terminal_size.row),
        real_dom: Buffer::new(terminal_size.col, terminal_size.row),
        renderer: Renderer::new(std::io::stdout()),
        reader: InputReader::new(std::io::stdin()),
    });
    Box::into_raw(handle_inner) as *mut RsactHandle
}

/// Returns total size of current terminal. If succesfully got the size, it returns `0`. Otherwise, returns (< 0)
///
/// # Examples
/// ```rust,no_run
/// use rsact_ffi::*;
///
/// let mut row: u16 = 0;
/// let mut col: u16 = 0;
/// rsact_terminal_size(&mut row, &mut col);
/// ```
#[unsafe(no_mangle)]
pub extern "C" fn rsact_terminal_size(row: &mut u16, col: &mut u16) -> i32 {
    let terminal_size = match term::terminal_size() {
        Ok(val) => val,
        Err(_) => return -1,
    };
    *row = terminal_size.row;
    *col = terminal_size.col;
    0
}

/// Destroys a handle created by [`rsact_create`], restoring the terminal
/// (dropping the handle drops its `RawModeGuard`). Does nothing if `handle`
/// is null.
///
/// # Safety
/// `handle` must be either null or a pointer previously returned by
/// [`rsact_create`] that hasn't already been passed to `rsact_destroy`.
/// Never call this twice on the same pointer, and never use `handle` again
/// afterward.
///
/// # Examples
/// ```rust,no_run
/// use rsact_ffi::*;
///
/// unsafe {
///     let handle = rsact_create(80, 24);
///     rsact_destroy(handle);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rsact_destroy(handle: *mut RsactHandle) {
    if handle.is_null() {
        return;
    }
    drop(unsafe { Box::from_raw(handle as *mut RsactHandleInner) });
}

/// Sets a single cell of the virtual buffer. `fg_rgb`/`bg_rgb` are packed as
/// `0xRRGGBB`. `attrs` is the *sum* of the ANSI SGR codes to apply — `1` for
/// bold, `4` for underline, `7` for reverse, added together for combinations
/// (e.g. `5` = bold + underline, `12` = all three); any other value is
/// rejected. Returns `0` on success, `-1` if `handle` is null, `-2` if `ch`
/// isn't a valid Unicode scalar value, `-3` if `attrs` isn't one of the
/// recognized sums.
///
/// # Safety
/// `handle` must be either null or a valid pointer returned by
/// [`rsact_create`] that hasn't been passed to [`rsact_destroy`] yet.
///
/// # Examples
/// ```rust,no_run
/// use rsact_ffi::*;
///
/// unsafe {
///     let handle = rsact_create(80, 24);
///     let rc = rsact_set_cell(handle, 0, 0, 'A' as u32, 0xffffff, 0x000000, 0);
///     assert_eq!(rc, 0);
///     rsact_destroy(handle);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rsact_set_cell(
    handle: *mut RsactHandle,
    row: u16,
    col: u16,
    ch: u32,
    fg_rgb: u32,
    bg_rgb: u32,
    attrs: u8,
) -> i32 {
    let Some(handle) = (unsafe { get_handle(handle) }) else {
        return -1;
    };
    let Some(ch) = char::from_u32(ch) else {
        return -2;
    };
    let Some((bold, underline, reverse)) = seperate_style(attrs) else {
        return -3;
    };
    handle.virtual_dom.set(
        row,
        col,
        Cell {
            ch,
            style: Style {
                fg: color_hex_to_struct(fg_rgb),
                bg: color_hex_to_struct(bg_rgb),
                bold,
                underline,
                reverse,
            },
        },
    );
    0
}

fn seperate_style(style: u8) -> Option<(bool, bool, bool)> {
    match style {
        0 => Some((false, false, false)),
        1 => Some((true, false, false)),
        4 => Some((false, true, false)),
        7 => Some((false, false, true)),

        5 => Some((true, true, false)),
        8 => Some((true, false, true)),
        11 => Some((false, true, true)),
        12 => Some((true, true, true)),

        _ => None,
    }
}

fn color_hex_to_struct(color: u32) -> Color {
    let r = (color >> 16) as u8;
    let g = (color >> 8) as u8;
    let b = color as u8;
    Color::Rgb(r, g, b)
}

/// Diffs the virtual buffer against the last-rendered buffer, writes only
/// the changed cells to the terminal in a single write, then commits the
/// virtual buffer as the new baseline for the next call. Returns `0` on
/// success, `-1` if `handle` is null or the write failed.
///
/// # Safety
/// `handle` must be either null or a valid pointer returned by
/// [`rsact_create`] that hasn't been passed to [`rsact_destroy`] yet.
///
/// # Examples
/// ```rust,no_run
/// use rsact_ffi::*;
///
/// unsafe {
///     let handle = rsact_create(80, 24);
///     rsact_set_cell(handle, 0, 0, 'A' as u32, 0xffffff, 0x000000, 0);
///     let rc = rsact_render(handle);
///     assert_eq!(rc, 0);
///     rsact_destroy(handle);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rsact_render(handle: *mut RsactHandle) -> i32 {
    let Some(handle) = (unsafe { get_handle(handle) }) else {
        return -1;
    };
    let diffs = diff::diff(&handle.real_dom, &handle.virtual_dom);
    match handle.renderer.draw(&diffs) {
        Ok(_) => {}
        Err(_) => return -1,
    }
    handle.real_dom = handle.virtual_dom.clone();
    0
}

#[repr(C)]
pub struct RsactKeyEvent {
    pub kind: u8,  // 0 = none, 1 = char, 2 = special (see RSACT_KEY_* constants)
    pub code: u32, // unicode scalar value for kind=1, key id for kind=2
}

/// Reads and decodes the next key from stdin into `out_event`. Blocks until
/// a key arrives — v0.1 has no non-blocking mode. Returns `1` and fills
/// `out_event` when a key was read, `0` on EOF, `-1` on error or a null
/// `handle`.
///
/// # Safety
/// `handle` must be either null or a valid pointer returned by
/// [`rsact_create`] that hasn't been passed to [`rsact_destroy`] yet.
/// `out_event` must be a valid, non-null, properly aligned pointer to
/// writable memory for a [`RsactKeyEvent`] — it is not currently
/// null-checked, unlike `handle`.
///
/// # Examples
/// ```rust,no_run
/// use rsact_ffi::*;
/// use std::mem::MaybeUninit;
///
/// unsafe {
///     let handle = rsact_create(80, 24);
///     let mut event = MaybeUninit::<RsactKeyEvent>::uninit();
///     if rsact_poll_key(handle, event.as_mut_ptr()) == 1 {
///         let event = event.assume_init();
///         println!("kind={} code={}", event.kind, event.code);
///     }
///     rsact_destroy(handle);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rsact_poll_key(
    handle: *mut RsactHandle,
    out_event: *mut RsactKeyEvent,
) -> i32 {
    let Some(handle) = (unsafe { get_handle(handle) }) else {
        return -1;
    };
    let key = match handle.reader.read_key() {
        Ok(key) => key,
        Err(_) => return -2,
    };
    let key = match key {
        Some(val) => val,
        None => return 0, // EOF
    };
    unsafe {
        *out_event = key_to_struct(key);
    }
    1
} // 0 = no event, 1 = event written, (< 0) = error
fn key_to_struct(key: Key) -> RsactKeyEvent {
    match key {
        Key::Char(ch) => key_char_to_struct(ch),
        Key::Ctrl(ch) => key_ctrl_to_struct(ch),
        Key::Enter => RsactKeyEvent::new(2, RSACT_KEY_ENTER),
        Key::Esc => RsactKeyEvent::new(2, RSACT_KEY_ESC),
        Key::Backspace => RsactKeyEvent::new(2, RSACT_KEY_BACKSPACE),
        Key::Up => RsactKeyEvent::new(2, RSACT_KEY_UP),
        Key::Down => RsactKeyEvent::new(2, RSACT_KEY_DOWN),
        Key::Right => RsactKeyEvent::new(2, RSACT_KEY_RIGHT),
        Key::Left => RsactKeyEvent::new(2, RSACT_KEY_LEFT),
    }
}
fn key_char_to_struct(ch: char) -> RsactKeyEvent {
    RsactKeyEvent::new(1, ch as u32)
}
fn key_ctrl_to_struct(ch: char) -> RsactKeyEvent {
    RsactKeyEvent::new(2, RSACT_KEY_CTRL_BASE + (ch as u32 - 'a' as u32))
}

pub const RSACT_KEY_UP: u32 = 1;
pub const RSACT_KEY_DOWN: u32 = 2;
pub const RSACT_KEY_RIGHT: u32 = 3;
pub const RSACT_KEY_LEFT: u32 = 4;
pub const RSACT_KEY_ENTER: u32 = 5;
pub const RSACT_KEY_ESC: u32 = 6;
pub const RSACT_KEY_BACKSPACE: u32 = 7;
/// Ctrl+'a'..'z' -> RSACT_KEY_CTRL_BASE + (letter - 'a')
pub const RSACT_KEY_CTRL_BASE: u32 = 0x100;

impl RsactKeyEvent {
    fn new(kind: u8, code: u32) -> Self {
        Self { kind, code }
    }
}
