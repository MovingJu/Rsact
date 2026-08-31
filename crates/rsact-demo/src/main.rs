use rsact_core::{term::RawModeGuard, *};

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
    println!();
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
