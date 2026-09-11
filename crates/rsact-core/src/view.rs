// view.rs — a light `macro_rules!` DSL for building `Element` trees
// declaratively, without pulling in a proc-macro crate (syn/quote). A tree
// built with `view!` reads top-to-bottom as the shape it produces, instead
// of the nesting getting buried inside `vec![]` calls and builder chains.

/// Builds an [`Element`](crate::element::Element) tree declaratively:
///
/// ```text
/// view!(
///     container(key, layout, width = w, height = h) {
///         text(key, content, width = w, style = s),
///         container(...) { ... },
///         some_expression_that_returns_an_element,
///     }
/// )
/// ```
///
/// `width`/`height`/`style` are all optional, in any combination — omit
/// whichever you don't need. A container's children are a comma-separated
/// list of nested `text(...)`/`container(...) { ... }` forms (no `view!`
/// prefix needed on each child, it's implied) — or any other expression
/// that evaluates to an `Element`, which is spliced in as-is. That last
/// part is what lets a container mix its own literal children with a
/// nested `Component`'s own `render()` output.
///
/// # Examples
/// ```
/// use rsact_core::element::Layout;
/// use rsact_core::view;
///
/// let label = "left";
/// let count = 0u32;
///
/// let tree = view!(
///     container(label, Layout::Vertical, width = 20, height = 2) {
///         text("label", label, width = 20),
///         text("value", count.to_string(), width = 20),
///     }
/// );
///
/// assert_eq!(tree.width, 20);
/// assert_eq!(tree.height, 2);
/// ```
#[macro_export]
macro_rules! view {
    (text($key:expr, $content:expr $(, width = $width:expr)? $(, height = $height:expr)? $(, style = $style:expr)?)) => {{
        #[allow(unused_mut)]
        let mut element = $crate::element::Element::text($key, $content);
        $( element = element.width($width); )?
        $( element = element.height($height); )?
        $( element = element.style($style); )?
        element
    }};
    (container($key:expr, $layout:expr $(, width = $width:expr)? $(, height = $height:expr)?) { $($children:tt)* }) => {{
        #[allow(unused_mut)]
        let mut element = $crate::element::Element::container(
            $key,
            $layout,
            $crate::__view_children!([] $($children)*),
        );
        $( element = element.width($width); )?
        $( element = element.height($height); )?
        element
    }};
}

/// Implementation detail of [`view!`]: a tt-muncher that accumulates a
/// container's children into a `Vec<Element>`, recognizing `text(...)`/
/// `container(...) { ... }` forms specially and falling back to treating
/// anything else as a plain expression. Not meant to be invoked directly.
#[doc(hidden)]
#[macro_export]
macro_rules! __view_children {
    ([$($acc:expr,)*]) => {
        vec![$($acc),*]
    };
    ([$($acc:expr,)*] text($($args:tt)*) $(, $($rest:tt)*)?) => {
        $crate::__view_children!([$($acc,)* $crate::view!(text($($args)*)),] $($($rest)*)?)
    };
    ([$($acc:expr,)*] container($($args:tt)*) { $($body:tt)* } $(, $($rest:tt)*)?) => {
        $crate::__view_children!([$($acc,)* $crate::view!(container($($args)*) { $($body)* }),] $($($rest)*)?)
    };
    ([$($acc:expr,)*] $child:expr $(, $($rest:tt)*)?) => {
        $crate::__view_children!([$($acc,)* $child,] $($($rest)*)?)
    };
}

#[cfg(test)]
mod tests {
    use crate::cell::Style;
    use crate::element::{Element, Layout};

    #[test]
    fn text_leaf_matches_the_builder_equivalent() {
        let via_macro = view!(text("a", "hi", width = 10));
        let via_builder = Element::text("a", "hi").width(10);
        assert_eq!(via_macro, via_builder);
    }

    #[test]
    fn text_leaf_supports_style() {
        let style = Style {
            bold: true,
            ..Style::default()
        };
        let via_macro = view!(text("a", "hi", width = 10, style = style));
        let via_builder = Element::text("a", "hi").width(10).style(style);
        assert_eq!(via_macro, via_builder);
    }

    #[test]
    fn container_defaults_are_zero_when_omitted() {
        let via_macro = view!(container("root", Layout::Vertical) {});
        let via_builder = Element::container("root", Layout::Vertical, vec![]);
        assert_eq!(via_macro, via_builder);
    }

    #[test]
    fn nested_container_matches_the_builder_equivalent() {
        let via_macro = view!(
            container("root", Layout::Vertical, width = 20, height = 2) {
                text("label", "left", width = 20),
                text("value", "0", width = 20),
            }
        );
        let via_builder = Element::container(
            "root",
            Layout::Vertical,
            vec![
                Element::text("label", "left").width(20),
                Element::text("value", "0").width(20),
            ],
        )
        .width(20)
        .height(2);
        assert_eq!(via_macro, via_builder);
    }

    #[test]
    fn containers_can_nest_three_levels_deep() {
        let via_macro = view!(
            container("root", Layout::Vertical, width = 10, height = 2) {
                container("row", Layout::Horizontal, width = 10, height = 1) {
                    text("a", "x", width = 5),
                    text("b", "y", width = 5),
                },
            }
        );
        let via_builder = Element::container(
            "root",
            Layout::Vertical,
            vec![
                Element::container(
                    "row",
                    Layout::Horizontal,
                    vec![
                        Element::text("a", "x").width(5),
                        Element::text("b", "y").width(5),
                    ],
                )
                .width(10)
                .height(1),
            ],
        )
        .width(10)
        .height(2);
        assert_eq!(via_macro, via_builder);
    }

    #[test]
    fn a_child_can_be_an_arbitrary_expression_returning_an_element() {
        fn render_leaf() -> Element {
            Element::text("spliced", "x").width(5)
        }

        let via_macro = view!(
            container("root", Layout::Horizontal, width = 10, height = 1) {
                render_leaf(),
                text("b", "y", width = 5),
            }
        );
        let via_builder = Element::container(
            "root",
            Layout::Horizontal,
            vec![render_leaf(), Element::text("b", "y").width(5)],
        )
        .width(10)
        .height(1);
        assert_eq!(via_macro, via_builder);
    }
}
