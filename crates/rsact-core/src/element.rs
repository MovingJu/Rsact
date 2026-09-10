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
    /// A single-line text leaf. `width`/`height` default to `0`/`1`; chain
    /// `.width(..)` and `.style(..)` to set them — see the builder methods
    /// below.
    pub fn text(key: impl Into<Key>, content: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            width: 0,
            height: 1,
            kind: ElementKind::Text(Text {
                content: content.into(),
                style: Style::default(),
            }),
        }
    }

    /// A container stacking `children` along `layout`'s axis. `width`/
    /// `height` default to `0`; chain `.width(..)`/`.height(..)` to size it
    /// for its own parent's layout.
    pub fn container(key: impl Into<Key>, layout: Layout, children: Vec<Element>) -> Self {
        Self {
            key: key.into(),
            width: 0,
            height: 0,
            kind: ElementKind::Container { layout, children },
        }
    }

    /// Sets the extent this element asks its parent for along the parent's
    /// stack axis (see `layout_children`).
    pub fn width(mut self, width: u16) -> Self {
        self.width = width;
        self
    }

    /// Sets the extent this element asks its parent for along the parent's
    /// stack axis (see `layout_children`). For a `Container`, also overrides
    /// the height children are stacked against on their cross axis.
    pub fn height(mut self, height: u16) -> Self {
        self.height = height;
        self
    }

    /// Sets the style of a `Text` leaf. A no-op on a `Container`.
    pub fn style(mut self, style: Style) -> Self {
        if let ElementKind::Text(text) = &mut self.kind {
            text.style = style;
        }
        self
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
            Element::text("a", "a").width(5),
            Element::container("b", Layout::Horizontal, vec![])
                .width(5)
                .height(4),
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
            Element::text("a", "a").width(4),
            Element::text("b", "b").width(6),
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
