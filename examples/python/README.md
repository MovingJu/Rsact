# rsact (Python)

A pure-Python [`ctypes`](https://docs.python.org/3/library/ctypes.html) wrapper around [`rsact-ffi`](https://github.com/MovingJu/Rsact/tree/main/crates/rsact-ffi)'s C ABI. No compiled extension of its own — see [#24](https://github.com/MovingJu/Rsact/issues/24) if you're after a real PyPI package (a `rsact-py` crate built with PyO3/maturin); this is the lighter alternative discussed there, useful today without waiting on that.

Standard `src/` layout (`src/rsact/__init__.py`), built with [`uv`](https://docs.astral.sh/uv/) — `uv run` is the default way to use anything here.

## Setup

`import rsact` needs the native `rsact_ffi` **shared** library (the *static* lib the CMake C examples use won't load via `ctypes` — this needs a `.so`/`.dylib`/`.dll`). It's found automatically, checked in this order:

1. The `RSACT_FFI_LIB` environment variable, if set (an exact path).
2. `target/release/<libname>` relative to the repo root — for running straight out of a checkout after `cargo build --release -p rsact-ffi`.
3. The system library search path (if `rsact_ffi` is installed system-wide).
4. **Downloaded automatically** from the matching [GitHub Release](https://github.com/MovingJu/Rsact/releases), SHA256-verified, and cached under `~/.cache/rsact/` — so this works with **no Rust toolchain at all**. (Needs a release built after the shared library started being published; see the note in [What's wrapped](#whats-wrapped).)

So the simplest path is just:

```sh
uv run examples/python/tree.py
```

`uv run` picks up this directory's `pyproject.toml`, provisions a Python interpreter and an editable install of the `rsact` package into `.venv` if needed, and runs the script — no `pip install` of your own required. Plain `python3 -m pip install -e examples/python && python3 examples/python/tree.py` works too, if you'd rather not use `uv`.

## Quick start

### Flat buffer (`Terminal`)

```python
from rsact import Terminal

with Terminal(40, 10) as term:
    for col in range(40):
        term.set_cell(0, col, "-")
    term.render()
```

### Component tree (`text` / `container` / `Tree`)

Python has no `Component` trait to implement, same as the C binding: rebuild the tree yourself each frame. Unlike the raw C API, the tree you build is just **immutable data** — `text()`/`container()` return plain `Text`/`Container` dataclasses, freely comparable and reusable, with no native handle behind them and nothing to `.free()`. The only place a native `RsactElement` ever exists is inside `Tree.present()`, which compiles the tree you hand it and immediately consumes the result:

```python
from rsact import RSACT_LAYOUT_VERTICAL, Tree, container, text

with Tree(20, 2) as tree:
    for count in range(5):
        tree.present(
            container(
                "counter",
                RSACT_LAYOUT_VERTICAL,
                [text("label", "left", width=20), text("value", str(count), width=20)],
                width=20,
                height=2,
            )
        )
```

## Examples

| Run it with | What it shows |
|---|---|
| `uv run examples/python/basic.py` | A single static frame (a horizontal line on row 0), then exits — mirrors `rsact-ffi`'s `basic.rs`/`examples/c/src/basic.c` |
| `uv run examples/python/animate.py` | A `#` character moving across row 5 (~16ms/frame) — mirrors `animate.rs`/`animate.c` |
| `uv run examples/python/tree.py` | A two-counter `Dashboard`, built through the component-tree API — mirrors `tree.rs`/`tree.c` |

## What's wrapped

- `Terminal` — `rsact_create`/`set_cell`/`render`/`poll_key`/`close` (also a context manager)
- `text()` / `container()` returning `Text`/`Container` (aliased as `Node`) — pure data, no FFI call happens until you `present()` them
- `Tree` — `rsact_tree_create`/`present`/`poll_key`/`close` (also a context manager); `present(node)` compiles a `Node` tree to native elements and draws it in one step
- `terminal_size()` — `rsact_terminal_size`
- `KeyEvent` — decodes `RsactKeyEvent` into `.char` (printable) or `.special` (one of the `RSACT_KEY_*` constants)

As in the C API, `Terminal` and `Tree` are independent: don't call both on work meant to share one screen, since each tracks its own "what's actually on screen" baseline and mixing them will leave one stale.

**On the auto-download fallback:** the release each `rsact.py` is pinned to (`_RSACT_VERSION`, currently `v0.2.0`) has to actually include a built shared library in its archive — this is new as of the release workflow change that added `_download_release_library`, so `v0.2.0` itself predates it and the fallback will report that clearly rather than silently failing. It'll start working from the next tagged release onward; `_RSACT_VERSION` (here) and `RSACT_VERSION` (in `examples/c/CMakeLists.txt`) need bumping together each time.

## License

[MIT](https://github.com/MovingJu/Rsact/blob/main/LICENSE) © [MovingJu](https://github.com/MovingJu) — same as the rest of the workspace.
