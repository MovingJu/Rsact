// buffer.rs — this IS the "terminal DOM": a flat, diffable snapshot of the screen
use crate::cell::Cell;

#[derive(Debug, PartialEq, Eq)]
pub struct Buffer {
    width: u16,
    height: u16,
    cells: Vec<Cell>, // row-major, len == width * height
}

impl Clone for Buffer {
    fn clone(&self) -> Self {
        Self {
            width: self.width,
            height: self.height,
            cells: self.cells.clone(),
        }
    }
    fn clone_from(&mut self, source: &Self) {
        self.width = source.width;
        self.height = source.height;
        self.cells.clone_from(&source.cells);
    }
}

impl Buffer {
    pub(crate) fn row(&self, row: u16) -> &[Cell] {
        let start = (row * self.width) as usize;
        &self.cells[start..(start + self.width as usize)]
    }
}

impl Buffer {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            cells: vec![Cell::default(); (width * height) as usize],
        }
    }

    pub fn width(&self) -> u16 {
        self.width
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    /// # Panics
    /// Panics with a message containing "out of bounds" if `row >= height()`
    /// or `col >= width()`.
    pub fn get(&self, row: u16, col: u16) -> Cell {
        self.bound_check(row, col);
        self.cells[(row * self.width + col) as usize]
    }

    /// # Panics
    /// Panics with a message containing "out of bounds" if `row >= height()`
    /// or `col >= width()`.
    pub fn set(&mut self, row: u16, col: u16, cell: Cell) {
        self.bound_check(row, col);
        self.cells[(row * self.width + col) as usize] = cell;
    }

    /// Resizes to `width x height` and clears every cell back to
    /// `Cell::default()` — v0.1 does not preserve old content, even when the
    /// new size matches the old one.
    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        self.cells = vec![Cell::default(); (width * height) as usize]
    }

    #[inline]
    fn bound_check(&self, row: u16, col: u16) {
        static OOB: &str = "out of bounds";
        assert!(row < self.height, "{}", OOB);
        assert!(col < self.width, "{}", OOB);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::{Color, Style};

    #[test]
    fn new_buffer_is_blank_and_reports_correct_dimensions() {
        let buf = Buffer::new(5, 3);
        assert_eq!(buf.width(), 5);
        assert_eq!(buf.height(), 3);
        for row in 0..3 {
            for col in 0..5 {
                assert_eq!(buf.get(row, col), Cell::default());
            }
        }
    }

    #[test]
    fn set_then_get_roundtrips_at_every_corner() {
        let mut buf = Buffer::new(4, 3);
        let corners = [(0, 0), (0, 3), (2, 0), (2, 3)];
        for (i, &(row, col)) in corners.iter().enumerate() {
            let cell = Cell {
                ch: char::from_u32('a' as u32 + i as u32).unwrap(),
                style: Style {
                    fg: Color::Indexed(i as u8),
                    ..Style::default()
                },
            };
            buf.set(row, col, cell);
            assert_eq!(buf.get(row, col), cell);
        }
        // untouched cell in the middle stays default
        assert_eq!(buf.get(1, 1), Cell::default());
    }

    #[test]
    #[should_panic(expected = "out of bounds")]
    fn get_out_of_bounds_row_panics() {
        let buf = Buffer::new(4, 3);
        buf.get(3, 0);
    }

    #[test]
    #[should_panic(expected = "out of bounds")]
    fn get_out_of_bounds_col_panics() {
        let buf = Buffer::new(4, 3);
        buf.get(0, 4);
    }

    #[test]
    #[should_panic(expected = "out of bounds")]
    fn set_out_of_bounds_panics() {
        let mut buf = Buffer::new(4, 3);
        buf.set(0, 4, Cell::default());
    }

    #[test]
    fn resize_updates_dimensions() {
        let mut buf = Buffer::new(4, 3);
        buf.resize(10, 2);
        assert_eq!(buf.width(), 10);
        assert_eq!(buf.height(), 2);
        for row in 0..2 {
            for col in 0..10 {
                assert_eq!(buf.get(row, col), Cell::default());
            }
        }
    }

    #[test]
    fn resize_clears_previous_content_even_at_same_size() {
        let mut buf = Buffer::new(4, 3);
        buf.set(
            1,
            1,
            Cell {
                ch: 'x',
                ..Cell::default()
            },
        );
        buf.resize(4, 3);
        assert_eq!(buf.get(1, 1), Cell::default());
    }
}
