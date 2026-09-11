"""Pythonic ctypes bindings for rsact-ffi.

This is a pure-Python `ctypes` wrapper around `rsact-ffi`'s C ABI — no
compiled extension, no build step of its own. It loads the native
`rsact_ffi` *shared* library (`librsact_ffi.so`/`.dylib`/`rsact_ffi.dll`)
at import time; see `_find_library` below for how it's located.

Two independent APIs are exposed, mirroring `rsact-ffi` itself:

- `Terminal` — the flat cell-buffer API (`rsact_create`/`set_cell`/`render`).
- `Element` + `Tree` — the declarative component-tree API
  (`rsact_element_*`/`rsact_tree_*`).

As in the C API, don't mix the two on work meant to share a screen: each
tracks its own independent "what's actually on screen" baseline.
"""

from __future__ import annotations

import ctypes
import ctypes.util
import os
import platform

__all__ = [
    "Terminal",
    "Element",
    "Tree",
    "KeyEvent",
    "terminal_size",
    "RSACT_LAYOUT_VERTICAL",
    "RSACT_LAYOUT_HORIZONTAL",
    "RSACT_KEY_UP",
    "RSACT_KEY_DOWN",
    "RSACT_KEY_RIGHT",
    "RSACT_KEY_LEFT",
    "RSACT_KEY_ENTER",
    "RSACT_KEY_ESC",
    "RSACT_KEY_BACKSPACE",
    "RSACT_KEY_CTRL_BASE",
]

RSACT_LAYOUT_VERTICAL = 0
RSACT_LAYOUT_HORIZONTAL = 1

RSACT_KEY_UP = 1
RSACT_KEY_DOWN = 2
RSACT_KEY_RIGHT = 3
RSACT_KEY_LEFT = 4
RSACT_KEY_ENTER = 5
RSACT_KEY_ESC = 6
RSACT_KEY_BACKSPACE = 7
# Ctrl+'a'..'z' -> RSACT_KEY_CTRL_BASE + (ord(letter) - ord('a'))
RSACT_KEY_CTRL_BASE = 0x100


def _find_library() -> str:
    """Locates the `rsact_ffi` shared library.

    Checks, in order: the `RSACT_FFI_LIB` environment variable (an exact
    path); `target/release/<libname>` relative to the repo root (for
    running straight out of a checkout after `cargo build --release -p
    rsact-ffi`); then the system library search path via
    `ctypes.util.find_library`.
    """
    override = os.environ.get("RSACT_FFI_LIB")
    if override:
        return override

    system = platform.system()
    names = {
        "Linux": "librsact_ffi.so",
        "Darwin": "librsact_ffi.dylib",
        "Windows": "rsact_ffi.dll",
    }
    name = names.get(system)
    if name is None:
        raise RuntimeError(f"rsact: unsupported platform {system!r}")

    here = os.path.dirname(os.path.abspath(__file__))
    dev_candidate = os.path.normpath(os.path.join(here, "..", "..", "target", "release", name))
    if os.path.exists(dev_candidate):
        return dev_candidate

    found = ctypes.util.find_library("rsact_ffi")
    if found:
        return found

    raise RuntimeError(
        "rsact: could not locate the rsact-ffi shared library. Build it "
        "with `cargo build --release -p rsact-ffi` from the repo root, "
        "or set the RSACT_FFI_LIB environment variable to its path."
    )


_lib = ctypes.CDLL(_find_library())


class RsactKeyEvent(ctypes.Structure):
    _fields_ = [("kind", ctypes.c_uint8), ("code", ctypes.c_uint32)]


_lib.rsact_create.argtypes = [ctypes.c_uint16, ctypes.c_uint16]
_lib.rsact_create.restype = ctypes.c_void_p

_lib.rsact_terminal_size.argtypes = [
    ctypes.POINTER(ctypes.c_uint16),
    ctypes.POINTER(ctypes.c_uint16),
]
_lib.rsact_terminal_size.restype = ctypes.c_int32

_lib.rsact_destroy.argtypes = [ctypes.c_void_p]
_lib.rsact_destroy.restype = None

_lib.rsact_set_cell.argtypes = [
    ctypes.c_void_p,
    ctypes.c_uint16,
    ctypes.c_uint16,
    ctypes.c_uint32,
    ctypes.c_uint32,
    ctypes.c_uint32,
    ctypes.c_uint8,
]
_lib.rsact_set_cell.restype = ctypes.c_int32

