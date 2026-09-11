# rsact (Python)

A pure-Python [`ctypes`](https://docs.python.org/3/library/ctypes.html) wrapper around [`rsact-ffi`](https://github.com/MovingJu/Rsact/tree/main/crates/rsact-ffi)'s C ABI. No compiled extension, no build step of its own — see [#24](https://github.com/MovingJu/Rsact/issues/24) if you're after a real PyPI package (a `rsact-py` crate built with PyO3/maturin); this is the lighter alternative discussed there, useful today without waiting on that.

## Setup

You need the native `rsact_ffi` **shared** library built (the *static* lib the CMake examples use won't work here — `ctypes` needs a `.so`/`.dylib`/`.dll`):

```sh
# from the repo root
cargo build --release -p rsact-ffi
```

This produces `target/release/librsact_ffi.so` (Linux) / `.dylib` (macOS) / `target/release/rsact_ffi.dll` (Windows). `import rsact` finds it automatically when running from inside a checkout of this repo. Outside of that (or if it's installed somewhere else), point at it explicitly:

```sh
export RSACT_FFI_LIB=/path/to/librsact_ffi.so
```

No dependencies (just the standard library), so [`uv run`](https://docs.astral.sh/uv/) is the default way to run anything here — it picks up this directory's `pyproject.toml`, provisions a Python interpreter if needed, and runs the script, no `pip install`/virtualenv setup of your own required:

```sh
uv run examples/python/basic.py
```

Plain `python3 examples/python/basic.py` works too (same standard library, no deps either way) — `uv` just removes the "do I have the right Python/venv" step.

## Quick start

### Flat buffer (`Terminal`)

```python
from rsact import Terminal

with Terminal(40, 10) as term:
    for col in range(40):
        term.set_cell(0, col, "-")
    term.render()
```

### Component tree (`Element` / `Tree`)

Python has no `Component` trait to implement, same as the C binding: rebuild the `Element` tree yourself each frame and hand it to `Tree.present`.

```python
from rsact import Element, RSACT_LAYOUT_VERTICAL, Tree

with Tree(20, 2) as tree:
    for count in range(5):
        root = Element.container(
            "counter",
            RSACT_LAYOUT_VERTICAL,
            [
                Element.text("label", "left", width=20),
                Element.text("value", str(count), width=20),
            ],
            width=20,
            height=2,
        )
        tree.present(root)
```

`Element.container`'s `children` always get consumed (ownership transfers to the container) whether or not the call succeeds — never reuse an `Element` after passing it to `container` or `Tree.present`. An `Element` you build but never attach anywhere needs `.free()`.

## Examples

| Run it with | What it shows |
|---|---|
| `uv run examples/python/basic.py` | A single static frame (a horizontal line on row 0), then exits — mirrors `rsact-ffi`'s `basic.rs`/`examples/c/src/basic.c` |
| `uv run examples/python/animate.py` | A `#` character moving across row 5 (~16ms/frame) — mirrors `animate.rs`/`animate.c` |
| `uv run examples/python/tree.py` | A two-counter `Dashboard`, built through the component-tree API — mirrors `tree.rs`/`tree.c` |

All three run straight from a checkout (no install step) as long as `rsact-ffi` has been built per Setup above.

## What's wrapped

- `Terminal` — `rsact_create`/`set_cell`/`render`/`poll_key`/`close` (also a context manager)
- `Element.text` / `Element.container` / `.free()` — `rsact_element_*`
- `Tree` — `rsact_tree_create`/`present`/`poll_key`/`close` (also a context manager)
- `terminal_size()` — `rsact_terminal_size`
- `KeyEvent` — decodes `RsactKeyEvent` into `.char` (printable) or `.special` (one of the `RSACT_KEY_*` constants)

As in the C API, `Terminal` and `Tree` are independent: don't call both on work meant to share one screen, since each tracks its own "what's actually on screen" baseline and mixing them will leave one stale.

## License

[MIT](https://github.com/MovingJu/Rsact/blob/main/LICENSE) © [MovingJu](https://github.com/MovingJu) — same as the rest of the workspace.
