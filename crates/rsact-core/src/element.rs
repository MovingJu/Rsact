// element.rs — the declarative tree components render into, sitting above
// `Buffer`. `Component::render` returns an `Element`; the `Tree` reconciles
// consecutive `Element` trees and only writes the cells that actually
// changed into the underlying `Buffer`.
use crate::cell::Style;

/// Stable identity for a child within its parent's children list. Reordering
/// keeps a child's `Key` attached to it, so the reconciler can tell "moved"
/// apart from "removed + added" even when position in the list changes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Key(pub String);

impl Key {
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }
}

impl From<&str> for Key {
    fn from(value: &str) -> Self {
        Key::new(value)
    }
}

impl From<String> for Key {
    fn from(value: String) -> Self {
        Key::new(value)
    }
}

/// A screen-space rectangle assigned to an element during layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub row: u16,
    pub col: u16,
    pub width: u16,
    pub height: u16,
}

/// How a container arranges its children along its own rect. No
/// flexbox-style constraint solving (see #2's non-goals) — each child
/// declares its own extent along the stack axis and simply follows the
/// previous one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Vertical,
    Horizontal,
}

/// A single-line run of styled text. Only the first row of the element's
/// assigned rect is painted; content is clipped or space-padded to `width`
/// so a shorter replacement string doesn't leave stale characters behind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text {
    pub content: String,
    pub style: Style,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementKind {
    Text(Text),
    Container {
        layout: Layout,
        children: Vec<Element>,
    },
}

/// A node in the declarative tree. `width`/`height` are the extent this
/// element asks its parent for along the parent's layout axis (the other
/// axis fills the parent's own extent) — see `layout_children`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    pub key: Key,
    pub width: u16,
    pub height: u16,
    pub kind: ElementKind,
}

impl Element {
    pub fn text(key: impl Into<Key>, width: u16, content: impl Into<String>, style: Style) -> Self {
        Self {
            key: key.into(),
            width,
            height: 1,
            kind: ElementKind::Text(Text {
                content: content.into(),
                style,
            }),
        }
    }

    pub fn container(
        key: impl Into<Key>,
        layout: Layout,
        width: u16,
        height: u16,
        children: Vec<Element>,
    ) -> Self {
        Self {
            key: key.into(),
            width,
            height,
            kind: ElementKind::Container { layout, children },
        }
    }
}

/// Assigns each child of a container a `Rect` inside `parent_rect`,
/// stacking along `layout`'s axis in order and filling the cross axis.
/// Children that would overflow `parent_rect` still get a rect (clipping to
/// the physical buffer, if any, is `Buffer::set`'s job) — v0.2 does no
/// scrolling/overflow handling.
pub(crate) fn layout_children(
    layout: Layout,
    children: &[Element],
    parent_rect: Rect,
) -> Vec<Rect> {
    let mut rects = Vec::with_capacity(children.len());
    let mut cursor_row = parent_rect.row;
    let mut cursor_col = parent_rect.col;
    for child in children {
        let rect = match layout {
            Layout::Vertical => Rect {
                row: cursor_row,
                col: parent_rect.col,
                width: parent_rect.width,
                height: child.height,
            },
            Layout::Horizontal => Rect {
                row: parent_rect.row,
                col: cursor_col,
                width: child.width,
                height: parent_rect.height,
            },
        };
        match layout {
            Layout::Vertical => cursor_row += child.height,
            Layout::Horizontal => cursor_col += child.width,
        }
        rects.push(rect);
    }
    rects
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertical_layout_stacks_children_top_to_bottom_at_full_width() {
        let parent = Rect {
            row: 2,
            col: 3,
            width: 20,
            height: 10,
        };
        let children = vec![
            Element::text("a", 5, "a", Style::default()),
            Element::container("b", Layout::Horizontal, 5, 4, vec![]),
        ];

        let rects = layout_children(Layout::Vertical, &children, parent);

        assert_eq!(
            rects[0],
            Rect {
                row: 2,
                col: 3,
                width: 20,
                height: 1
            }
        );
        assert_eq!(
            rects[1],
            Rect {
                row: 3,
                col: 3,
                width: 20,
                height: 4
            }
        );
    }

    #[test]
    fn horizontal_layout_stacks_children_left_to_right_at_full_height() {
        let parent = Rect {
            row: 0,
            col: 0,
            width: 20,
            height: 5,
        };
        let children = vec![
            Element::text("a", 4, "a", Style::default()),
            Element::text("b", 6, "b", Style::default()),
        ];

        let rects = layout_children(Layout::Horizontal, &children, parent);

        assert_eq!(
            rects[0],
            Rect {
                row: 0,
                col: 0,
                width: 4,
                height: 5
            }
        );
        assert_eq!(
            rects[1],
            Rect {
                row: 0,
                col: 4,
                width: 6,
                height: 5
            }
        );
    }
}
