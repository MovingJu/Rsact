#!/usr/bin/env python3
"""Draws a single static frame: a horizontal line across row 0.

Mirrors examples/c/src/basic.c and rsact-ffi's basic.rs example.

Run with: uv run examples/python/basic.py
(from the repo root, after `cargo build --release -p rsact-ffi`)
"""

import time

from rsact import Terminal


def main() -> None:
    with Terminal(40, 10) as term:
        for col in range(40):
            term.set_cell(0, col, "-")
        term.render()
        time.sleep(2)


if __name__ == "__main__":
    main()
