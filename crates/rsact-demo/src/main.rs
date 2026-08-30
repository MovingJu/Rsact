use rsact_core::*;

fn main() {
    // Setup terminal and DOMS.
    let (term_row, term_col) = term::terminal_size().expect("Can't size the terminal.");
    let mut real_dom = buffer::Buffer::new(term_col, term_row);
    let mut virtual_dom = buffer::Buffer::new(term_col, term_row);

    // Draw as you want.
    rectangle(&mut virtual_dom, term_row, term_col);

    // Reflect your virtual DOM to terminal.
    let differences = diff::diff(&real_dom, &virtual_dom);
    let mut rend = renderer::Renderer::new(std::io::stdout());
    rend.draw(&differences).expect("Failed stdout");
    real_dom = virtual_dom.clone();

    std::thread::sleep(core::time::Duration::from_millis(500));

    for i in 1..8 {
        rectangle(&mut virtual_dom, 5 * i, 10 * i);

        let differences = diff::diff(&real_dom, &virtual_dom);
        let mut rend = renderer::Renderer::new(std::io::stdout());
        rend.draw(&differences).expect("Failed stdout");

        std::thread::sleep(core::time::Duration::from_millis(500));
    }

    virtual_dom.set(
        term_row - 1,
        term_col - 1,
        cell::Cell {
            ch: '#',
            style: cell::Style::default(),
        },
    );
    let differences = diff::diff(&real_dom, &virtual_dom);
    let mut rend = renderer::Renderer::new(std::io::stdout());
    rend.draw(&differences).expect("Failed stdout");

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
