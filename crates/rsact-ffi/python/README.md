# rsact (Python)

A pure-Python [`ctypes`](https://docs.python.org/3/library/ctypes.html) wrapper around [`rsact-ffi`](https://github.com/MovingJu/Rsact/tree/main/crates/rsact-ffi)'s C ABI — this directory is that crate's Python binding, the same way [`../examples`](../examples) holds its Rust ones. No compiled extension of its own — see [#24](https://github.com/MovingJu/Rsact/issues/24) if you're after a real PyPI package (a `rsact-py` crate built with PyO3/maturin); this is the lighter alternative discussed there, useful today without waiting on that.

Runnable examples that use this package live separately, under [`examples/python`](https://github.com/MovingJu/Rsact/tree/main/examples/python) at the repo root — same split as `crates/rsact-ffi` (the library) vs. `examples/c` (a consumer of it).

## Install

Not on PyPI yet, but already a real installable package — `pip`/`uv` can pull it straight from this repo, same as any other git-hosted Python package. You don't need a clone of this repo for this, or anything after it:

```sh
uv add "rsact @ git+https://github.com/MovingJu/Rsact.git#subdirectory=crates/rsact-ffi/python"
# or: pip install "git+https://github.com/MovingJu/Rsact.git#subdirectory=crates/rsact-ffi/python"
```

Once #24 lands, that becomes `uv add rsact` / `pip install rsact` — nothing about the API or this install shape changes, just the source `uv`/`pip` fetch it from.

`import rsact` needs the native `rsact_ffi` **shared** library too (the *static* lib the CMake C examples use won't load via `ctypes`). It finds one automatically, checked in this order:

1. The `RSACT_FFI_LIB` environment variable, if set (an exact path).
2. `target/release/<libname>` relative to the repo root, if you happen to be running from inside a checkout with `rsact-ffi` already built — this is what makes local development (see below) work, not something an installed-from-git user needs to think about.
3. The system library search path (if `rsact_ffi` is installed system-wide).
4. **Downloaded automatically** from the matching [GitHub Release](https://github.com/MovingJu/Rsact/releases), SHA256-verified, and cached under `~/.cache/rsact/` — this is the path a real `pip`/`uv` install actually takes, with **no Rust toolchain and no Rsact checkout involved at all**. (Needs a release built after the shared library started being published; see the note in [What's wrapped](#whats-wrapped) — `v0.2.0` itself predates it.)

Verified this end to end: installed the package above into a throwaway venv with no Rsact checkout anywhere nearby, then ran a plain script doing nothing but `import rsact` from that venv's interpreter — it correctly skipped straight past steps 1–3 (nothing local to find) to step 4, downloaded and SHA256-verified the real release archive, and reported the expected "this release predates prebuilt shared libraries" once it got there.

## Local development

Hacking on this package itself (not just using it)? Clone the repo and use [`uv`](https://docs.astral.sh/uv/) from inside this directory:

```sh
cargo build --release -p rsact-ffi   # from the repo root, once
cd crates/rsact-ffi/python
uv run python3 -c "import rsact; print(rsact.terminal_size())"
```

To exercise it against a real terminal, use the example scripts in [`examples/python`](https://github.com/MovingJu/Rsact/tree/main/examples/python) instead — they depend on this package the same way an external consumer would, but with a local-path override so iterating on this package's source is immediate, no publish/reinstall step.

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

## What's wrapped

- `Terminal` — `rsact_create`/`set_cell`/`render`/`poll_key`/`close` (also a context manager)
- `text()` / `container()` returning `Text`/`Container` (aliased as `Node`) — pure data, no FFI call happens until you `present()` them
- `Tree` — `rsact_tree_create`/`present`/`poll_key`/`close` (also a context manager); `present(node)` compiles a `Node` tree to native elements and draws it in one step
- `terminal_size()` — `rsact_terminal_size`
- `KeyEvent` — decodes `RsactKeyEvent` into `.char` (printable) or `.special` (one of the `RSACT_KEY_*` constants)

As in the C API, `Terminal` and `Tree` are independent: don't call both on work meant to share one screen, since each tracks its own "what's actually on screen" baseline and mixing them will leave one stale.

**On the auto-download fallback:** the release each `rsact/__init__.py` is pinned to (`_RSACT_VERSION`, currently `v0.2.0`) has to actually include a built shared library in its archive — this is new as of the release workflow change that added `_download_release_library`, so `v0.2.0` itself predates it and the fallback will report that clearly rather than silently failing. It'll start working from the next tagged release onward; `_RSACT_VERSION` (here) and `RSACT_VERSION` (in `examples/c/CMakeLists.txt`) need bumping together each time.

## License

[MIT](https://github.com/MovingJu/Rsact/blob/main/LICENSE) © [MovingJu](https://github.com/MovingJu) — same as the rest of the workspace.
