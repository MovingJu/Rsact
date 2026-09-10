# Rsact

[![Commit Messages](https://github.com/MovingJu/Rsact/actions/workflows/commitlint.yml/badge.svg)](https://github.com/MovingJu/Rsact/actions/workflows/commitlint.yml)
[![PR Title](https://github.com/MovingJu/Rsact/actions/workflows/pr-title.yml/badge.svg)](https://github.com/MovingJu/Rsact/actions/workflows/pr-title.yml)
[![crates.io](https://img.shields.io/crates/v/rsact-core.svg)](https://crates.io/crates/rsact-core)
[![docs.rs](https://docs.rs/rsact-core/badge.svg)](https://docs.rs/rsact-core)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/MovingJu/Rsact/blob/main/LICENSE)
[![Rust edition](https://img.shields.io/badge/edition-2024-orange.svg)](https://github.com/MovingJu/Rsact/blob/main/Cargo.toml)

**A React-flavored virtual DOM rendering engine for the terminal.** Written in Rust, and exposed to C/C++ through a thin C ABI (`rsact-ffi`).

You draw the state you want into a virtual buffer — a grid of cells, each a character plus a style. `rsact-core` diffs that buffer against the last frame it actually drew, and turns only the cells that changed into the minimal set of ANSI escape sequences needed, sent to the terminal in a **single `write()` call**. It's the same idea React applies to the real DOM, applied to a terminal's cell grid instead.

> This is an actively developed personal project (currently v0.1.1). The API is not yet stable — see the [Roadmap](#roadmap) below.

## What is this?

- **No TUI dependencies at all** — no `crossterm`, no `ncurses`. Raw-mode entry/exit, cursor movement, and color/style escape sequences are all implemented directly in `rsact-core`. The only dependencies are one per platform: `libc` (termios) on Unix, `windows-sys` (Win32 console API) on Windows.
- **The virtual DOM is just a cell array.** `Buffer` holds `width * height` `Cell { ch: char, style: Style }` values in row-major order — a plain `Vec`, no tree structure, no macros.
- **Diffing extracts only the changed runs.** Two same-sized buffers are scanned row by row; adjacent changed columns are merged into a single contiguous `Patch`, so the renderer needs at most one cursor move per patch, not per cell.
- **The renderer re-emits SGR codes only when the style actually changes.** If a cell's style matches the last one written, only the character is appended. A whole frame is assembled into one buffer and flushed with **exactly one `write_all` call** (none at all if there are no patches).

## Features

- **Cell-based virtual buffer** — `Color::Default` / `Indexed(u8)` (256-color) / `Rgb(u8, u8, u8)` (truecolor), plus `bold` / `underline` / `reverse` styling
- **Minimal diff engine** — compares two buffers and returns non-overlapping contiguous `Patch` runs; a seeded property test verifies that applying the patches reconstructs the next frame exactly
- **Batched renderer** — one cursor move per patch, SGR emitted only on style change, the whole frame flushed in a single syscall
- **Cross-platform raw mode** — `term_unix.rs` (libc/termios) and `term_windows.rs` (windows-sys, with VT processing enabled) are the only platform-specific code; `renderer.rs` and everything above it has no `cfg` branching at all
- **Cursor restored on exit** — `RawModeGuard::enable_safe_exit` moves the cursor to the terminal's last row/col and emits a trailing newline on `Drop`, so the shell prompt doesn't land on top of the last frame ([#4](https://github.com/MovingJu/Rsact/issues/4), fixed in [#5](https://github.com/MovingJu/Rsact/pull/5))
- **UTF-8-aware input parser** — arrow keys, Ctrl+letter, Backspace/Enter/Esc, and multi-byte UTF-8 characters (Korean syllables, emoji) all decode correctly from a single `read_key()` call
- **Auto-generated C header** — `rsact-ffi`'s `build.rs` regenerates `include/rsact.h` via `cbindgen` on every build

## Crate layout

This repository is a Cargo workspace with three crates:

| Crate | Kind | Description |
|---|---|---|
| [`rsact-core`](https://github.com/MovingJu/Rsact/tree/main/crates/rsact-core) | lib | The virtual terminal buffer, diffing, renderer, raw mode, and input parser. The heart of the project |
| [`rsact-demo`](https://github.com/MovingJu/Rsact/tree/main/crates/rsact-demo) | bin (`publish = false`) | An example built on `rsact-core` alone — animates a growing rectangle |
| [`rsact-ffi`](https://github.com/MovingJu/Rsact/tree/main/crates/rsact-ffi) | cdylib/staticlib/lib | A C ABI wrapper around `rsact-core`: a handle-based API for the flat buffer (`rsact_create`/`rsact_set_cell`/`rsact_render`) plus one for the component tree (`rsact_element_*`/`rsact_tree_*`), and a `cbindgen`-generated header |

```text
Rsact/
├── CMakeLists.txt          # Corrosion-based build exposing the rsact::rsact_ffi CMake target
├── crates/
│   ├── rsact-core/        # buffer · cell · diff · renderer · term(+term_unix/term_windows) · input
│   ├── rsact-demo/        # rsact-core-only demo (animated rectangle)
│   └── rsact-ffi/         # C ABI; build.rs generates include/rsact.h via cbindgen
├── examples/c/             # 3 C examples (src/) linking a prebuilt rsact-ffi release via CMake FetchContent
├── .github/workflows/      # PR title & commit message convention checks
├── CONTRIBUTING.md
└── LICENSE (MIT)
```

## Quick start

### Using it from Rust

Not published to crates.io yet, so pull it in as a git dependency:

```toml
[dependencies]
rsact-core = { git = "https://github.com/MovingJu/Rsact", package = "rsact-core" }
```

Minimal usage (see [`crates/rsact-demo/src/main.rs`](https://github.com/MovingJu/Rsact/blob/main/crates/rsact-demo/src/main.rs) for the full version):

```rust,no_run
use rsact_core::{buffer::Buffer, cell::Cell, diff, renderer::Renderer, term};
use rsact_core::term::RawModeGuard;

fn main() -> std::io::Result<()> {
    let size = term::terminal_size()?;

    // Restores raw mode and the cursor position when dropped.
    let _guard = RawModeGuard::enable_safe_exit(size, std::io::stdout())?;

    let mut real_dom = Buffer::new(size.col, size.row);
    let mut virtual_dom = Buffer::new(size.col, size.row);
    let mut renderer = Renderer::new(std::io::stdout());

    // Draw the state you want into the virtual buffer.
    virtual_dom.set(0, 0, Cell { ch: 'R', ..Cell::default() });

    // Compute what changed, draw it, and commit it as the next frame's baseline.
    let patches = diff::diff(&real_dom, &virtual_dom);
    renderer.draw(&patches)?;
    real_dom = virtual_dom.clone();

    Ok(())
}
```

### Using it from C/C++

The recommended way to consume `rsact-ffi` from CMake is `FetchContent`, backed by [Corrosion](https://github.com/corrosion-rs/corrosion). Corrosion drives `cargo build` for you and exposes the result as a normal CMake target — no manual `cbindgen` step, no hunting for the built static lib.

> **Prerequisite:** a Rust toolchain (`cargo`/`rustc`) must already be installed and on `PATH`. Corrosion orchestrates `cargo` on your behalf, but it does not install Rust itself — the only thing it provisions automatically via `rustup` is a missing cross-compilation *target*, not the toolchain.

```cmake
include(FetchContent)
FetchContent_Declare(
    rsact
    GIT_REPOSITORY https://github.com/MovingJu/Rsact.git
    GIT_TAG <tag-or-commit>   # no tagged releases yet — pin to a commit for now
)
FetchContent_MakeAvailable(rsact)

target_link_libraries(my_app PRIVATE rsact::rsact_ffi)
```

`rsact::rsact_ffi` carries both the built library and the `cbindgen`-generated header as an interface include directory, so `#include "rsact.h"` just works — no manual `-I` flag needed.

[`examples/c/CMakeLists.txt`](https://github.com/MovingJu/Rsact/blob/main/examples/c/CMakeLists.txt) takes a different, dependency-free route: it skips Corrosion/`cargo` entirely and downloads the prebuilt `rsact-ffi` static lib + header for your platform straight from the matching [GitHub Release](https://github.com/MovingJu/Rsact/releases), so building it needs no Rust toolchain at all — just `cmake -S examples/c -B build && cmake --build build`. It covers the targets Rsact's release workflow publishes: `x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, and `x86_64-pc-windows-msvc`.

<details>
<summary>Building manually with <code>cargo</code> instead of CMake</summary>

```sh
git clone https://github.com/MovingJu/Rsact
cd Rsact
cargo build --release -p rsact-ffi   # produces target/release/librsact_ffi.a and include/rsact.h

cc -I crates/rsact-ffi/include your_app.c target/release/librsact_ffi.a -o your_app
```
</details>

Either way, the C-side usage looks the same:

```c
#include "rsact.h"

int main(void) {
    RsactHandle *h = rsact_create(40, 10); /* also enters raw mode */
    if (!h) return 1;

    for (int col = 0; col < 40; col++) {
        rsact_set_cell(h, 0, col, '-', 0xffffff, 0x000000, 0);
    }
    rsact_render(h);

    rsact_destroy(h); /* also restores the terminal */
    return 0;
}
```

The full C ABI surface is in [`crates/rsact-ffi/include/rsact.h`](https://github.com/MovingJu/Rsact/blob/main/crates/rsact-ffi/include/rsact.h) — `rsact_create` / `rsact_terminal_size` / `rsact_set_cell` / `rsact_render` / `rsact_poll_key` / `rsact_destroy`, plus the `RsactKeyEvent` struct and `RSACT_KEY_*` constants.

## How it works (the render pipeline)

```text
virtual_dom.set(row, col, cell)   draw the frame you want into the virtual buffer
        │
        ▼
diff::diff(&real_dom, &virtual_dom)   scan both buffers row by row,
        │                              merge changed columns into Patch runs
        ▼
Renderer::draw(&patches)   one cursor move per Patch
        │                   + one SGR sequence only when the style changes
        │                   + characters written as UTF-8
        ▼
one write_all() call flushes the whole frame to the terminal
        │
        ▼
real_dom = virtual_dom.clone()   commit as the baseline for the next diff
```

Both `rsact-demo` and `rsact-ffi`'s `rsact_render` follow exactly these five steps.

## Examples

| Run it with | What it shows |
|---|---|
| `cargo run -p rsact-demo` | An animated growing rectangle, using `rsact-core` alone |
| `cargo run --example basic -p rsact-ffi` | A single static frame (a horizontal line on row 0), then exits |
| `cargo run --example animate -p rsact-ffi` | A `#` character moving across row 5 (~16ms/frame) |
| `cargo run --example tree -p rsact-ffi` | A two-counter `Dashboard`, built purely through the C-style `rsact_element_*`/`rsact_tree_*` API |
| `examples/c/src/basic.c` · `animate.c` · `input.c` | The same two examples, plus key-input polling, reproduced in plain C against `rsact-ffi`'s header. [`examples/c/CMakeLists.txt`](https://github.com/MovingJu/Rsact/blob/main/examples/c/CMakeLists.txt) builds all three against a prebuilt `rsact-ffi` downloaded from the matching GitHub Release |
| `examples/c/src/tree.c` | The same `Dashboard` as `examples/tree.rs`, in plain C. Written, and compiles/links/runs against a locally-built `rsact-ffi`, but **not yet wired into `CMakeLists.txt`** — `examples/c` links a prebuilt release binary that predates these functions; it'll be added as a CMake target once a release ships them |

## Development

```sh
cargo build --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

- **MSRV**: the workspace uses `edition = "2024"`, which requires Rust 1.85 or newer.
- `rsact-core` currently has 46 unit tests — buffer bounds checks, a seeded property test proving that applying `diff`'s patches reconstructs the next frame exactly (using a tiny hand-rolled xorshift32 PRNG, no external dependency), and coverage for the renderer not re-emitting SGR codes unnecessarily.
- Commit and PR conventions for contributors are documented in [`CONTRIBUTING.md`](https://github.com/MovingJu/Rsact/blob/main/CONTRIBUTING.md).

## CI

CI in this repository doesn't run builds or tests automatically yet — it's focused on **enforcing commit and PR title conventions**. `cargo build` / `test` / `clippy` are run locally before opening a PR, per the checklist in `CONTRIBUTING.md`.

| Workflow | File | Trigger | What it does |
|---|---|---|---|
| **Commit Messages** | [`commitlint.yml`](https://github.com/MovingJu/Rsact/blob/main/.github/workflows/commitlint.yml) | PR opened/edited/synchronize/reopened | Lints every commit message in the PR with [commitlint](https://commitlint.js.org/) (`@commitlint/config-conventional`, plus a 72-char header limit and a lowercase-subject rule) |
| **PR Title** | [`pr-title.yml`](https://github.com/MovingJu/Rsact/blob/main/.github/workflows/pr-title.yml) | PR opened/edited/synchronize/reopened | Checks that the PR title follows `type(scope): summary` (Conventional Commits) with a lowercase subject, via [`amannn/action-semantic-pull-request`](https://github.com/amannn/action-semantic-pull-request) — the title becomes the squash-merge commit message |

Both workflows accept the types `feat` / `fix` / `docs` / `style` / `refactor` / `perf` / `test` / `build` / `ci` / `chore` / `revert`.

> The PR Title workflow failed with `Resource not accessible by integration` twice: first right after [#5](https://github.com/MovingJu/Rsact/pull/5) was merged (its `pull_request_target` trigger lacked `pull-requests: write`, granted in [`9c0b70b`](https://github.com/MovingJu/Rsact/commit/9c0b70b)), then again on [#7](https://github.com/MovingJu/Rsact/pull/7) — `amannn/action-semantic-pull-request` also posts a commit status, which needs `statuses: write` specifically, not covered by `pull-requests: write`. Both permissions are granted now.

## Roadmap

Summarized from the issue tracker:

- [x] **v0.1 MVP** — cell-buffer virtual DOM diff/render engine + C/C++ FFI ([#1](https://github.com/MovingJu/Rsact/issues/1)) — this is the state of the repository today.
- [x] Fix: cursor not restored to a known position on exit ([#4](https://github.com/MovingJu/Rsact/issues/4), resolved by [#5](https://github.com/MovingJu/Rsact/pull/5))
- [ ] **v0.2.0** — component tree: declarative composition + reconciliation ([#2](https://github.com/MovingJu/Rsact/issues/2))
- [ ] **v0.3.0** — maximize performance of the diff/render pipeline, with opt-in multithreading ([#3](https://github.com/MovingJu/Rsact/issues/3))
- [x] CMake `FetchContent` integration — using [Corrosion](https://github.com/corrosion-rs/corrosion) so C/C++ consumers never need to know about `cargo build` or `cbindgen` directly ([#6](https://github.com/MovingJu/Rsact/issues/6))

Check the [issue tracker](https://github.com/MovingJu/Rsact/issues) for the latest status.

## Contributing

Issues and PRs are welcome. Before opening a PR, please read the commit/PR title conventions in [`CONTRIBUTING.md`](https://github.com/MovingJu/Rsact/blob/main/CONTRIBUTING.md) (Conventional Commits, enforced by CI).

1. Branch off `main`.
2. Make sure `cargo build --workspace && cargo test --workspace && cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings` all pass.
3. Open a PR with a `type(scope): summary`-formatted title.

## License

[MIT License](https://github.com/MovingJu/Rsact/blob/main/LICENSE) © 2026 [MovingJu](https://github.com/MovingJu)
