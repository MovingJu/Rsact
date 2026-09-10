use rsact_core::{
    buffer::Buffer,
    cell::{Cell, Color, Style},
    component::Component,
    diff,
    element::{Element, ElementKind, Layout},
    input::{InputReader, Key},
    renderer::Renderer,
    term::{self, RawModeGuard, TerminalSize},
    tree::Tree,
};
use std::ffi::{CStr, c_char};

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

// --- Component tree -------------------------------------------------------
//
// C has no equivalent of the `Component` trait, so instead of rendering a
// tree from Rust state each frame, a C caller builds an `Element` tree
// directly with the functions below and hands the finished root over to a
// `RsactTreeHandle` each frame via `rsact_tree_set_root`. Internally this
// wraps the built `Element` in a trivial `Component` (`FfiComponent`) whose
// `render` just clones it, so it can drive the same `Tree` reconciler
// Rust callers use.

/// An opaque, owned `Element` (sub)tree, built up with the
/// `rsact_element_*` functions below. Passing one to
/// `rsact_element_add_child` or `rsact_tree_set_root` transfers ownership —
/// never touch or free it again afterward. An `Element` you built but never
/// attached anywhere must be freed with `rsact_element_free`.
#[repr(C)]
pub struct RsactElement {
    _private: [u8; 0],
}

/// # Safety
/// See `get_handle`'s safety notes — the same constraints apply here, with
/// `RsactElement` in place of `RsactHandle`.
unsafe fn get_element<'a>(elem: *mut RsactElement) -> Option<&'a mut Element> {
    if elem.is_null() {
        None
    } else {
        Some(unsafe { &mut *(elem as *mut Element) })
    }
}

/// # Safety
/// `ptr` must be either null or a valid, NUL-terminated, UTF-8 C string.
unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr) }.to_str().ok()
}

/// Creates a single-line text leaf element, width/height defaulting to
/// `0`/`1` (set width with `rsact_element_set_width`). Returns null if
/// `key` or `content` is null or not valid UTF-8.
///
/// # Safety
/// `key` and `content` must each be either null or a valid, NUL-terminated,
/// UTF-8 C string.
///
/// # Examples
/// ```rust,no_run
/// use rsact_ffi::*;
/// use std::ffi::CString;
///
/// unsafe {
///     let key = CString::new("label").unwrap();
///     let content = CString::new("hello").unwrap();
///     let elem = rsact_element_text(key.as_ptr(), content.as_ptr());
///     assert!(!elem.is_null());
///     rsact_element_free(elem);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rsact_element_text(
    key: *const c_char,
    content: *const c_char,
) -> *mut RsactElement {
    let Some(key) = (unsafe { cstr_to_str(key) }) else {
        return std::ptr::null_mut();
    };
    let Some(content) = (unsafe { cstr_to_str(content) }) else {
        return std::ptr::null_mut();
    };
    Box::into_raw(Box::new(Element::text(key, content))) as *mut RsactElement
}

/// Creates a container element with no children yet (add them with
/// `rsact_element_add_child`); width/height default to `0`.
/// `layout` is `0` for `Vertical`, `1` for `Horizontal`. Returns null if
/// `key` is null/not valid UTF-8, or `layout` isn't `0`/`1`.
///
/// # Safety
/// `key` must be either null or a valid, NUL-terminated, UTF-8 C string.
///
/// # Examples
/// ```rust,no_run
/// use rsact_ffi::*;
/// use std::ffi::CString;
///
/// unsafe {
///     let key = CString::new("root").unwrap();
///     let elem = rsact_element_container(key.as_ptr(), 0);
///     assert!(!elem.is_null());
///     rsact_element_free(elem);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rsact_element_container(
    key: *const c_char,
    layout: u8,
) -> *mut RsactElement {
    let Some(key) = (unsafe { cstr_to_str(key) }) else {
        return std::ptr::null_mut();
    };
    let layout = match layout {
        0 => Layout::Vertical,
        1 => Layout::Horizontal,
        _ => return std::ptr::null_mut(),
    };
    Box::into_raw(Box::new(Element::container(key, layout, Vec::new()))) as *mut RsactElement
}

/// Sets the extent `elem` asks its parent for along the parent's stack
/// axis. Returns `0` on success, `-1` if `elem` is null.
///
/// # Safety
/// `elem` must be a valid pointer returned by `rsact_element_text`/
/// `rsact_element_container` that hasn't yet been passed to
/// `rsact_element_add_child`, `rsact_tree_set_root`, or
/// `rsact_element_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rsact_element_set_width(elem: *mut RsactElement, width: u16) -> i32 {
    let Some(elem) = (unsafe { get_element(elem) }) else {
        return -1;
    };
    elem.width = width;
    0
}

/// Sets the extent `elem` asks its parent for along the parent's stack
/// axis. Returns `0` on success, `-1` if `elem` is null.
///
/// # Safety
/// Same as `rsact_element_set_width`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rsact_element_set_height(elem: *mut RsactElement, height: u16) -> i32 {
    let Some(elem) = (unsafe { get_element(elem) }) else {
        return -1;
    };
    elem.height = height;
    0
}