_lib.rsact_render.argtypes = [ctypes.c_void_p]
_lib.rsact_render.restype = ctypes.c_int32

_lib.rsact_poll_key.argtypes = [ctypes.c_void_p, ctypes.POINTER(RsactKeyEvent)]
_lib.rsact_poll_key.restype = ctypes.c_int32

_lib.rsact_element_text.argtypes = [
    ctypes.c_char_p,
    ctypes.c_char_p,
    ctypes.c_uint16,
    ctypes.c_uint32,
    ctypes.c_uint32,
    ctypes.c_uint8,
]
_lib.rsact_element_text.restype = ctypes.c_void_p

_lib.rsact_element_container.argtypes = [
    ctypes.c_char_p,
    ctypes.c_uint8,
    ctypes.c_uint16,
    ctypes.c_uint16,
    ctypes.POINTER(ctypes.c_void_p),
    ctypes.c_size_t,
]
_lib.rsact_element_container.restype = ctypes.c_void_p

_lib.rsact_element_free.argtypes = [ctypes.c_void_p]
_lib.rsact_element_free.restype = None

_lib.rsact_tree_create.argtypes = [ctypes.c_uint16, ctypes.c_uint16]
_lib.rsact_tree_create.restype = ctypes.c_void_p

_lib.rsact_tree_present.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
_lib.rsact_tree_present.restype = ctypes.c_int32

_lib.rsact_tree_poll_key.argtypes = [ctypes.c_void_p, ctypes.POINTER(RsactKeyEvent)]
_lib.rsact_tree_poll_key.restype = ctypes.c_int32

_lib.rsact_tree_destroy.argtypes = [ctypes.c_void_p]
_lib.rsact_tree_destroy.restype = None


def _attrs(bold: bool = False, underline: bool = False, reverse: bool = False) -> int:
    """Same encoding rsact-ffi's C API uses: the *sum* of the ANSI SGR
    codes to apply (1=bold, 4=underline, 7=reverse), not an OR of flags."""
    total = 0
    if bold:
        total += 1
    if underline:
        total += 4
    if reverse:
        total += 7
    return total


def terminal_size() -> tuple[int, int]:
    """Returns `(rows, cols)` of the current terminal."""
    row = ctypes.c_uint16()
    col = ctypes.c_uint16()
    rc = _lib.rsact_terminal_size(ctypes.byref(row), ctypes.byref(col))
    if rc != 0:
        raise OSError("rsact_terminal_size failed")
    return row.value, col.value


class KeyEvent:
    """A decoded key. `.char` is set for a printable character, `.special`
    for one of the `RSACT_KEY_*` constants — exactly one of the two."""

    __slots__ = ("char", "special")

    def __init__(self, raw: RsactKeyEvent):
        if raw.kind == 1:
            self.char: str | None = chr(raw.code)
            self.special: int | None = None
        elif raw.kind == 2:
            self.char = None
            self.special = raw.code
        else:
            self.char = None
            self.special = None

    def __repr__(self) -> str:
        if self.char is not None:
            return f"KeyEvent(char={self.char!r})"
        return f"KeyEvent(special={self.special!r})"


class Terminal:
    """The flat cell-buffer API: draw with `set_cell`, then `render` to
    diff-and-flush just the changed cells to the terminal in one write.

    Use as a context manager, or call `close()` yourself:

    >>> with Terminal(40, 10) as term:  # doctest: +SKIP
    ...     term.set_cell(0, 0, "R")
    ...     term.render()
    """

    def __init__(self, width: int, height: int):
        handle = _lib.rsact_create(width, height)
        if not handle:
            raise RuntimeError("rsact_create failed (not a real terminal?)")
        self._handle = handle

    def set_cell(
        self,
        row: int,
        col: int,
        ch: str,
        fg: int = 0xFFFFFF,
        bg: int = 0x000000,
        bold: bool = False,
        underline: bool = False,
        reverse: bool = False,
    ) -> None:
        """Sets one cell. `fg`/`bg` are packed as `0xRRGGBB`."""
        rc = _lib.rsact_set_cell(
            self._handle, row, col, ord(ch), fg, bg, _attrs(bold, underline, reverse)
        )
        if rc != 0:
            raise ValueError(f"rsact_set_cell failed (rc={rc})")

    def render(self) -> None:
        """Diffs against the last-rendered frame and writes only the
        changed cells, in a single write."""
        rc = _lib.rsact_render(self._handle)
        if rc != 0:
            raise OSError("rsact_render failed")

    def poll_key(self) -> KeyEvent | None:
        """Blocks until a key arrives; returns `None` on EOF."""
        event = RsactKeyEvent()
        rc = _lib.rsact_poll_key(self._handle, ctypes.byref(event))
        if rc < 0:
            raise OSError(f"rsact_poll_key failed (rc={rc})")
        if rc == 0:
            return None
        return KeyEvent(event)

    def close(self) -> None:
        if getattr(self, "_handle", None):
            _lib.rsact_destroy(self._handle)
            self._handle = None

    def __enter__(self) -> "Terminal":
        return self

    def __exit__(self, *exc_info) -> None:
        self.close()

    def __del__(self) -> None:
        self.close()


