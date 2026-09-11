use rsact_core::{
    component::Component,
    element::{Element, Layout},
    term::RawModeGuard,
    tree::Tree,
    *,
};

fn main() {
    // Setup terminal
    let terminal = term::terminal_size().expect("Can't size the terminal.");
    let _rmg = RawModeGuard::enable_safe_exit(terminal, std::io::stdout())
        .expect("Failed to enter raw mode.");

    // Setup DOMs and renderer.
    let mut real_dom = buffer::Buffer::new(terminal.col, terminal.row);
    let mut virtual_dom = buffer::Buffer::new(terminal.col, terminal.row);
    let mut rend = renderer::Renderer::new(std::io::stdout());

    // Draw as you want.
    rectangle(&mut virtual_dom, terminal.row, terminal.col);

    // Reflect your virtual DOM to terminal.
    let differences = diff::diff(&real_dom, &virtual_dom);
    rend.draw(&differences).expect("Failed stdout");
    real_dom = virtual_dom.clone();

    std::thread::sleep(core::time::Duration::from_millis(500));

    for i in 1..8 {
        // Clamp to the actual terminal size: `Buffer` stays sized to
        // `terminal.row`×`terminal.col` for the whole demo, but this loop's
        // requested rectangle grows past that (up to 35×70) regardless of
        // how big the terminal actually is — without clamping, `set` panics
        // on any terminal smaller than that.
        let row = (5 * i).min(terminal.row);
        let col = (10 * i).min(terminal.col);
        rectangle(&mut virtual_dom, row, col);

        let differences = diff::diff(&real_dom, &virtual_dom);
        rend.draw(&differences).expect("Failed stdout");

        std::thread::sleep(core::time::Duration::from_millis(500));
    }

    std::thread::sleep(core::time::Duration::from_millis(3000));

    component_tree_demo(&mut rend);

    std::thread::sleep(core::time::Duration::from_millis(3000));
    println!();
}

/// A leaf component owning its own local state: a label and the count it's
/// currently showing.
struct Counter {
    label: &'static str,
    count: u32,
}

impl Component for Counter {
    fn render(&self) -> Element {
        view!(
            container(self.label, Layout::Vertical, width = 20, height = 2) {
                text("label", self.label, width = 20),
                text("value", self.count.to_string(), width = 20),
            }
        )
    }
}

/// Two `Counter`s side by side — the second, nested level of the component
/// tree. Each `Counter` advances its own `count` independently.
struct Dashboard {
    left: Counter,
    right: Counter,
}

impl Component for Dashboard {
    fn render(&self) -> Element {
        view!(
            container("dashboard", Layout::Horizontal, width = 40, height = 2) {
                self.left.render(),
                self.right.render(),
            }
        )
    }
}

/// Drives a small `Tree` for a few frames, incrementing only the left
/// counter each frame to show that reconciling the tree repaints just the
/// subtree that actually changed — the right counter's cells are never
/// touched again after the first frame. `Tree::present` owns the
/// diff/draw/swap loop, so the caller only has to call it once per frame.
fn component_tree_demo<W: std::io::Write>(rend: &mut renderer::Renderer<W>) {
    let mut tree = Tree::new(
        Dashboard {
            left: Counter {
                label: "left",
                count: 0,
            },
            right: Counter {
                label: "right",
                count: 42,
            },
        },
        40,
        2,
    );

    for _ in 0..5 {
        tree.present(rend).expect("Failed stdout");
        std::thread::sleep(core::time::Duration::from_millis(500));
        tree.root_mut().left.count += 1;
    }
}

fn rectangle(buffer: &mut buffer::Buffer, row: u16, col: u16) {
    for idx in 0..col {
        buffer.set(
            0,
            idx,
            cell::Cell {
                ch: '#',
                style: cell::Style::default(),
            },
        );
        buffer.set(
            row - 1,
            idx,
            cell::Cell {
                ch: '#',
                style: cell::Style::default(),
            },
        );
    }
    for idx in 0..row {
        buffer.set(
            idx,
            0,
            cell::Cell {
                ch: '#',
                style: cell::Style::default(),
            },
        );
        buffer.set(
            idx,
            col - 1,
            cell::Cell {
                ch: '#',
                style: cell::Style::default(),
            },
        );
    }
}
