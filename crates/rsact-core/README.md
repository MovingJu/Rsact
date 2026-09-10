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
- **Declarative component tree** — describe the screen as a tree of `Component`s (`component`/`element`/`tree` modules); `Tree` reconciles consecutive frames by `Key` and repaints only what changed. See [Component tree](#component-tree) below.

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

## Component tree

On top of that flat-buffer pipeline, this crate also has a declarative layer: describe the screen as a tree of `Component`s, and let `Tree` figure out which cells actually need repainting between frames.

```rust,no_run
use rsact_core::{component::Component, element::{Element, Layout}, renderer::Renderer, tree::Tree};

struct Counter { label: &'static str, count: u32 }

impl Component for Counter {
    fn render(&self) -> Element {
        Element::container(
            self.label,
            Layout::Vertical,
            vec![
                Element::text("label", self.label).width(20),
                Element::text("value", self.count.to_string()).width(20),
            ],
        )
        .width(20)
        .height(2)
    }
}

fn main() -> std::io::Result<()> {
    let mut renderer = Renderer::new(std::io::stdout());
    let mut tree = Tree::new(Counter { label: "left", count: 0 }, 20, 2);

    for _ in 0..5 {
        tree.present(&mut renderer)?; // render → reconcile → diff → draw → sync
        tree.root_mut().count += 1;
    }
    Ok(())
}
```

`Tree` keeps the previous frame's tree and reconciles the new one against it by matching children **by `Key`, not list position** — an unchanged subtree costs zero `Buffer` writes, and a reordered keyed child is recognized as "moved" rather than removed-then-re-added. See [`docs/component-tree.md`](https://github.com/MovingJu/Rsact/blob/main/docs/component-tree.md) in the workspace root for the full concepts walkthrough and a nested-tree diagram.

## Learn more

This crate is the core of the [Rsact](https://github.com/MovingJu/Rsact) workspace, which also ships a C ABI (`rsact-ffi`) and CMake-based C examples. See the [project README](https://github.com/MovingJu/Rsact#readme) for the full picture, [CONTRIBUTING.md](https://github.com/MovingJu/Rsact/blob/main/CONTRIBUTING.md) to contribute, and the [issue tracker](https://github.com/MovingJu/Rsact/issues) for the roadmap.

## License

[MIT](https://github.com/MovingJu/Rsact/blob/main/LICENSE) © [MovingJu](https://github.com/MovingJu)