/// Sets the style of a `Text` leaf; same `fg_rgb`/`bg_rgb`/`attrs` encoding
/// as `rsact_set_cell`. Returns `0` on success, `-1` if `elem` is null,
/// `-2` if `elem` is a container (styling is a no-op there), `-3` if
/// `attrs` isn't one of the recognized sums.
///
/// # Safety
/// Same as `rsact_element_set_width`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rsact_element_set_style(
    elem: *mut RsactElement,
    fg_rgb: u32,
    bg_rgb: u32,
    attrs: u8,
) -> i32 {
    let Some(elem) = (unsafe { get_element(elem) }) else {
        return -1;
    };
    let Some((bold, underline, reverse)) = seperate_style(attrs) else {
        return -3;
    };
    match &mut elem.kind {
        ElementKind::Text(text) => {
            text.style = Style {
                fg: color_hex_to_struct(fg_rgb),
                bg: color_hex_to_struct(bg_rgb),
                bold,
                underline,
                reverse,
            };
            0
        }
        ElementKind::Container { .. } => -2,
    }
}

/// Appends `child` as the last child of `container`. Always consumes
/// `child` — never use or free it again after this call, whether or not it
/// succeeds. Returns `0` on success, `-1` if `container` or `child` is
/// null, `-2` if `container` is a `Text` leaf (it can't have children).
///
/// # Safety
/// `container` and `child` must each be either null or a valid pointer
/// returned by `rsact_element_text`/`rsact_element_container` that hasn't
/// yet been passed to `rsact_element_add_child`, `rsact_tree_set_root`, or
/// `rsact_element_free`. `container` and `child` must not be the same
/// pointer.
///
/// # Examples
/// ```rust,no_run
/// use rsact_ffi::*;
/// use std::ffi::CString;
///
/// unsafe {
///     let root_key = CString::new("root").unwrap();
///     let root = rsact_element_container(root_key.as_ptr(), 0);
///
///     let child_key = CString::new("child").unwrap();
///     let child_content = CString::new("hi").unwrap();
///     let child = rsact_element_text(child_key.as_ptr(), child_content.as_ptr());
///
///     assert_eq!(rsact_element_add_child(root, child), 0);
///     rsact_element_free(root); // also frees the attached child
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rsact_element_add_child(
    container: *mut RsactElement,
    child: *mut RsactElement,
) -> i32 {
    if child.is_null() {
        return -1;
    }
    // Always take ownership of `child`, even on failure below, so a caller
    // never has to guess whether it still needs freeing.
    let child = *unsafe { Box::from_raw(child as *mut Element) };
    let Some(container) = (unsafe { get_element(container) }) else {
        return -1;
    };
    match &mut container.kind {
        ElementKind::Container { children, .. } => {
            children.push(child);
            0
        }
        ElementKind::Text(_) => -2,
    }
}

/// Frees an element (sub)tree that was never attached via
/// `rsact_element_add_child` or `rsact_tree_set_root`. Does nothing if
/// `elem` is null.
///
/// # Safety
/// `elem` must be either null or a valid pointer returned by
/// `rsact_element_text`/`rsact_element_container` that hasn't already been
/// passed to `rsact_element_add_child`, `rsact_tree_set_root`, or
/// `rsact_element_free`. Never call this twice on the same pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rsact_element_free(elem: *mut RsactElement) {
    if elem.is_null() {
        return;
    }
    drop(unsafe { Box::from_raw(elem as *mut Element) });
}

/// Wraps a caller-built `Element` so it can stand in as the `Tree`'s root
/// `Component` — `render` just clones the currently-set tree.
struct FfiComponent {
    element: Element,
}

impl Component for FfiComponent {
    fn render(&self) -> Element {
        self.element.clone()
    }
}

/// An opaque handle owning a `Tree`, its own raw-mode guard, renderer, and
/// input reader — the component-tree equivalent of `RsactHandle`.
#[repr(C)]
pub struct RsactTreeHandle {
    _private: [u8; 0],
}

struct RsactTreeHandleInner {
    _rmg: RawModeGuard,
    tree: Tree<FfiComponent>,
    renderer: Renderer<std::io::Stdout>,
    reader: InputReader<std::io::Stdin>,
}

/// # Safety
/// Same constraints as `get_handle`, with `RsactTreeHandle` in place of
/// `RsactHandle`.
unsafe fn get_tree_handle<'a>(
    handle: *mut RsactTreeHandle,
) -> Option<&'a mut RsactTreeHandleInner> {
    if handle.is_null() {
        None
    } else {
        Some(unsafe { &mut *(handle as *mut RsactTreeHandleInner) })
    }
}

