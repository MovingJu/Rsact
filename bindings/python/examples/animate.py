#!/usr/bin/env python3
"""Moves a single '#' across row 5, one frame every ~16ms.

Mirrors examples/c/src/animate.c and rsact-ffi's animate.rs example.

Run with: python3 bindings/python/examples/animate.py
(from the repo root, after `cargo build --release -p rsact-ffi`)
"""

import os
import sys
import time

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from rsact import Terminal


def main() -> None:
    with Terminal(40, 10) as term:
        for frame in range(100):
            term.set_cell(5, frame % 40, "#")
            term.render()
            time.sleep(0.016)


if __name__ == "__main__":
    main()
