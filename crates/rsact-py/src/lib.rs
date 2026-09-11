//! PyO3 bindings for `rsact-core` — a native alternative to the pure-`ctypes`
//! wrapper in `crates/rsact-ffi/python`. Wraps `rsact-core` directly (no C
//! ABI indirection through `rsact-ffi`), so there's no hand-written FFI
//! signature to keep in sync with a header, and building an `Element` tree
//! from a `Text`/`Container` graph is done with plain, safe Rust — a
//! partially-built subtree on a later sibling's error is cleaned up by
//! Rust's own `Drop`, not by a hand-rolled leak-safety net.

use pyo3::exceptions::{PyOSError, PyRuntimeError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3_stub_gen::define_stub_info_gatherer;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pyfunction, gen_stub_pymethods};

use rsact_core::cell::{Cell, Color, Style};
use rsact_core::component::Component;
use rsact_core::element::{Element, Layout};
use rsact_core::input::{InputReader, Key};
use rsact_core::term::{RawModeGuard, TerminalSize};
use rsact_core::tree::Tree as CoreTree;
use rsact_core::{buffer::Buffer, diff, renderer::Renderer, term};

fn style_from(fg: u32, bg: u32, bold: bool, underline: bool, reverse: bool) -> Style {
    fn color(rgb: u32) -> Color {
        Color::Rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
    }
    Style {
        fg: color(fg),
        bg: color(bg),
        bold,
        underline,
        reverse,
    }
}

/// Returns `(rows, cols)` of the current terminal.
#[gen_stub_pyfunction]
#[pyfunction]
fn terminal_size() -> PyResult<(u16, u16)> {
    let size = term::terminal_size().map_err(|e| PyOSError::new_err(e.to_string()))?;
    Ok((size.row, size.col))
}

/// A decoded key. `char` is set for a printable character, `special` for
/// one of the `RSACT_KEY_*` constants — exactly one of the two.
#[gen_stub_pyclass]
#[pyclass(frozen, get_all)]
struct KeyEvent {
    char: Option<char>,
    special: Option<u32>,
}

#[gen_stub_pymethods]
#[pymethods]
impl KeyEvent {
    fn __repr__(&self) -> String {
        match self.char {
            Some(ch) => format!("KeyEvent(char={ch:?})"),
            None => format!("KeyEvent(special={:?})", self.special),
        }
    }
}

const RSACT_KEY_UP: u32 = 1;
const RSACT_KEY_DOWN: u32 = 2;
const RSACT_KEY_RIGHT: u32 = 3;
const RSACT_KEY_LEFT: u32 = 4;
const RSACT_KEY_ENTER: u32 = 5;
const RSACT_KEY_ESC: u32 = 6;
const RSACT_KEY_BACKSPACE: u32 = 7;
const RSACT_KEY_CTRL_BASE: u32 = 0x100;

fn key_event_from(key: Key) -> KeyEvent {
    match key {
        Key::Char(ch) => KeyEvent {
            char: Some(ch),
            special: None,
        },
        Key::Ctrl(ch) => KeyEvent {
            char: None,
            special: Some(RSACT_KEY_CTRL_BASE + (ch as u32 - 'a' as u32)),
        },
        Key::Enter => KeyEvent {
            char: None,
            special: Some(RSACT_KEY_ENTER),
        },
        Key::Esc => KeyEvent {
            char: None,
            special: Some(RSACT_KEY_ESC),
        },
        Key::Backspace => KeyEvent {
            char: None,
            special: Some(RSACT_KEY_BACKSPACE),
        },
        Key::Up => KeyEvent {
            char: None,
            special: Some(RSACT_KEY_UP),
        },
        Key::Down => KeyEvent {
            char: None,
            special: Some(RSACT_KEY_DOWN),
        },
        Key::Right => KeyEvent {
            char: None,
            special: Some(RSACT_KEY_RIGHT),
        },
        Key::Left => KeyEvent {
            char: None,
            special: Some(RSACT_KEY_LEFT),
        },
    }
}

