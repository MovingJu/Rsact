use crate::cell::{Color, Style};
use crate::diff::Patch;
use std::io::{self, Write};

pub struct Renderer<W: Write> {
    out: W,
    last_style: Option<Style>, // avoid re-emitting SGR codes when style hasn't changed
}

impl<W: Write> Renderer<W> {
    pub fn new(out: W) -> Self {
        Self {
            out,
            last_style: None,
        }
    }

    /// Encodes all patches into one byte buffer and performs at most one
    /// `write_all` call (none at all if `patches` is empty). For each patch:
    /// one cursor-move escape `\x1b[{row+1};{col+1}H` (1-indexed — the CUP
    /// escape is 1-indexed even though `Patch`/`Buffer` coordinates are
    /// 0-indexed), then for each cell in order: an SGR sequence
    /// `\x1b[0;{codes}m` *only* when the cell's style differs from the last
    /// one written (bold=1, underline=4, reverse=7, fg as `38;5;n` /
    /// `38;2;r;g;b`, bg as `48;5;n` / `48;2;r;g;b`, joined by `;`; a fully
    /// default style is just `\x1b[0m`), then the character's UTF-8 bytes.
    /// Cells within one patch are contiguous columns, so the cursor is only
    /// repositioned once per patch — printing each character already
    /// advances it to the next column.
    pub fn draw(&mut self, patches: &[Patch]) -> io::Result<()> {
        let mut buf: Vec<u8> = Vec::new();
        for patch in patches {
            move_cursor(&mut buf, patch.row, patch.col);
            for cell in &patch.cells {
                let does_style_change = self.last_style != Some(cell.style);
                if does_style_change {
                    self.last_style = Some(cell.style);
                    let mut style_slice = style_to_slice(&cell.style);
                    buf.append(&mut style_slice);
                }
                let mut tmp = [0u8; 4];
                buf.extend_from_slice(cell.ch.encode_utf8(&mut tmp).as_bytes());
            }
        }
        self.out.write_all(&buf)?;
        self.out.flush()?;
        Ok(())
    }
    pub(crate) fn down_cursor(&mut self) -> io::Result<()> {
        let mut buf: Vec<u8> = style_to_slice(&Style::default());
        buf.push(b'\n');
        self.out.write_all(&buf)?;
        Ok(())
    }
    pub(crate) fn move_cursor(&mut self, row: u16, col: u16) -> io::Result<()> {
        let mut buf: Vec<u8> = Vec::new();
        move_cursor(&mut buf, row - 1, col - 1);
        self.out.write_all(&buf)?;
        Ok(())
    }
}
static ESC: &str = "\x1b";
static FAILED_TO_WRITE_BUFFER: &str = "Failed to write buffer vector.";
fn move_cursor(buf: &mut Vec<u8>, row: u16, col: u16) {
    write!(buf, "{ESC}[{};{}H", row + 1, col + 1).expect(FAILED_TO_WRITE_BUFFER);
}
fn style_to_slice(style: &Style) -> Vec<u8> {
    let mut slice = Vec::new();
    let mut font_style = String::new();
    if style.bold {
        font_style.push_str(";1");
    }
    if style.underline {
        font_style.push_str(";4");
    }
    if style.reverse {
        font_style.push_str(";7");
    }
    write!(slice, "{ESC}[{}{font_style}", 0).expect(FAILED_TO_WRITE_BUFFER);
    let slice = color_font(slice, &style.fg, true);
    let mut slice = color_font(slice, &style.bg, false);
    write!(slice, "m").expect(FAILED_TO_WRITE_BUFFER);
    slice
}
fn color_font(slice: Vec<u8>, color: &Color, is_fg: bool) -> Vec<u8> {
    match color {
        Color::Default => slice,
        Color::Indexed(idx) => indexed_color(slice, *idx, is_fg),
        Color::Rgb(r, g, b) => true_color(slice, (*r, *g, *b), is_fg),
    }
}
fn indexed_color(mut slice: Vec<u8>, idx: u8, is_fg: bool) -> Vec<u8> {
    let prefix = if is_fg { "38" } else { "48" };
    write!(slice, ";{prefix};5;{idx}").expect(FAILED_TO_WRITE_BUFFER);
    slice
}
fn true_color(mut slice: Vec<u8>, rgb: (u8, u8, u8), is_fg: bool) -> Vec<u8> {
    let prefix = if is_fg { "38" } else { "48" };
    write!(slice, ";{prefix};2;{};{};{}", rgb.0, rgb.1, rgb.2).expect(FAILED_TO_WRITE_BUFFER);
    slice
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::{Cell, Color};

    #[derive(Default)]
    struct CountingSink {
        buf: Vec<u8>,
        write_calls: usize,
    }

    impl Write for CountingSink {
        fn write(&mut self, data: &[u8]) -> io::Result<usize> {
            self.write_calls += 1;
            self.buf.extend_from_slice(data);
            Ok(data.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn output(sink: &CountingSink) -> &str {
        std::str::from_utf8(&sink.buf).expect("renderer output must be valid UTF-8")
    }

    /// Extracts the contents (without the leading `\x1b[` or trailing `m`) of
    /// every SGR escape sequence in `out`, in order. Distinguishes them from
    /// cursor-move (`...H`) escapes, which share the same `\x1b[` prefix.
    fn sgr_sequences(out: &str) -> Vec<&str> {
        let mut seqs = Vec::new();
        let mut rest = out;
        while let Some(start) = rest.find("\x1b[") {
            let after = &rest[start + 2..];
            let Some(end) = after.find(|c: char| c == 'm' || c == 'H') else {
                break;
            };
            if after.as_bytes()[end] == b'm' {
                seqs.push(&after[..end]);
            }
            rest = &after[end + 1..];
        }
        seqs
    }

    fn plain(ch: char) -> Cell {
        Cell {
            ch,
            style: Style::default(),
        }
    }

    #[test]
    fn moves_cursor_to_one_indexed_position_and_writes_the_character() {
        let mut sink = CountingSink::default();
        let mut renderer = Renderer::new(&mut sink);
        let patch = Patch {
            row: 2,
            col: 5,
            cells: vec![plain('x')],
        };

        renderer.draw(&[patch]).unwrap();

        let out = output(&sink);
        assert!(
            out.contains("\x1b[3;6H"),
            "expected a cursor move to (row 3, col 6), 1-indexed; got {out:?}"
        );
        assert!(
            out.ends_with('x'),
            "expected the character last; got {out:?}"
        );
    }

    #[test]
    fn multibyte_utf8_character_is_written_whole_not_truncated() {
        let mut sink = CountingSink::default();
        let mut renderer = Renderer::new(&mut sink);
        let patch = Patch {
            row: 0,
            col: 0,
            cells: vec![plain('한')],
        };

        renderer.draw(&[patch]).unwrap();

        let out = output(&sink);
        assert!(
            out.ends_with('한'),
            "expected the multi-byte character written whole, not truncated to its low byte; got {out:?}"
        );
    }

    #[test]
    fn draw_with_no_patches_performs_no_writes_at_all() {
        let mut sink = CountingSink::default();
        let mut renderer = Renderer::new(&mut sink);

        renderer.draw(&[]).unwrap();

        assert_eq!(sink.write_calls, 0);
        assert!(sink.buf.is_empty());
    }

    #[test]
    fn draw_performs_exactly_one_write_call_per_invocation() {
        let mut sink = CountingSink::default();
        let mut renderer = Renderer::new(&mut sink);
        let patches = vec![
            Patch {
                row: 0,
                col: 0,
                cells: vec![plain('a')],
            },
            Patch {
                row: 1,
                col: 0,
                cells: vec![plain('b')],
            },
        ];

        renderer.draw(&patches).unwrap();

        assert_eq!(
            sink.write_calls, 1,
            "expected one write() for the whole frame regardless of patch count"
        );
    }

    #[test]
    fn cursor_is_moved_once_per_patch_not_once_per_cell() {
        let mut sink = CountingSink::default();
        let mut renderer = Renderer::new(&mut sink);
        let patch = Patch {
            row: 4,
            col: 2,
            cells: vec![plain('a'), plain('b'), plain('c')],
        };

        renderer.draw(&[patch]).unwrap();

        let out = output(&sink);
        assert_eq!(
            out.matches('H').count(),
            1,
            "3 contiguous cells in one patch should need only one cursor move; got {out:?}"
        );
    }

    #[test]
    fn multiple_patches_each_get_their_own_cursor_move() {
        let mut sink = CountingSink::default();
        let mut renderer = Renderer::new(&mut sink);
        let patches = vec![
            Patch {
                row: 0,
                col: 0,
                cells: vec![plain('a')],
            },
            Patch {
                row: 3,
                col: 7,
                cells: vec![plain('b')],
            },
        ];

        renderer.draw(&patches).unwrap();

        let out = output(&sink);
        assert!(out.contains("\x1b[1;1H"), "got {out:?}");
        assert!(out.contains("\x1b[4;8H"), "got {out:?}");
    }

    #[test]
    fn default_style_emits_a_bare_reset_with_no_extra_codes() {
        let mut sink = CountingSink::default();
        let mut renderer = Renderer::new(&mut sink);

        renderer
            .draw(&[Patch {
                row: 0,
                col: 0,
                cells: vec![plain('x')],
            }])
            .unwrap();

        let seqs = sgr_sequences(output(&sink));
        assert_eq!(
            seqs,
            vec!["0"],
            "default style should just reset with nothing else; got {seqs:?}"
        );
    }

    #[test]
    fn same_style_across_two_cells_in_one_patch_emits_only_one_sgr_sequence() {
        let mut sink = CountingSink::default();
        let mut renderer = Renderer::new(&mut sink);
        let style = Style {
            bold: true,
            ..Style::default()
        };
        let patch = Patch {
            row: 0,
            col: 0,
            cells: vec![Cell { ch: 'a', style }, Cell { ch: 'b', style }],
        };

        renderer.draw(&[patch]).unwrap();

        let seqs = sgr_sequences(output(&sink));
        assert_eq!(
            seqs.len(),
            1,
            "same style twice in a row must not re-emit SGR; got {seqs:?}"
        );
    }

    #[test]
    fn style_change_between_cells_emits_a_new_sgr_sequence() {
        let mut sink = CountingSink::default();
        let mut renderer = Renderer::new(&mut sink);
        let patch = Patch {
            row: 0,
            col: 0,
            cells: vec![
                Cell {
                    ch: 'a',
                    style: Style {
                        bold: true,
                        ..Style::default()
                    },
                },
                Cell {
                    ch: 'b',
                    style: Style {
                        underline: true,
                        ..Style::default()
                    },
                },
            ],
        };

        renderer.draw(&[patch]).unwrap();

        let seqs = sgr_sequences(output(&sink));
        assert_eq!(
            seqs.len(),
            2,
            "each distinct style needs its own SGR sequence; got {seqs:?}"
        );
    }

    #[test]
    fn style_cache_persists_across_separate_draw_calls() {
        let mut sink = CountingSink::default();
        let mut renderer = Renderer::new(&mut sink);
        let style = Style {
            fg: Color::Rgb(1, 2, 3),
            ..Style::default()
        };

        renderer
            .draw(&[Patch {
                row: 0,
                col: 0,
                cells: vec![Cell { ch: 'a', style }],
            }])
            .unwrap();
        renderer
            .draw(&[Patch {
                row: 1,
                col: 0,
                cells: vec![Cell { ch: 'b', style }],
            }])
            .unwrap();

        let seqs = sgr_sequences(output(&sink));
        assert_eq!(
            seqs.len(),
            1,
            "second draw() reused the same style as the first, so no new SGR sequence should appear; got {seqs:?}"
        );
    }

    #[test]
    fn rgb_foreground_uses_38_2_sgr_code() {
        let mut sink = CountingSink::default();
        let mut renderer = Renderer::new(&mut sink);
        let style = Style {
            fg: Color::Rgb(10, 20, 30),
            ..Style::default()
        };

        renderer
            .draw(&[Patch {
                row: 0,
                col: 0,
                cells: vec![Cell { ch: 'x', style }],
            }])
            .unwrap();

        let seqs = sgr_sequences(output(&sink));
        assert!(
            seqs.iter().any(|s| s.contains("38;2;10;20;30")),
            "got {seqs:?}"
        );
    }

    #[test]
    fn indexed_background_uses_48_5_sgr_code() {
        let mut sink = CountingSink::default();
        let mut renderer = Renderer::new(&mut sink);
        let style = Style {
            bg: Color::Indexed(200),
            ..Style::default()
        };

        renderer
            .draw(&[Patch {
                row: 0,
                col: 0,
                cells: vec![Cell { ch: 'x', style }],
            }])
            .unwrap();

        let seqs = sgr_sequences(output(&sink));
        assert!(seqs.iter().any(|s| s.contains("48;5;200")), "got {seqs:?}");
    }

    #[test]
    fn bold_underline_reverse_each_add_their_own_sgr_code() {
        let mut sink = CountingSink::default();
        let mut renderer = Renderer::new(&mut sink);
        let style = Style {
            bold: true,
            underline: true,
            reverse: true,
            ..Style::default()
        };

        renderer
            .draw(&[Patch {
                row: 0,
                col: 0,
                cells: vec![Cell { ch: 'x', style }],
            }])
            .unwrap();

        let codes: Vec<&str> = sgr_sequences(output(&sink))
            .into_iter()
            .flat_map(|s| s.split(';'))
            .collect();
        assert!(codes.contains(&"1"), "missing bold code; got {codes:?}");
        assert!(
            codes.contains(&"4"),
            "missing underline code; got {codes:?}"
        );
        assert!(codes.contains(&"7"), "missing reverse code; got {codes:?}");
    }
}
