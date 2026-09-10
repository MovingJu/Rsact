// component.rs — the trait consumers implement to describe a UI as a tree.
use crate::element::Element;

/// Something that can describe itself as an `Element` subtree. Rsact drives
/// components by calling `render` once per frame and reconciling the result
/// against the previous frame's tree (see `tree::Tree`).
pub trait Component {
    fn render(&self) -> Element;
}
