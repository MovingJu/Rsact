// tree.rs — drives one frame: `render -> reconcile -> write into Buffer`.
// The cell-level `diff`/`Renderer` from v0.1 are unchanged and untouched by
// this module; a caller still runs those over the `Buffer` this module
// writes into to get bytes to send to the terminal.
use std::collections::HashMap;

use crate::buffer::Buffer;
use crate::cell::Cell;
use crate::component::Component;
use crate::element::{Element, ElementKind, Key, Rect, layout_children};

/// The write side of painting an element tree. `Buffer` is the only real
/// implementer; tests substitute a counting wrapper to verify that
/// reconciling an unchanged subtree performs zero writes.
pub(crate) trait Paint {
    fn paint(&mut self, row: u16, col: u16, cell: Cell);
}

impl Paint for Buffer {
    fn paint(&mut self, row: u16, col: u16, cell: Cell) {
        self.set(row, col, cell);
    }
}

/// Owns a `Component`, the persistent `Buffer` it paints into, and the
/// previous frame's `Element` tree so each call to `frame` only repaints
/// what changed.
pub struct Tree<C: Component> {
    root: C,
    buffer: Buffer,
    prev: Option<(Element, Rect)>,
}

impl<C: Component> Tree<C> {
    pub fn new(root: C, width: u16, height: u16) -> Self {
        Self {
            root,
            buffer: Buffer::new(width, height),
            prev: None,
        }
    }

    /// Renders the root component, reconciles it against the previous
    /// frame's tree, paints only what changed into the internal `Buffer`,
    /// and returns that `Buffer` so the caller can run it through v0.1's
    /// `diff`/`Renderer` pipeline unchanged.
    pub fn frame(&mut self) -> &Buffer {
        let next = self.root.render();
        let rect = Rect {
            row: 0,
            col: 0,
            width: self.buffer.width(),
            height: self.buffer.height(),
        };
        let prev = self.prev.as_ref().map(|(elem, rect)| (elem, *rect));
        reconcile_paint(&next, rect, prev, &mut self.buffer);
        self.prev = Some((next, rect));
        &self.buffer
    }

    pub fn root(&self) -> &C {
        &self.root
    }

    pub fn root_mut(&mut self) -> &mut C {
        &mut self.root
    }
}

/// Recursively paints `next` (assigned `next_rect`) into `sink`, skipping
/// any subtree that is byte-for-byte identical (content *and* rect) to the
/// matching node in the previous tree. Matching between a container's
/// previous and next children is by `Key`, not position, so a reordered
/// keyed child is recognized as "moved" rather than repainted as a fresh
/// add + a stale removal.
pub(crate) fn reconcile_paint<P: Paint>(
    next: &Element,
    next_rect: Rect,
    prev: Option<(&Element, Rect)>,
    sink: &mut P,
) {
    if let Some((prev_elem, prev_rect)) = prev
        && prev_elem == next
        && prev_rect == next_rect
    {
        return;
    }

    match &next.kind {
        ElementKind::Text(text) => {
            let mut chars = text.content.chars();
            for col_offset in 0..next_rect.width {
                let ch = chars.next().unwrap_or(' ');
                sink.paint(
                    next_rect.row,
                    next_rect.col + col_offset,
                    Cell {
                        ch,
                        style: text.style,
                    },
                );
            }
        }
        ElementKind::Container { layout, children } => {
            let next_rects = layout_children(*layout, children, next_rect);

            let prev_children: HashMap<&Key, (&Element, Rect)> = match prev {
                Some((
                    Element {
                        kind:
                            ElementKind::Container {
                                layout: prev_layout,
                                children: prev_children,
                            },
                        ..
                    },
                    prev_rect,
                )) => {
                    let prev_rects = layout_children(*prev_layout, prev_children, prev_rect);
                    prev_children
                        .iter()
                        .zip(prev_rects)
                        .map(|(child, rect)| (&child.key, (child, rect)))
                        .collect()
                }
                _ => HashMap::new(),
            };

            for (child, rect) in children.iter().zip(next_rects) {
                let prev_for_child = prev_children.get(&child.key).copied();
                reconcile_paint(child, rect, prev_for_child, sink);
            }

            let next_keys: std::collections::HashSet<&Key> =
                children.iter().map(|child| &child.key).collect();
            for (key, (_, rect)) in prev_children {
                if !next_keys.contains(key) {
                    clear_rect(rect, sink);
                }
            }
        }
    }
}

