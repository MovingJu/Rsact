# rsact-py

Native [PyO3](https://pyo3.rs) bindings for [`rsact-core`](https://github.com/MovingJu/Rsact/tree/main/crates/rsact-core) — wraps it directly (no C ABI indirection through `rsact-ffi`), so there's no hand-written FFI signature to keep in sync with a header. The Python module is named `rsact` (see `[lib] name` in `Cargo.toml`); this crate is `rsact-py`.

This is the "final goal" binding discussed in [#24](https://github.com/MovingJu/Rsact/issues/24), alongside the lighter [`ctypes`-based one](https://github.com/MovingJu/Rsact/tree/main/crates/rsact-ffi/python) that needs no Rust toolchain to build against a prebuilt release. Pick whichever fits: this one for compile-time-checked signatures, generated type stubs, and no hand-maintained FFI declarations; the `ctypes` one for zero build step against an already-released binary.

## Not a workspace member

`crates/rsact-py` is deliberately **excluded** from the root `[workspace]` (see the comment there). Building it — even a plain `cargo build`, not just via `maturin` — needs a **linkable** `libpython` (some Python installs, including some `uv`-managed ones, only ship the runtime `.so`, not the unversioned dev symlink needed to link against). That's not guaranteed on every contributor's machine, so keeping it out of the workspace means `cargo build/test/clippy --workspace` behave exactly as they did before this crate existed, for everyone who isn't touching Python bindings. Its `Cargo.toml` fields are plain values instead of `field.workspace = true` for the same reason — kept in sync with the workspace version by hand, same as `examples/c/CMakeLists.txt`'s `RSACT_VERSION` and `crates/rsact-ffi/python`'s `_RSACT_VERSION` already are.

## Building

Not built directly — [`examples/python`](https://github.com/MovingJu/Rsact/tree/main/examples/python) depends on it via a `[tool.uv.sources]` local-path override, so `uv run examples/python/tree.py` builds this crate (via `maturin`, which `uv` invokes automatically per this crate's `pyproject.toml`) and makes it importable in one step. See that directory's README for the full picture.

If you hit `cannot find -lpython3.XX` building this crate directly (`cargo build`/`cargo run --bin stub_gen`): your Python's dev symlink for its shared library is missing. Either install your platform's Python dev package (e.g. `apt install python3-dev` on Debian/Ubuntu), or point the linker at wherever the versioned `.so` actually lives:

```sh
RUSTFLAGS="-L /path/to/dir/containing/libpythonX.Y.so" cargo build
```

(On Debian/Ubuntu with only the runtime package installed, that's usually `/usr/lib/pythonX.Y/config-X.Y-<arch>/`.)

## Regenerating type stubs

`rsact.pyi` (shipped as `rsact/__init__.pyi` in the built wheel, alongside a `py.typed` marker — see `pyproject.toml`'s `[tool.maturin] include`) is generated from this crate's `#[gen_stub_*]`-annotated PyO3 definitions, not hand-written:

```sh
cargo run --bin stub_gen
```

Regenerate it after changing any public class/function signature or doc comment, and commit the result — `pyo3-stub-gen` derives real type hints (including keyword defaults) straight from the Rust source, so IDEs and `mypy`/`pyright` see accurate signatures without anyone hand-maintaining a second copy.

## API shape

Mirrors the `ctypes` binding's design (see its README for the full walkthrough) with one difference worth calling out: there's no `with`/context-manager here. `Terminal`/`Tree` hold a `RawModeGuard` (from `rsact-core`), and Rust's own `Drop` on it runs automatically when the Python object is garbage-collected — restoring the terminal needs no explicit `close()` or `with` block, so a real, long-running TUI script doesn't have to nest its whole body one indent level deeper just to get that. `close()` still exists for when you want the terminal restored at a specific point instead of whenever GC gets to it.

## Publishing to PyPI

`pyo3 = { features = ["abi3-py310"] }` (see `Cargo.toml`) means one wheel per platform covers every Python 3.10+ interpreter — no per-minor-version build matrix.

[`.github/workflows/python-release.yml`](https://github.com/MovingJu/Rsact/blob/main/.github/workflows/python-release.yml) builds that wheel for Linux (manylinux, via `PyO3/maturin-action`'s `manylinux: auto`), macOS, and Windows, gated behind the same `Tag Version Check` workflow as `release.yml`, and publishes them with [`pypa/gh-action-pypi-publish`](https://github.com/pypa/gh-action-pypi-publish) using **Trusted Publishing** (OIDC — `id-token: write`, no stored API token/secret).

Trusted Publishing needs one manual, one-time setup step on pypi.org that CI can't do for you: register a "pending publisher" for the `rsact` project — PyPI project name `rsact`, owner/repo `MovingJu/Rsact`, workflow filename `python-release.yml`, environment name `pypi` (matching the `environment:` key in that workflow). Do this *before* the first tag push that should publish; after that first successful publish, PyPI links the project to the workflow automatically and no further manual step is needed for subsequent releases.

## License

[MIT](https://github.com/MovingJu/Rsact/blob/main/LICENSE) © [MovingJu](https://github.com/MovingJu) — same as the rest of the workspace.
