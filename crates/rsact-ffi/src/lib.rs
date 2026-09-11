use rsact_core::{
    buffer::Buffer,
    cell::{Cell, Color, Style},
    component::Component,
    diff,
    element::{Element, Layout},
    input::{InputReader, Key},
    renderer::Renderer,
    term::{self, RawModeGuard, TerminalSize},
    tree::Tree,
};
use std::ffi::{CStr, c_char};

/// The raw-mode guard, renderer, and input reader every handle needs —
/// factored out so `RsactHandle` and `RsactTreeHandle` share the setup
/// instead of each re-entering raw mode and re-building a `Renderer`/
/// `InputReader` themselves.
struct RawTerminal {
    _rmg: RawModeGuard,
    renderer: Renderer<std::io::Stdout>,
    reader: InputReader<std::io::Stdin>,
}

impl RawTerminal {
    /// Enters raw mode for a `width`×`height` terminal. Returns `None` if
    /// raw mode couldn't be enabled (for example, when stdin/stdout isn't a
    /// real terminal).
    fn enter(width: u16, height: u16) -> Option<Self> {
        let terminal_size = TerminalSize {
            row: height,
            col: width,
        };
        let rmg = RawModeGuard::enable_safe_exit(terminal_size, std::io::stdout()).ok()?;
        Some(Self {
            _rmg: rmg,
            renderer: Renderer::new(std::io::stdout()),
            reader: InputReader::new(std::io::stdin()),
        })
    }
}

/// An opaque handle for the flat-buffer API (`rsact_set_cell`/
/// `rsact_render`). A distinct type from `RsactTreeHandle` on purpose: the
/// two APIs each track their own independent "what's actually on screen"
/// baseline, so a handle from one can't be passed to the other's
/// functions — the C compiler rejects that as an incompatible pointer type
/// instead of it silently producing incorrect repaints at runtime.
#[repr(C)]
pub struct RsactHandle {
    _private: [u8; 0],
}

