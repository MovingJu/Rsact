"""Pythonic ctypes bindings for rsact-ffi.

This is a pure-Python `ctypes` wrapper around `rsact-ffi`'s C ABI — no
compiled extension, no build step of its own. It loads the native
`rsact_ffi` *shared* library (`librsact_ffi.so`/`.dylib`/`rsact_ffi.dll`)
at import time; see `_find_library` below for how it's located.

Two independent APIs are exposed, mirroring `rsact-ffi` itself:

- `Terminal` — the flat cell-buffer API (`rsact_create`/`set_cell`/`render`).
- `text()`/`container()` + `Tree` — the declarative component-tree API
  (`rsact_element_*`/`rsact_tree_*`). Unlike the raw C API, a tree you
  build here is just immutable data (`Text`/`Container`) — no owned
  native handles, no `.free()`, nothing to consume-and-invalidate. The
  FFI conversion happens once, functionally, inside `Tree.present()`.

As in the C API, don't mix the two on work meant to share a screen: each
tracks its own independent "what's actually on screen" baseline.
"""

from __future__ import annotations

import ctypes
import ctypes.util
import hashlib
import io
import os
import platform
import tarfile
import urllib.request
import zipfile
from dataclasses import dataclass, field
from typing import Iterable, Union

__all__ = [
    "Terminal",
    "Text",
    "Container",
    "Node",
    "text",
    "container",
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


# The version and per-platform archive hashes examples/c/CMakeLists.txt
# also pins (RSACT_VERSION/RSACT_ARCHIVE_SHA256) — bump both together on
# every release. (system, machine) -> (target triple, archive extension,
# archive SHA256).
_RSACT_VERSION = "v0.2.0"
_RELEASE_TARGETS: dict[tuple[str, str], tuple[str, str, str]] = {
    ("Linux", "x86_64"): (
        "x86_64-unknown-linux-gnu",
        "tar.gz",
        "ed588de9eaec01c0e8784bb3fd3941b5ff55b393bbd92c8e97069eb8f51b06f7",
    ),
    ("Darwin", "arm64"): (
        "aarch64-apple-darwin",
        "tar.gz",
        "6f1c0d6f7ec1fa0d7d98217d8d724b7dfd4ff0737a243ac78cd5ef98f4112115",
    ),
    ("Darwin", "aarch64"): (
        "aarch64-apple-darwin",
        "tar.gz",
        "6f1c0d6f7ec1fa0d7d98217d8d724b7dfd4ff0737a243ac78cd5ef98f4112115",
    ),
    ("Windows", "AMD64"): (
        "x86_64-pc-windows-msvc",
        "zip",
        "e37363e9644411cf0c966a6f5af9e2c028d4fcdda821bba1c653ec30a6d09c81",
    ),
}


def _download_release_library(lib_name: str) -> str:
    """Last resort: downloads the prebuilt archive for this platform from
    the pinned GitHub Release (same URL/hash scheme
    `examples/c/CMakeLists.txt` uses), verifies its SHA256, and extracts
    just the shared library into a local cache — so someone with no Rust
    toolchain at all can still `import rsact`. Cached, so this only
    touches the network once per version/platform.
    """
    key = (platform.system(), platform.machine())
    entry = _RELEASE_TARGETS.get(key)
    if entry is None:
        raise RuntimeError(
            f"rsact: no prebuilt release published for {key[0]}/{key[1]}. "
            "Build rsact-ffi from source (`cargo build --release -p "
            "rsact-ffi`) and set RSACT_FFI_LIB to the result instead."
        )
    target, ext, expected_sha256 = entry

    cache_dir = os.path.join(os.path.expanduser("~"), ".cache", "rsact", _RSACT_VERSION, target)
    cached = os.path.join(cache_dir, lib_name)
    if os.path.exists(cached):
        return cached

    url = f"https://github.com/MovingJu/Rsact/releases/download/{_RSACT_VERSION}/rsact-{target}.{ext}"
    with urllib.request.urlopen(url) as response:  # noqa: S310 (fixed https:// URL, checksum verified below)
        archive_bytes = response.read()

    digest = hashlib.sha256(archive_bytes).hexdigest()
    if digest != expected_sha256:
        raise RuntimeError(
            f"rsact: checksum mismatch downloading {url} "
            f"(got {digest}, expected {expected_sha256}) — refusing to load it."
        )

    member_path = f"rsact-{target}/lib/{lib_name}"
    buf = io.BytesIO(archive_bytes)
    try:
        if ext == "zip":
            with zipfile.ZipFile(buf) as archive:
                data = archive.read(member_path)
        else:
            with tarfile.open(fileobj=buf, mode="r:gz") as archive:
                member = archive.getmember(member_path)
                extracted = archive.extractfile(member)
                assert extracted is not None
                data = extracted.read()
    except KeyError as exc:
        raise RuntimeError(
            f"rsact: {url} doesn't contain {member_path} — this release "
            "predates prebuilt shared libraries; build rsact-ffi from "
            "source instead, or use a newer release."
        ) from exc

    os.makedirs(cache_dir, exist_ok=True)
    with open(cached, "wb") as f:
        f.write(data)
    return cached


def _find_library() -> str:
    """Locates the `rsact_ffi` shared library.

    Checks, in order: the `RSACT_FFI_LIB` environment variable (an exact
    path); `target/release/<libname>` relative to the repo root (for
    running straight out of a checkout after `cargo build --release -p
    rsact-ffi`); the system library search path via
    `ctypes.util.find_library`; and finally, as a last resort, downloads
    the prebuilt shared library for this platform from the matching
    GitHub Release (see `_download_release_library`) — the path that
    needs no local Rust toolchain at all.
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
    dev_candidate = os.path.normpath(
        os.path.join(here, "..", "..", "..", "..", "..", "target", "release", name)
    )
    if os.path.exists(dev_candidate):
        return dev_candidate

    found = ctypes.util.find_library("rsact_ffi")
    if found:
        return found

    return _download_release_library(name)


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


@dataclass(frozen=True)
class Text:
    """A single-line text leaf. Immutable, plain data — build one with
    `text()`, not directly. See the module docstring: this is the whole
    point of the functional redesign — a `Node` is just a value, freely
    composable, comparable, and reusable, with no native resource behind
    it until `Tree.present()` compiles it."""

    key: str
    content: str
    width: int = 0
    fg: int = 0xFFFFFF
    bg: int = 0x000000
    bold: bool = False
    underline: bool = False
    reverse: bool = False


@dataclass(frozen=True)
class Container:
    """A container stacking `children` along `layout`'s axis
    (`RSACT_LAYOUT_VERTICAL`/`_HORIZONTAL`). Immutable, plain data — build
    one with `container()`, not directly. See `Text`."""

    key: str
    layout: int
    children: tuple["Node", ...] = field(default_factory=tuple)
    width: int = 0
    height: int = 0


Node = Union[Text, Container]


def text(
    key: str,
    content: str,
    width: int = 0,
    fg: int = 0xFFFFFF,
    bg: int = 0x000000,
    bold: bool = False,
    underline: bool = False,
    reverse: bool = False,
) -> Text:
    """Builds a `Text` leaf."""
    return Text(key, content, width, fg, bg, bold, underline, reverse)


def container(
    key: str,
    layout: int,
    children: Iterable[Node] = (),
    width: int = 0,
    height: int = 0,
) -> Container:
    """Builds a `Container`, stacking `children` (any iterable of `Node` —
    a list, a generator, another `container()` call, ...) along `layout`'s
    axis."""
    return Container(key, layout, tuple(children), width, height)


def _compile(node: Node) -> int:
    """Converts a `Node` tree into native `RsactElement*` pointers with
    one bottom-up walk, handing each child straight into its parent as
    soon as it's built — the only place ownership of a native element
    pointer is ever visible is right here, never in application code."""
    if isinstance(node, Text):
        ptr = _lib.rsact_element_text(
            node.key.encode("utf-8"),
            node.content.encode("utf-8"),
            node.width,
            node.fg,
            node.bg,
            _attrs(node.bold, node.underline, node.reverse),
        )
        if not ptr:
            raise ValueError(f"rsact_element_text failed for key={node.key!r}")
        return ptr

    if isinstance(node, Container):
        child_ptrs: list[int] = []
        try:
            for child in node.children:
                child_ptrs.append(_compile(child))
        except Exception:
            # A later sibling failed to compile; the ones already built
            # were never handed to rsact_element_container, so nothing
            # else owns them yet — free them ourselves instead of leaking.
            for ptr in child_ptrs:
                _lib.rsact_element_free(ptr)
            raise

        arr = (ctypes.c_void_p * len(child_ptrs))(*child_ptrs)
        ptr = _lib.rsact_element_container(
            node.key.encode("utf-8"), node.layout, node.width, node.height, arr, len(child_ptrs)
        )
        if not ptr:
            # rsact_element_container already consumed (freed) every
            # child_ptrs entry internally, win or lose, so there's
            # nothing left for us to clean up here.
            raise ValueError(f"rsact_element_container failed for key={node.key!r}")
        return ptr

    raise TypeError(f"not a rsact.Text/rsact.Container node: {node!r}")


class Tree:
    """The component-tree API: build a fresh `Node` tree every frame (a
    plain, immutable value — see `text()`/`container()`) and call
    `present`, which compiles it to native elements, reconciles it
    against the previous frame, and draws only what changed.

    >>> with Tree(20, 1) as tree:  # doctest: +SKIP
    ...     tree.present(text("greeting", "hi", width=10))
    """

    def __init__(self, width: int, height: int):
        handle = _lib.rsact_tree_create(width, height)
        if not handle:
            raise RuntimeError("rsact_tree_create failed (not a real terminal?)")
        self._handle = handle

    def present(self, root: Node) -> None:
        """Compiles `root` to native elements, replaces the tree's root
        with the result, and draws it."""
        ptr = _compile(root)
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
