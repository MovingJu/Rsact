# Python examples

Three scripts against [`rsact`](https://github.com/MovingJu/Rsact/tree/main/crates/rsact-ffi/python) — `rsact-ffi`'s Python binding — mirroring the Rust (`crates/rsact-ffi/examples/`) and C (`examples/c/src/`) ones. This directory holds only the examples; the package itself lives at [`crates/rsact-ffi/python`](https://github.com/MovingJu/Rsact/tree/main/crates/rsact-ffi/python), same split as `examples/c` (a consumer) vs. `crates/rsact-ffi` (the library).

## Run them

```sh
cargo build --release -p rsact-ffi   # from the repo root, once
uv run examples/python/basic.py
```

`uv run` resolves this directory's `rsact` dependency to the local `crates/rsact-ffi/python` package (see `pyproject.toml`'s `[tool.uv.sources]` — that override is a repo-local convenience, not something these scripts need outside this checkout) and provisions everything else automatically.

| Run it with | What it shows |
|---|---|
| `uv run examples/python/basic.py` | A single static frame (a horizontal line on row 0), then exits — mirrors `rsact-ffi`'s `basic.rs`/`examples/c/src/basic.c` |
| `uv run examples/python/animate.py` | A `#` character moving across row 5 (~16ms/frame) — mirrors `animate.rs`/`animate.c` |
| `uv run examples/python/tree.py` | A two-counter `Dashboard`, built through the component-tree API — mirrors `tree.rs`/`tree.c` |

## Using `rsact` in your own project

See [`crates/rsact-ffi/python/README.md`](https://github.com/MovingJu/Rsact/blob/main/crates/rsact-ffi/python/README.md) for the full API and how to install the package itself (a git+subdirectory URL today, plain `pip install rsact` once [#24](https://github.com/MovingJu/Rsact/issues/24) publishes to PyPI) — none of that needs a clone of this repo, unlike these example scripts.