/// The flat cell-buffer API: draw with `set_cell`, then `render` to
/// diff-and-flush just the changed cells to the terminal in one write.
///
/// `unsendable`: holds a `RawModeGuard` (owns the terminal's raw-mode
/// state via a `Box<dyn Write>`), which isn't `Send`/`Sync` — matches
/// `rsact-ffi`'s own documented constraint of one thread at a time.
#[gen_stub_pyclass]
#[pyclass(unsendable)]
struct Terminal {
    _rmg: Option<RawModeGuard>,
    virtual_dom: Buffer,
    real_dom: Buffer,
    renderer: Renderer<std::io::Stdout>,
    reader: InputReader<std::io::Stdin>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Terminal {
    #[new]
    fn new(width: u16, height: u16) -> PyResult<Self> {
        let size = TerminalSize {
            row: height,
            col: width,
        };
        let rmg = RawModeGuard::enable_safe_exit(size, std::io::stdout())
            .map_err(|_| PyRuntimeError::new_err("rsact_create failed (not a real terminal?)"))?;
        Ok(Self {
            _rmg: Some(rmg),
            virtual_dom: Buffer::new(width, height),
            real_dom: Buffer::new(width, height),
            renderer: Renderer::new(std::io::stdout()),
            reader: InputReader::new(std::io::stdin()),
        })
    }

    #[pyo3(signature = (row, col, ch, fg=0xFFFFFF, bg=0x000000, bold=false, underline=false, reverse=false))]
    #[allow(clippy::too_many_arguments)]
    fn set_cell(
        &mut self,
        row: u16,
        col: u16,
        ch: char,
        fg: u32,
        bg: u32,
        bold: bool,
        underline: bool,
        reverse: bool,
    ) {
        self.virtual_dom.set(
            row,
            col,
            Cell {
                ch,
                style: style_from(fg, bg, bold, underline, reverse),
            },
        );
    }

    fn render(&mut self) -> PyResult<()> {
        let patches = diff::diff(&self.real_dom, &self.virtual_dom);
        self.renderer
            .draw(&patches)
            .map_err(|e| PyOSError::new_err(e.to_string()))?;
        self.real_dom.clone_from(&self.virtual_dom);
        Ok(())
    }

    fn poll_key(&mut self) -> PyResult<Option<KeyEvent>> {
        let key = self
            .reader
            .read_key()
            .map_err(|e| PyOSError::new_err(e.to_string()))?;
        Ok(key.map(key_event_from))
    }

    /// Restores the terminal immediately, without waiting for this object
    /// to be garbage-collected. Not required for correctness — dropping
    /// (or letting the last reference go out of scope) does the same
    /// thing automatically, via Rust's own `Drop` on the `RawModeGuard`
    /// this holds — this is only for when you want that to happen at a
    /// specific point instead of whenever GC gets to it.
    fn close(&mut self) {
        self._rmg = None;
    }
}

/// A single-line text leaf. Immutable, plain data — build one with
/// `text()`, not directly.
#[gen_stub_pyclass]
#[pyclass(frozen, eq, get_all)]
#[derive(PartialEq)]
struct Text {
    key: String,
    content: String,
    width: u16,
    fg: u32,
    bg: u32,
    bold: bool,
    underline: bool,
    reverse: bool,
}

/// A container stacking `children` along `layout`'s axis
/// (`RSACT_LAYOUT_VERTICAL`/`_HORIZONTAL`). Immutable, plain data — build
/// one with `container()`, not directly.
#[gen_stub_pyclass]
#[pyclass(frozen, get_all)]
struct Container {
    key: String,
    layout: u8,
    children: Vec<Py<PyAny>>,
    width: u16,
    height: u16,
}

/// Builds a `Text` leaf.
#[gen_stub_pyfunction]
#[pyfunction]
#[pyo3(signature = (key, content, width=0, fg=0xFFFFFF, bg=0x000000, bold=false, underline=false, reverse=false))]
#[allow(clippy::too_many_arguments)]
fn text(
    key: String,
    content: String,
    width: u16,
    fg: u32,
    bg: u32,
    bold: bool,
    underline: bool,
    reverse: bool,
) -> Text {
    Text {
        key,
        content,
        width,
        fg,
        bg,
        bold,
        underline,
        reverse,
    }
}

/// Builds a `Container`, stacking `children` (a list of `text()`/
/// `container()` results) along `layout`'s axis.
#[gen_stub_pyfunction]
#[pyfunction]
#[pyo3(signature = (key, layout, children=Vec::new(), width=0, height=0))]
fn container(
    key: String,
    layout: u8,
    children: Vec<Py<PyAny>>,
    width: u16,
    height: u16,
) -> Container {
    Container {
        key,
        layout,
        children,
        width,
        height,
    }
}

/// Converts a `Text`/`Container` Python object graph into a native
/// `rsact_core::element::Element` tree with one recursive, safe-Rust walk.
/// Unlike the `ctypes` binding's `_compile`, a later sibling erroring here
/// needs no manual cleanup for earlier ones — they're plain owned `Element`
/// values, dropped automatically like any other Rust value on unwind.
fn compile_node(py: Python<'_>, node: &Bound<'_, PyAny>) -> PyResult<Element> {
    if let Ok(text_ref) = node.cast::<Text>() {
        let text = text_ref.get();
        let style = style_from(text.fg, text.bg, text.bold, text.underline, text.reverse);
        return Ok(Element::text(text.key.clone(), text.content.clone())
            .width(text.width)
            .style(style));
    }

    if let Ok(container_ref) = node.cast::<Container>() {
        let container = container_ref.get();
        let layout = match container.layout {
            0 => Layout::Vertical,
            1 => Layout::Horizontal,
            other => return Err(PyValueError::new_err(format!("invalid layout: {other}"))),
        };
        let children = container
            .children
            .iter()
            .map(|child| compile_node(py, child.bind(py)))
            .collect::<PyResult<Vec<_>>>()?;
        return Ok(Element::container(container.key.clone(), layout, children)
            .width(container.width)
            .height(container.height));
    }

    Err(PyTypeError::new_err(format!(
        "not a rsact.Text/rsact.Container node: {node:?}"
    )))
}

/// Wraps a Python-built `Element` so it can stand in as `CoreTree`'s root
/// `Component` — `render` just clones the currently-set tree.
struct PyElementComponent {
    element: Element,
}

impl Component for PyElementComponent {
    fn render(&self) -> Element {
        self.element.clone()
    }
}

/// The component-tree API: build a fresh `Text`/`Container` tree every
/// frame (plain, immutable data — see `text()`/`container()`) and call
/// `present`, which compiles it to native elements, reconciles it against
/// the previous frame, and draws only what changed.
///
/// `unsendable`: see `Terminal`'s doc comment — same reason.
#[gen_stub_pyclass]
#[pyclass(unsendable)]
struct Tree {
    _rmg: Option<RawModeGuard>,
    inner: CoreTree<PyElementComponent>,
    renderer: Renderer<std::io::Stdout>,
    reader: InputReader<std::io::Stdin>,
}

#[gen_stub_pymethods]
#[pymethods]
impl Tree {
    #[new]
    fn new(width: u16, height: u16) -> PyResult<Self> {
        let size = TerminalSize {
            row: height,
            col: width,
        };
        let rmg = RawModeGuard::enable_safe_exit(size, std::io::stdout()).map_err(|_| {
            PyRuntimeError::new_err("rsact_tree_create failed (not a real terminal?)")
        })?;
        let root = PyElementComponent {
            element: Element::container("root", Layout::Vertical, Vec::new()),
        };
        Ok(Self {
            _rmg: Some(rmg),
            inner: CoreTree::new(root, width, height),
            renderer: Renderer::new(std::io::stdout()),
            reader: InputReader::new(std::io::stdin()),
        })
    }