fn clear_rect<P: Paint>(rect: Rect, sink: &mut P) {
    for row_offset in 0..rect.height {
        for col_offset in 0..rect.width {
            sink.paint(
                rect.row + row_offset,
                rect.col + col_offset,
                Cell::default(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::element::Layout;

    /// Wraps a `Buffer`, counting every `paint` call so tests can assert
    /// that an unchanged subtree triggers zero writes, not just that the
    /// resulting content happens to match.
    struct CountingSink<'a> {
        buffer: &'a mut Buffer,
        calls: usize,
    }

    impl Paint for CountingSink<'_> {
        fn paint(&mut self, row: u16, col: u16, cell: Cell) {
            self.calls += 1;
            self.buffer.set(row, col, cell);
        }
    }

    fn full_rect(buffer: &Buffer) -> Rect {
        Rect {
            row: 0,
            col: 0,
            width: buffer.width(),
            height: buffer.height(),
        }
    }

    fn text_at(row: u16, col: u16, width: u16, buffer: &Buffer) -> String {
        (0..width).map(|c| buffer.get(row, col + c).ch).collect()
    }

    #[test]
    fn first_frame_with_no_prev_paints_every_leaf() {
        let mut buffer = Buffer::new(10, 2);
        let tree = Element::container(
            "root",
            Layout::Vertical,
            vec![
                Element::text("a", "hello").width(10),
                Element::text("b", "world").width(10),
            ],
        )
        .width(10)
        .height(2);
        let rect = full_rect(&buffer);

        reconcile_paint(&tree, rect, None, &mut buffer);

        assert_eq!(text_at(0, 0, 5, &buffer), "hello");
        assert_eq!(text_at(1, 0, 5, &buffer), "world");
    }

    #[test]
    fn unchanged_subtree_performs_zero_paint_calls() {
        let mut buffer = Buffer::new(10, 2);
        let make_tree = || {
            Element::container(
                "root",
                Layout::Vertical,
                vec![
                    Element::text("a", "static").width(10),
                    Element::text("b", "changes").width(10),
                ],
            )
            .width(10)
            .height(2)
        };
        let rect = full_rect(&buffer);
        let first = make_tree();
        reconcile_paint(&first, rect, None, &mut buffer);

        let second = Element::container(
            "root",
            Layout::Vertical,
            vec![
                Element::text("a", "static").width(10),
                Element::text("b", "changed!").width(10),
            ],
        )
        .width(10)
        .height(2);

        let mut sink = CountingSink {
            buffer: &mut buffer,
            calls: 0,
        };
        reconcile_paint(&second, rect, Some((&first, rect)), &mut sink);

        // only "b" (width 10) should repaint; "a" must not trigger any
        // paint() calls at all.
        assert_eq!(
            sink.calls, 10,
            "expected exactly one child's cells repainted"
        );
        assert_eq!(text_at(0, 0, 6, &buffer), "static");
        assert_eq!(text_at(1, 0, 8, &buffer), "changed!");
    }

    #[test]
    fn reordering_keyed_children_is_recognized_as_a_move_not_a_remove_and_add() {
        let mut buffer = Buffer::new(10, 2);
        let rect = full_rect(&buffer);
        let first = Element::container(
            "root",
            Layout::Vertical,
            vec![
                Element::text("a", "AAA").width(10),
                Element::text("b", "BBB").width(10),
            ],
        )
        .width(10)
        .height(2);
        reconcile_paint(&first, rect, None, &mut buffer);

        // swap order: "b" now first, "a" now second. Neither child's own
        // content changed, only its position — since matching is by key,
        // this must repaint both (their rects changed) but must not clear
        // either as "removed".
        let second = Element::container(
            "root",
            Layout::Vertical,
            vec![
                Element::text("b", "BBB").width(10),
                Element::text("a", "AAA").width(10),
            ],
        )
        .width(10)
        .height(2);
        reconcile_paint(&second, rect, Some((&first, rect)), &mut buffer);

        assert_eq!(text_at(0, 0, 3, &buffer), "BBB");
        assert_eq!(text_at(1, 0, 3, &buffer), "AAA");
    }

    #[test]
    fn removed_keyed_child_has_its_rect_cleared() {
        let mut buffer = Buffer::new(10, 2);
        let rect = full_rect(&buffer);
        let first = Element::container(
            "root",
            Layout::Vertical,
            vec![
                Element::text("a", "AAA").width(10),
                Element::text("b", "BBB").width(10),
            ],
        )
        .width(10)
        .height(2);
        reconcile_paint(&first, rect, None, &mut buffer);

        let second = Element::container(
            "root",
            Layout::Vertical,
            vec![Element::text("a", "AAA").width(10)],
        )
        .width(10)
        .height(2);
        reconcile_paint(&second, rect, Some((&first, rect)), &mut buffer);

        assert_eq!(text_at(0, 0, 3, &buffer), "AAA");
        assert_eq!(text_at(1, 0, 3, &buffer), "   ");
    }

    #[test]
    fn added_keyed_child_is_painted() {
        let mut buffer = Buffer::new(10, 2);
        let rect = full_rect(&buffer);
        let first = Element::container(
            "root",
            Layout::Vertical,
            vec![Element::text("a", "AAA").width(10)],
        )
        .width(10)
        .height(1);
        reconcile_paint(&first, rect, None, &mut buffer);

        let second = Element::container(
            "root",
            Layout::Vertical,
            vec![
                Element::text("a", "AAA").width(10),
                Element::text("b", "BBB").width(10),
            ],
        )
        .width(10)
        .height(2);
        reconcile_paint(&second, rect, Some((&first, rect)), &mut buffer);

        assert_eq!(text_at(0, 0, 3, &buffer), "AAA");
        assert_eq!(text_at(1, 0, 3, &buffer), "BBB");
    }

    struct Counter {
        count: u32,
    }

    impl Component for Counter {
        fn render(&self) -> Element {
            Element::container(
                "counter",
                Layout::Vertical,
                vec![
                    Element::text("label", "count:").width(20),
                    Element::text("value", self.count.to_string()).width(20),
                ],
            )
            .width(20)
            .height(2)
        }
    }

    struct App {
        left: Counter,
        right: Counter,
    }

    impl Component for App {
        fn render(&self) -> Element {
            Element::container(
                "app",
                Layout::Horizontal,
                vec![
                    {
                        let mut e = self.left.render();
                        e.key = Key::new("left");
                        e
                    },
                    {
                        let mut e = self.right.render();
                        e.key = Key::new("right");
                        e
                    },
                ],
            )
            .width(40)
            .height(2)
        }
    }

    #[test]
    fn tree_drives_two_nested_components_with_independent_local_state() {
        let mut tree = Tree::new(
            App {
                left: Counter { count: 0 },
                right: Counter { count: 100 },
            },
            40,
            2,
        );

        let buffer = tree.frame();
        assert_eq!(text_at(1, 0, 1, buffer), "0");
        assert_eq!(text_at(1, 20, 3, buffer), "100");

        tree.root_mut().left.count += 1;
        let buffer = tree.frame();
        assert_eq!(
            text_at(1, 0, 1, buffer),
            "1",
            "left counter's own state advanced"
        );
        assert_eq!(
            text_at(1, 20, 3, buffer),
            "100",
            "right counter's state is independent and untouched"
        );
    }
}
