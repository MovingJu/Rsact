#!/usr/bin/env python3
"""Builds a small two-counter dashboard through the component-tree API,
rebuilding the tree each frame as plain, immutable data — there's no
Component trait to implement in Python, same as the C binding, but no
native handles to manage either.

Mirrors examples/c/src/tree.c and rsact-ffi's tree.rs example.

Run with: uv run examples/python/tree.py
(from the repo root, after `cargo build --release -p rsact-ffi`)
"""

import time

from rsact import RSACT_LAYOUT_HORIZONTAL, RSACT_LAYOUT_VERTICAL, Node, Tree, container, text


def counter(label: str, count: int) -> Node:
    return container(
        label,
        RSACT_LAYOUT_VERTICAL,
        [text("label", label, width=20), text("value", str(count), width=20)],
        width=20,
        height=2,
    )


def dashboard(left_count: int, right_count: int) -> Node:
    return container(
        "dashboard",
        RSACT_LAYOUT_HORIZONTAL,
        [counter("left", left_count), counter("right", right_count)],
        width=40,
        height=2,
    )


def main() -> None:
    with Tree(40, 2) as tree:
        for frame in range(5):
            tree.present(dashboard(frame, 42))
            time.sleep(0.5)


if __name__ == "__main__":
    main()
