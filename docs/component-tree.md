# The component tree

`rsact-core`'s v0.1 layer (`Buffer` / `diff` / `Renderer`) is a flat cell
grid — you `set()` cells by hand. The component tree, added in v0.2.0
([#2](https://github.com/MovingJu/Rsact/issues/2)), sits on top of that:
you describe *what* the screen should look like as a tree of components,
and `rsact-core` figures out *which cells* actually need to change.

This doc walks through the pieces (`Element`, `Component`, `Tree`), shows
how a tree is shaped and reconciled, and ends with a full runnable example.

## Concepts

| Type | Role |
|---|---|
| [`Element`](https://github.com/MovingJu/Rsact/blob/main/crates/rsact-core/src/element.rs) | A node in the tree: either a `Text` leaf or a `Container` of children. Built with a fluent builder. |
| [`Key`](https://github.com/MovingJu/Rsact/blob/main/crates/rsact-core/src/element.rs) | Stable identity for a child within its parent's children list — how the reconciler recognizes "this is the same node as last frame," even if its position changed. |
| [`Layout`](https://github.com/MovingJu/Rsact/blob/main/crates/rsact-core/src/element.rs) | How a `Container` stacks its children: `Vertical` or `Horizontal`. No flexbox — each child just declares its own extent along the stack axis. |
| [`Component`](https://github.com/MovingJu/Rsact/blob/main/crates/rsact-core/src/component.rs) | A trait: `fn render(&self) -> Element`. Your app's state lives on the type that implements it. |
| [`Tree`](https://github.com/MovingJu/Rsact/blob/main/crates/rsact-core/src/tree.rs) | Owns a root `Component`, drives one frame at a time, and reconciles each new `Element` tree against the previous one. |

## Shape of a tree

A `Tree` is built from nested `Component`s, each rendering an `Element`.
Here's what `Dashboard { left: Counter, right: Counter }` (from
[`rsact-demo`](https://github.com/MovingJu/Rsact/blob/main/crates/rsact-demo/src/main.rs))
looks like once both `Counter`s have rendered:

```text
Dashboard::render()
        │
        ▼
                              Container "dashboard"
                                 Layout::Horizontal
                              ┌───────────┴───────────┐
                              ▼                       ▼
                    Container "left"          Container "right"
                       Layout::Vertical           Layout::Vertical
                    ┌──────┴──────┐            ┌──────┴──────┐
                    ▼             ▼            ▼             ▼
              Text "label"  Text "value"  Text "label"  Text "value"
              "left"         "0"           "right"       "42"
```

Every box is an `Element`; the string under each container/leaf name is its
`Key`. `Component::render()` doesn't need to build this whole tree from
scratch by hand each time in application code — each nested `Component`
(here, each `Counter`) renders its own subtree, and the parent just
assembles the pieces:

```rust
impl Component for Dashboard {
    fn render(&self) -> Element {
        Element::container(
            "dashboard",
            Layout::Horizontal,
            vec![self.left.render(), self.right.render()],
        )
        .width(40)
        .height(2)
    }
}

impl Component for Counter {
    fn render(&self) -> Element {
        Element::container(
            self.label, // "left" or "right" — doubles as this node's Key
            Layout::Vertical,
            vec![
                Element::text("label", self.label).width(20),
                Element::text("value", self.count.to_string()).width(20),
            ],
        )
        .width(20)
        .height(2)
    }
}
```

## Reconciliation: only the diff repaints

`Tree` keeps the *previous* frame's `Element` tree around. Each call to
`frame()`/`present()` walks the *new* tree next to it, one container level
at a time, matching children **by `Key`, not by list position**:

```text
Component::render()          build a fresh Element tree from current state
        │
        ▼
Tree reconciles              walk next tree vs. previous tree, paired by Key
        │
        ├─ same content, same Rect  ──▶  skip — zero Buffer writes
        ├─ different content        ──▶  paint this node's cells
        ├─ new Key                  ──▶  paint (it's an add)
        └─ Key missing from next    ──▶  clear its old Rect (it's a removal)
        ▼
Buffer                        only the cells above were touched this frame
        │
        ▼
Tree::present(&mut renderer)  diff(on_screen, buffer) → Patch runs (v0.1 pipeline)
        │                     Renderer::draw(&patches) → one write_all()
        ▼
on_screen = buffer.clone()    commit as the baseline for the next frame
```

Matching by `Key` instead of position is what makes reordering cheap:
swapping two keyed children in the list is recognized as "these two moved,"
not "these two were removed and two new ones were added in their place."

The zero-write guarantee for unchanged subtrees is covered by a dedicated
test using a counting `Paint` sink —
[`tree::tests::unchanged_subtree_performs_zero_paint_calls`](https://github.com/MovingJu/Rsact/blob/main/crates/rsact-core/src/tree.rs).

## Driving a `Tree`

`Tree::present` is the one call you need per frame — it renders, reconciles,
diffs against what's actually on screen, and draws, all in one step:

```rust
use rsact_core::{component::Component, element::{Element, Layout}, renderer::Renderer, tree::Tree};

struct Counter {
    label: &'static str,
    count: u32,
}

impl Component for Counter {
    fn render(&self) -> Element {
        Element::container(
            self.label,
            Layout::Vertical,
            vec![
                Element::text("label", self.label).width(20),
                Element::text("value", self.count.to_string()).width(20),
            ],
        )
        .width(20)
        .height(2)
    }
}

fn main() -> std::io::Result<()> {
    let mut renderer = Renderer::new(std::io::stdout());
    let mut tree = Tree::new(Counter { label: "left", count: 0 }, 20, 2);

    for _ in 0..5 {
        tree.present(&mut renderer)?; // render → reconcile → diff → draw → sync
        tree.root_mut().count += 1;   // mutate state directly between frames
    }
    Ok(())
}
```

If you need the reconciled `Buffer` without drawing it yet (e.g. to run
your own diff/draw loop, or in a test), call `tree.frame()` instead —
`present()` is just `frame()` plus the diff/draw/sync step.

See [`rsact-demo/src/main.rs`](https://github.com/MovingJu/Rsact/blob/main/crates/rsact-demo/src/main.rs)
for the full `Dashboard`/`Counter` example, including two nested component
levels with independent local state.

## Current limitations (v0.2.0)

These are intentional non-goals for this milestone, not bugs — see
[#2](https://github.com/MovingJu/Rsact/issues/2):

- **No flexbox-style layout.** A `Container` only stacks children
  `Vertical`ly or `Horizontal`ly; each child declares its own fixed
  width/height. No wrapping, no flexible sizing, no overflow handling.
- **`Text` is single-line.** Content is clipped or space-padded to the
  element's declared width; multi-row text wrapping isn't implemented.
- **No built-in widget set yet.** `Text` and `Container` are the only
  element kinds — things like a progress bar or a bordered rectangle are
  built by composing those, or are candidates for a future crate/issue.
- **Single-threaded, synchronous.** Reconciliation and painting all happen
  on the caller's thread inside `present()`/`frame()`; v0.3.0
  ([#3](https://github.com/MovingJu/Rsact/issues/3)) is where opt-in
  multithreading is planned.