struct RsactHandleInner {
    term: RawTerminal,
    virtual_dom: Buffer,
    real_dom: Buffer,
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
    let Some(term) = RawTerminal::enter(width, height) else {
        return std::ptr::null_mut();
    };
    let handle_inner = Box::new(RsactHandleInner {
        term,
        virtual_dom: Buffer::new(width, height),
        real_dom: Buffer::new(width, height),
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
    match handle.term.renderer.draw(&diffs) {
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
    let key = match handle.term.reader.read_key() {
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
// `RsactTreeHandle` each frame via `rsact_tree_present`. Internally this
// wraps the built `Element` in a trivial `Component` (`FfiComponent`) whose
// `render` just clones it, so it can drive the same `Tree` reconciler Rust
// callers use.

/// An opaque, owned `Element` (sub)tree, built up with the
/// `rsact_element_*` functions below. Passing one to `rsact_element_container`
/// (as one of its `children`) or `rsact_tree_present` transfers ownership —
/// never touch or free it again afterward. An `Element` you built but never
/// attached anywhere must be freed with `rsact_element_free`.
#[repr(C)]
pub struct RsactElement {
    _private: [u8; 0],
}

/// `layout` value for `rsact_element_container`: stack children top to
/// bottom, each spanning the container's full width.
pub const RSACT_LAYOUT_VERTICAL: u8 = 0;
/// `layout` value for `rsact_element_container`: stack children left to
/// right, each spanning the container's full height.
pub const RSACT_LAYOUT_HORIZONTAL: u8 = 1;

/// # Safety
/// `ptr` must be either null or a valid, NUL-terminated, UTF-8 C string.
unsafe fn cstr_to_str<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr) }.to_str().ok()
}

/// Creates a single-line text leaf element with its width and style set up
/// front, so it's ready to nest straight into a `rsact_element_container`
/// call — no separate setter calls needed. `fg_rgb`/`bg_rgb`/`attrs` use the
/// same encoding as `rsact_set_cell`. Returns null if `key`/`content` is
/// null or not valid UTF-8, or `attrs` isn't one of the recognized sums.
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
///     let elem = rsact_element_text(key.as_ptr(), content.as_ptr(), 20, 0xffffff, 0x000000, 0);
///     assert!(!elem.is_null());
///     rsact_element_free(elem);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rsact_element_text(
    key: *const c_char,
    content: *const c_char,
    width: u16,
    fg_rgb: u32,
    bg_rgb: u32,
    attrs: u8,
) -> *mut RsactElement {
    let Some(key) = (unsafe { cstr_to_str(key) }) else {
        return std::ptr::null_mut();
    };
    let Some(content) = (unsafe { cstr_to_str(content) }) else {
        return std::ptr::null_mut();
    };
    let Some((bold, underline, reverse)) = seperate_style(attrs) else {
        return std::ptr::null_mut();
    };
    let element = Element::text(key, content).width(width).style(Style {
        fg: color_hex_to_struct(fg_rgb),
        bg: color_hex_to_struct(bg_rgb),
        bold,
        underline,
        reverse,
    });
    Box::into_raw(Box::new(element)) as *mut RsactElement
}

/// Creates a container with `children` already attached, so a whole subtree
/// can be built as one nested expression — pass a C99 compound literal
/// array of `rsact_element_text`/`rsact_element_container` calls directly
/// as `children` (or a heap-allocated array, for a runtime-determined
/// count), instead of building each child as a separate named variable and
/// wiring it in afterward:
///
/// ```c
/// RsactElement *row = rsact_element_container(
///     "row", RSACT_LAYOUT_HORIZONTAL, 40, 1,
///     (RsactElement *[]){
///         rsact_element_text("label", "left", 20, 0xffffff, 0, 0),
///         rsact_element_text("value", "0",    20, 0xffffff, 0, 0),
///     }, 2);
/// ```
///
/// Always consumes every non-null pointer in `children` — never use or free
/// any of them again after this call, whether or not it succeeds.
/// `layout` is `RSACT_LAYOUT_VERTICAL` or `RSACT_LAYOUT_HORIZONTAL`. Returns
/// null if `key` is null/not valid UTF-8, `layout` isn't one of those two
/// values, or any pointer in `children` is null.
///
/// # Safety
/// `key` must be either null or a valid, NUL-terminated, UTF-8 C string.
/// `children` must be either null (with `n_children == 0`) or point to an
/// array of exactly `n_children` valid `*mut RsactElement` pointers, each
/// meeting the pointer requirements of `rsact_element_free` — and no two of
/// them (including nested descendants already attached to one of them) may
/// alias each other.
///
/// # Examples
/// ```rust,no_run
/// use rsact_ffi::*;
/// use std::ffi::CString;
///
/// unsafe {
///     let child_key = CString::new("child").unwrap();
///     let child_content = CString::new("hi").unwrap();
///     let child = rsact_element_text(child_key.as_ptr(), child_content.as_ptr(), 10, 0xffffff, 0, 0);
///
///     let root_key = CString::new("root").unwrap();
///     let children = [child];
///     let root = rsact_element_container(root_key.as_ptr(), RSACT_LAYOUT_VERTICAL, 10, 1, children.as_ptr(), children.len());
///     assert!(!root.is_null());
///     rsact_element_free(root); // also frees the attached child
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rsact_element_container(
    key: *const c_char,
    layout: u8,
    width: u16,
    height: u16,
    children: *const *mut RsactElement,
    n_children: usize,
) -> *mut RsactElement {
    // Always take ownership of every non-null child pointer first, even on
    // a failure below, so a caller never has to guess which ones (if any)
    // still need freeing themselves.
    let mut owned_children = Vec::with_capacity(n_children);
    let mut saw_null_child = false;
    for i in 0..n_children {
        let child_ptr = if children.is_null() {
            std::ptr::null_mut()
        } else {
            unsafe { *children.add(i) }
        };
        if child_ptr.is_null() {
            saw_null_child = true;
            continue;
        }
        owned_children.push(*unsafe { Box::from_raw(child_ptr as *mut Element) });
    }
    if saw_null_child {
        return std::ptr::null_mut();
    }

    let Some(key) = (unsafe { cstr_to_str(key) }) else {
        return std::ptr::null_mut();
    };
    let layout = match layout {
        RSACT_LAYOUT_VERTICAL => Layout::Vertical,
        RSACT_LAYOUT_HORIZONTAL => Layout::Horizontal,
        _ => return std::ptr::null_mut(),
    };
    let element = Element::container(key, layout, owned_children)
        .width(width)
        .height(height);
    Box::into_raw(Box::new(element)) as *mut RsactElement
}

/// Frees an element (sub)tree that was never attached to a
/// `rsact_element_container` call or `rsact_tree_present`. Does nothing if
/// `elem` is null.
///
/// # Safety
/// `elem` must be either null or a valid pointer returned by
/// `rsact_element_text`/`rsact_element_container` that hasn't already been
/// passed as a child to `rsact_element_container`, to `rsact_tree_present`,
/// or to `rsact_element_free`. Never call this twice on the same pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rsact_element_free(elem: *mut RsactElement) {
    if elem.is_null() {
        return;
    }
    drop(unsafe { Box::from_raw(elem as *mut Element) });
}