    fn present(&mut self, py: Python<'_>, node: Py<PyAny>) -> PyResult<()> {
        let element = compile_node(py, node.bind(py))?;
        self.inner.root_mut().element = element;
        self.inner
            .present(&mut self.renderer)
            .map_err(|e| PyOSError::new_err(e.to_string()))
    }

    fn poll_key(&mut self) -> PyResult<Option<KeyEvent>> {
        let key = self
            .reader
            .read_key()
            .map_err(|e| PyOSError::new_err(e.to_string()))?;
        Ok(key.map(key_event_from))
    }

    /// Restores the terminal immediately, without waiting for this object
    /// to be garbage-collected. Not required for correctness — dropping
    /// (or letting the last reference go out of scope) does the same
    /// thing automatically, via Rust's own `Drop` on the `RawModeGuard`
    /// this holds — this is only for when you want that to happen at a
    /// specific point instead of whenever GC gets to it.
    fn close(&mut self) {
        self._rmg = None;
    }
}

#[pymodule]
fn rsact(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(terminal_size, m)?)?;
    m.add_function(wrap_pyfunction!(text, m)?)?;
    m.add_function(wrap_pyfunction!(container, m)?)?;
    m.add_class::<Terminal>()?;
    m.add_class::<Tree>()?;
    m.add_class::<Text>()?;
    m.add_class::<Container>()?;
    m.add_class::<KeyEvent>()?;
    m.add("RSACT_LAYOUT_VERTICAL", 0u8)?;
    m.add("RSACT_LAYOUT_HORIZONTAL", 1u8)?;
    m.add("RSACT_KEY_UP", RSACT_KEY_UP)?;
    m.add("RSACT_KEY_DOWN", RSACT_KEY_DOWN)?;
    m.add("RSACT_KEY_RIGHT", RSACT_KEY_RIGHT)?;
    m.add("RSACT_KEY_LEFT", RSACT_KEY_LEFT)?;
    m.add("RSACT_KEY_ENTER", RSACT_KEY_ENTER)?;
    m.add("RSACT_KEY_ESC", RSACT_KEY_ESC)?;
    m.add("RSACT_KEY_BACKSPACE", RSACT_KEY_BACKSPACE)?;
    m.add("RSACT_KEY_CTRL_BASE", RSACT_KEY_CTRL_BASE)?;
    Ok(())
}

define_stub_info_gatherer!(stub_info);
// force rebuild 1789114385
