# rsact-core

[![crates.io](https://img.shields.io/crates/v/rsact-core.svg)](https://crates.io/crates/rsact-core)
[![docs.rs](https://docs.rs/rsact-core/badge.svg)](https://docs.rs/rsact-core)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/MovingJu/Rsact/blob/main/LICENSE)

**The virtual terminal buffer, diffing, and rendering engine behind [Rsact](https://github.com/MovingJu/Rsact).**

You draw the state you want into a virtual buffer — a grid of cells, each a character plus a style. `rsact-core` diffs that buffer against the last frame it actually drew, and turns only the cells that changed into the minimal set of ANSI escape sequences needed, sent to the terminal in a **single `write()` call**.

> This crate is not yet stable — see the [project roadmap](https://github.com/MovingJu/Rsact#roadmap).

## Features

- **No TUI dependencies at all** — no `crossterm`, no `ncurses`. Raw-mode entry/exit, cursor movement, and color/style escape sequences are all implemented directly here. The only dependencies are one per platform: `libc` (termios) on Unix, `windows-sys` (Win32 console API) on Windows.
- **The virtual DOM is just a cell array** — `Buffer` holds `width * height` `Cell { ch: char, style: Style }` values in row-major order, a plain `Vec`, no tree structure.
- **Minimal diff engine** — compares two buffers and returns non-overlapping contiguous `Patch` runs.
- **Batched renderer** — one cursor move per patch, an SGR sequence only when the style actually changes, the whole frame flushed in a single syscall.
- **Cursor restored on exit** — `RawModeGuard::enable_safe_exit` moves the cursor to the terminal's last row/col and emits a trailing newline on `Drop`.
- **UTF-8-aware input parser** — arrow keys, Ctrl+letter, Backspace/Enter/Esc, and multi-byte UTF-8 characters all decode correctly from a single `read_key()` call.

## Quick start

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

## How it works

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

## Learn more

This crate is the core of the [Rsact](https://github.com/MovingJu/Rsact) workspace, which also ships a C ABI (`rsact-ffi`) and CMake-based C examples. See the [project README](https://github.com/MovingJu/Rsact#readme) for the full picture, [CONTRIBUTING.md](https://github.com/MovingJu/Rsact/blob/main/CONTRIBUTING.md) to contribute, and the [issue tracker](https://github.com/MovingJu/Rsact/issues) for the roadmap.

## License

[MIT](https://github.com/MovingJu/Rsact/blob/main/LICENSE) © [MovingJu](https://github.com/MovingJu)