/// Creates a handle for a `width`×`height` component tree and enters raw
/// mode. The tree starts with an empty root container — set real content
/// with `rsact_tree_set_root` before the first `rsact_tree_present`.
/// Returns a null pointer if raw mode couldn't be enabled (for example,
/// when stdin/stdout isn't a real terminal).
///
/// # Safety
/// Same as `rsact_create`.
///
/// # Examples
/// ```rust,no_run
/// use rsact_ffi::*;
///
/// unsafe {
///     let handle = rsact_tree_create(40, 2);
///     assert!(!handle.is_null());
///     rsact_tree_destroy(handle);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rsact_tree_create(width: u16, height: u16) -> *mut RsactTreeHandle {
    let terminal_size = TerminalSize {
        row: height,
        col: width,
    };
    let rmg = match RawModeGuard::enable_safe_exit(terminal_size, std::io::stdout()) {
        Ok(rmg) => rmg,
        Err(_) => return std::ptr::null_mut(),
    };
    let root = FfiComponent {
        element: Element::container("root", Layout::Vertical, Vec::new())
            .width(width)
            .height(height),
    };
    let handle_inner = Box::new(RsactTreeHandleInner {
        _rmg: rmg,
        tree: Tree::new(root, width, height),
        renderer: Renderer::new(std::io::stdout()),
        reader: InputReader::new(std::io::stdin()),
    });
    Box::into_raw(handle_inner) as *mut RsactTreeHandle
}

/// Replaces `handle`'s tree root with `element`, taking ownership of it —
/// never use or free `element` again after this call. Takes effect on the
/// next `rsact_tree_present`. Returns `0` on success, `-1` if `handle` is
/// null, `-2` if `element` is null.
///
/// # Safety
/// `handle` must be either null or a valid pointer returned by
/// `rsact_tree_create` that hasn't been passed to `rsact_tree_destroy` yet.
/// `element` must be either null or a valid pointer returned by
/// `rsact_element_text`/`rsact_element_container` that hasn't yet been
/// passed to `rsact_element_add_child`, `rsact_tree_set_root`, or
/// `rsact_element_free`.
///
/// # Examples
/// ```rust,no_run
/// use rsact_ffi::*;
/// use std::ffi::CString;
///
/// unsafe {
///     let handle = rsact_tree_create(20, 1);
///     let key = CString::new("greeting").unwrap();
///     let content = CString::new("hi").unwrap();
///     let root = rsact_element_text(key.as_ptr(), content.as_ptr());
///     assert_eq!(rsact_tree_set_root(handle, root), 0);
///     assert_eq!(rsact_tree_present(handle), 0);
///     rsact_tree_destroy(handle);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rsact_tree_set_root(
    handle: *mut RsactTreeHandle,
    element: *mut RsactElement,
) -> i32 {
    if element.is_null() {
        return -2;
    }
    let element = *unsafe { Box::from_raw(element as *mut Element) };
    let Some(handle) = (unsafe { get_tree_handle(handle) }) else {
        return -1;
    };
    handle.tree.root_mut().element = element;
    0
}

/// Renders the current root, reconciles it against the previous frame,
/// diffs the result against what's actually on screen, and draws only that
/// patch set in a single write — see `Tree::present` in `rsact-core`.
/// Returns `0` on success, `-1` if `handle` is null or the write failed.
///
/// # Safety
/// `handle` must be either null or a valid pointer returned by
/// `rsact_tree_create` that hasn't been passed to `rsact_tree_destroy` yet.
///
/// # Examples
/// ```rust,no_run
/// use rsact_ffi::*;
///
/// unsafe {
///     let handle = rsact_tree_create(20, 1);
///     let rc = rsact_tree_present(handle);
///     assert_eq!(rc, 0);
///     rsact_tree_destroy(handle);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rsact_tree_present(handle: *mut RsactTreeHandle) -> i32 {
    let Some(handle) = (unsafe { get_tree_handle(handle) }) else {
        return -1;
    };
    match handle.tree.present(&mut handle.renderer) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

/// Reads and decodes the next key from stdin into `out_event`, identical to
/// `rsact_poll_key` but for a `RsactTreeHandle`. Blocks until a key
/// arrives. Returns `1` and fills `out_event` when a key was read, `0` on
/// EOF, `-1`/`-2` on a null handle/read error.
///
/// # Safety
/// Same as `rsact_poll_key`, with `RsactTreeHandle` in place of
/// `RsactHandle`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rsact_tree_poll_key(
    handle: *mut RsactTreeHandle,
    out_event: *mut RsactKeyEvent,
) -> i32 {
    let Some(handle) = (unsafe { get_tree_handle(handle) }) else {
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
}

/// Destroys a handle created by `rsact_tree_create`, restoring the
/// terminal. Does nothing if `handle` is null.
///
/// # Safety
/// `handle` must be either null or a pointer previously returned by
/// `rsact_tree_create` that hasn't already been passed to
/// `rsact_tree_destroy`. Never call this twice on the same pointer, and
/// never use `handle` again afterward.
///
/// # Examples
/// ```rust,no_run
/// use rsact_ffi::*;
///
/// unsafe {
///     let handle = rsact_tree_create(20, 1);
///     rsact_tree_destroy(handle);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rsact_tree_destroy(handle: *mut RsactTreeHandle) {
    if handle.is_null() {
        return;
    }
    drop(unsafe { Box::from_raw(handle as *mut RsactTreeHandleInner) });
}