class Element:
    """An owned, unattached element tree, built by `Element.text`/
    `Element.container`. Passing one as a child to `Element.container`,
    or to `Tree.present`, transfers ownership — the `Element` object
    becomes inert afterward. An `Element` you built but never attached
    anywhere must be freed with `.free()`."""

    __slots__ = ("_ptr",)

    def __init__(self, ptr: int):
        self._ptr = ptr

    @staticmethod
    def text(
        key: str,
        content: str,
        width: int = 0,
        fg: int = 0xFFFFFF,
        bg: int = 0x000000,
        bold: bool = False,
        underline: bool = False,
        reverse: bool = False,
    ) -> "Element":
        """A single-line text leaf, width and style set up front."""
        ptr = _lib.rsact_element_text(
            key.encode("utf-8"),
            content.encode("utf-8"),
            width,
            fg,
            bg,
            _attrs(bold, underline, reverse),
        )
        if not ptr:
            raise ValueError("rsact_element_text failed (bad key/content?)")
        return Element(ptr)

    @staticmethod
    def container(
        key: str,
        layout: int,
        children,
        width: int = 0,
        height: int = 0,
    ) -> "Element":
        """A container stacking `children` (any iterable of `Element`)
        along `layout`'s axis (`RSACT_LAYOUT_VERTICAL`/`_HORIZONTAL`).
        Always consumes every child, whether or not this call succeeds —
        don't reuse them afterward."""
        children = list(children)
        arr = (ctypes.c_void_p * len(children))(*(c._ptr for c in children))
        for child in children:
            child._ptr = None  # always consumed, per rsact_element_container's contract
        ptr = _lib.rsact_element_container(
            key.encode("utf-8"), layout, width, height, arr, len(children)
        )
        if not ptr:
            raise ValueError("rsact_element_container failed (bad key/layout, or a null child)?")
        return Element(ptr)

    def free(self) -> None:
        """Frees this element tree. Only call this on an `Element` you
        built but never attached to a container or presented."""
        if self._ptr:
            _lib.rsact_element_free(self._ptr)
            self._ptr = None


class Tree:
    """The component-tree API: build a fresh `Element` tree every frame
    and call `present`, which reconciles it against the previous frame
    and draws only what changed.

    >>> with Tree(20, 1) as tree:  # doctest: +SKIP
    ...     tree.present(Element.text("greeting", "hi", width=10))
    """

    def __init__(self, width: int, height: int):
        handle = _lib.rsact_tree_create(width, height)
        if not handle:
            raise RuntimeError("rsact_tree_create failed (not a real terminal?)")
        self._handle = handle

    def present(self, element: Element) -> None:
        """Replaces the tree's root with `element` (consuming it,
        regardless of outcome) and draws the result."""
        ptr = element._ptr
        element._ptr = None
        rc = _lib.rsact_tree_present(self._handle, ptr)
        if rc != 0:
            raise OSError(f"rsact_tree_present failed (rc={rc})")

    def poll_key(self) -> KeyEvent | None:
        """Blocks until a key arrives; returns `None` on EOF."""
        event = RsactKeyEvent()
        rc = _lib.rsact_tree_poll_key(self._handle, ctypes.byref(event))
        if rc < 0:
            raise OSError(f"rsact_tree_poll_key failed (rc={rc})")
        if rc == 0:
            return None
        return KeyEvent(event)

    def close(self) -> None:
        if getattr(self, "_handle", None):
            _lib.rsact_tree_destroy(self._handle)
            self._handle = None

    def __enter__(self) -> "Tree":
        return self

    def __exit__(self, *exc_info) -> None:
        self.close()

    def __del__(self) -> None:
        self.close()