/// Wraps a caller-built `Element` so it can stand in as a `Tree`'s root
/// `Component` — `render` just clones the currently-set tree.
struct FfiComponent {
    element: Element,
}

impl Component for FfiComponent {
    fn render(&self) -> Element {
        self.element.clone()
    }
}

/// An opaque handle for the component-tree API (`rsact_tree_present`). A
/// distinct type from `RsactHandle` on purpose — see `RsactHandle`'s docs.
#[repr(C)]
pub struct RsactTreeHandle {
    _private: [u8; 0],
}

struct RsactTreeHandleInner {
    term: RawTerminal,
    tree: Tree<FfiComponent>,
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
/// mode. The tree starts with an empty root container — give it real
/// content with the first `rsact_tree_present` call. Returns a null
/// pointer if raw mode couldn't be enabled (for example, when
/// stdin/stdout isn't a real terminal).
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
    let Some(term) = RawTerminal::enter(width, height) else {
        return std::ptr::null_mut();
    };
    let root = FfiComponent {
        element: Element::container("root", Layout::Vertical, Vec::new()),
    };
    let handle_inner = Box::new(RsactTreeHandleInner {
        term,
        tree: Tree::new(root, width, height),
    });
    Box::into_raw(handle_inner) as *mut RsactTreeHandle
}

/// Replaces `handle`'s tree root with `element` (taking ownership of it —
/// never use or free `element` again after this call), reconciles it
/// against the previous frame, diffs the result against what's actually on
/// screen, and draws only that patch set in a single write — see
/// `Tree::present` in `rsact-core`. Returns `0` on success, `-1` if
/// `handle` is null or the write failed, `-2` if `element` is null.
///
/// # Safety
/// `handle` must be either null or a valid pointer returned by
/// `rsact_tree_create` that hasn't been passed to `rsact_tree_destroy` yet.
/// `element` must be either null or a valid pointer returned by
/// `rsact_element_text`/`rsact_element_container` that hasn't yet been
/// passed as a child to `rsact_element_container`, to another
/// `rsact_tree_present` call, or to `rsact_element_free`.
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
///     let root = rsact_element_text(key.as_ptr(), content.as_ptr(), 10, 0xffffff, 0, 0);
///     assert_eq!(rsact_tree_present(handle, root), 0);
///     rsact_tree_destroy(handle);
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rsact_tree_present(
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
    match handle.tree.present(&mut handle.term.renderer) {
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
    let key = match handle.term.reader.read_key() {
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
