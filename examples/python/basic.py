#!/usr/bin/env python3
"""Draws a single static frame: a horizontal line across row 0.

Mirrors examples/c/src/basic.c and rsact-ffi's basic.rs example.

Run with: uv run examples/python/basic.py
(from the repo root, after `cargo build --release -p rsact-ffi`)
"""

import time

from rsact import Terminal


def main() -> None:
    # No `with` here on purpose: Terminal restores the terminal via Rust's
    # own Drop (RawModeGuard) when it's garbage-collected, same as falling
    # out of scope in Rust — no context-manager block needed just to get
    # that, so a real, longer-running TUI app doesn't have to nest its
    # whole body one indent level deeper for it.
    term = Terminal(40, 10)
    for col in range(40):
        term.set_cell(0, col, "-")
    term.render()
    time.sleep(2)


if __name__ == "__main__":
    main()
