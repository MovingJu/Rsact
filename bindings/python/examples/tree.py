#!/usr/bin/env python3
"""Builds a small two-counter dashboard through the component-tree API,
rebuilding the element tree each frame — there's no Component trait to
implement in Python, same as the C binding.

Mirrors examples/c/src/tree.c and rsact-ffi's tree.rs example.

Run with: python3 bindings/python/examples/tree.py
(from the repo root, after `cargo build --release -p rsact-ffi`)
"""

import os
import sys
import time

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))

from rsact import Element, RSACT_LAYOUT_HORIZONTAL, RSACT_LAYOUT_VERTICAL, Tree


def counter_element(label: str, count: int) -> Element:
    return Element.container(
        label,
        RSACT_LAYOUT_VERTICAL,
        [
            Element.text("label", label, width=20),
            Element.text("value", str(count), width=20),
        ],
        width=20,
        height=2,
    )


def dashboard_element(left_count: int, right_count: int) -> Element:
    return Element.container(
        "dashboard",
        RSACT_LAYOUT_HORIZONTAL,
        [counter_element("left", left_count), counter_element("right", right_count)],
        width=40,
        height=2,
    )


def main() -> None:
    with Tree(40, 2) as tree:
        for frame in range(5):
            tree.present(dashboard_element(frame, 42))
            time.sleep(0.5)


if __name__ == "__main__":
    main()
