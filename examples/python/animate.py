#!/usr/bin/env python3
"""Moves a single '#' across row 5, one frame every ~16ms.

Mirrors examples/c/src/animate.c and rsact-ffi's animate.rs example.

Run with: uv run examples/python/animate.py
(from the repo root, after `cargo build --release -p rsact-ffi`)
"""

import time

from rsact import Terminal


def main() -> None:
    # No `with`: Terminal restores the terminal on garbage collection via
    # Rust's own Drop (RawModeGuard) — see basic.py's comment.
    term = Terminal(40, 10)
    for frame in range(100):
        term.set_cell(5, frame % 40, "#")
        term.render()
        time.sleep(0.016)


if __name__ == "__main__":
    main()
