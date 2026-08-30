#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Color {
    #[default]
    Default,
    Indexed(u8),     // 256-color palette
    Rgb(u8, u8, u8), // truecolor
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Style {
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
    pub underline: bool,
    pub reverse: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub style: Style,
}

impl Default for Cell {
    /// A blank space in the default style — what every `Buffer` cell starts
    /// as. Not derived because `char::default()` is `'\0'`, not `' '`.
    fn default() -> Self {
        Cell {
            ch: ' ',
            style: Style::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_cell_is_a_blank_space_with_default_style() {
        let cell = Cell::default();
        assert_eq!(cell.ch, ' ');
        assert_eq!(cell.style, Style::default());
        assert_eq!(cell.style.fg, Color::Default);
        assert_eq!(cell.style.bg, Color::Default);
        assert!(!cell.style.bold);
        assert!(!cell.style.underline);
        assert!(!cell.style.reverse);
    }

    #[test]
    fn cells_with_same_char_but_different_style_are_not_equal() {
        let plain = Cell::default();
        let bold = Cell {
            style: Style {
                bold: true,
                ..Style::default()
            },
            ..plain
        };
        assert_ne!(plain, bold);
    }
}
