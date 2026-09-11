# Python examples

Three scripts against [`rsact-py`](https://github.com/MovingJu/Rsact/tree/main/crates/rsact-py) — native [PyO3](https://pyo3.rs) bindings for `rsact-core` — mirroring the Rust (`crates/rsact-ffi/examples/`) and C (`examples/c/src/`) ones. This directory holds only the examples; the package itself lives at [`crates/rsact-py`](https://github.com/MovingJu/Rsact/tree/main/crates/rsact-py), same split as `examples/c` (a consumer) vs. `crates/rsact-ffi` (the library).

(There's also a [`ctypes`-based binding](https://github.com/MovingJu/Rsact/tree/main/crates/rsact-ffi/python) that needs no Rust toolchain to use against a prebuilt release — see its README for the tradeoff. These examples exercise the native one.)

## Run them

```sh
uv run examples/python/tree.py
```

`uv run` resolves this directory's `rsact` dependency to the local `crates/rsact-py` package (see `pyproject.toml`'s `[tool.uv.sources]` — a repo-local convenience; an external consumer's own project would have no such override) and builds it via `maturin` automatically — no separate `cargo build` step of your own, no `pip install`. The very first run compiles `rsact-py` and its dependencies from scratch, so expect it to take a while; later runs are fast.

| Run it with | What it shows |
|---|---|
| `uv run examples/python/basic.py` | A single static frame (a horizontal line on row 0), then exits — mirrors `rsact-ffi`'s `basic.rs`/`examples/c/src/basic.c` |
| `uv run examples/python/animate.py` | A `#` character moving across row 5 (~16ms/frame) — mirrors `animate.rs`/`animate.c` |
| `uv run examples/python/tree.py` | A two-counter `Dashboard`, built through the component-tree API — mirrors `tree.rs`/`tree.c` |

Notice none of them use `with`: `Terminal`/`Tree` restore the terminal automatically via Rust's own `Drop` when garbage-collected, so a real script doesn't have to nest its whole body one indent level deeper just for cleanup. See `crates/rsact-py/README.md`'s [API shape](https://github.com/MovingJu/Rsact/blob/main/crates/rsact-py/README.md#api-shape) section.

## Using `rsact` in your own project

See [`crates/rsact-py/README.md`](https://github.com/MovingJu/Rsact/blob/main/crates/rsact-py/README.md) for the full API. It's not on PyPI yet ([#24](https://github.com/MovingJu/Rsact/issues/24)), and — unlike the `ctypes` binding — there's no prebuilt-binary install path for it either, since PyO3 extensions need to be built per Python ABI: for now, using it outside this repo means building it yourself against a clone.
