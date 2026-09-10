use rsact_core::{
    cell::Style,
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
        rectangle(&mut virtual_dom, 5 * i, 10 * i);

        let differences = diff::diff(&real_dom, &virtual_dom);
        rend.draw(&differences).expect("Failed stdout");

        std::thread::sleep(core::time::Duration::from_millis(500));
    }

    std::thread::sleep(core::time::Duration::from_millis(3000));

    component_tree_demo(&mut real_dom, &mut rend);

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
        Element::container(
            self.label,
            Layout::Vertical,
            20,
            2,
            vec![
                Element::text("label", 20, self.label, Style::default()),
                Element::text("value", 20, self.count.to_string(), Style::default()),
            ],
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
        Element::container(
            "dashboard",
            Layout::Horizontal,
            40,
            2,
            vec![self.left.render(), self.right.render()],
        )
    }
}

/// Drives a small `Tree` for a few frames, incrementing only the left
/// counter each frame to show that reconciling the tree repaints just the
/// subtree that actually changed — the right counter's cells are never
/// touched again after the first frame.
fn component_tree_demo<W: std::io::Write>(
    real_dom: &mut buffer::Buffer,
    rend: &mut renderer::Renderer<W>,
) {
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
        let virtual_dom = tree.frame();
        let differences = diff::diff(real_dom, virtual_dom);
        rend.draw(&differences).expect("Failed stdout");
        real_dom.clone_from(virtual_dom);

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
