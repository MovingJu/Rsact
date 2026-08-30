use crate::buffer::Buffer;
use crate::cell::Cell;

/// A contiguous run of changed cells within a single row. Never spans more
/// than one row — the renderer needs at most one cursor move per `Patch`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Patch {
    pub row: u16,
    pub col: u16,
    pub cells: Vec<Cell>, // contiguous run of changed cells starting at (row, col)
}

/// Compares two same-sized buffers and returns the minimal list of contiguous
/// changed runs, scanning row by row and merging adjacent differing columns
/// into a single Patch so the renderer needs one cursor move per run instead
/// of one per cell.
///
/// # Panics
/// Panics if `prev` and `next` have different dimensions.
pub fn diff(prev: &Buffer, next: &Buffer) -> Vec<Patch> {
    assert_eq!(
        (prev.width(), prev.height()),
        (next.width(), next.height()),
        "diff: buffer dimensions must match"
    );
    let mut diffs: Vec<Patch> = Vec::new();
    for row in 0..prev.height() {
        let prev_row = prev.row(row);
        let next_row = next.row(row);
        let mut run: Option<(u16, Vec<Cell>)> = None;

        for (col, (&p, &n)) in prev_row.iter().zip(next_row).enumerate() {
            let col = col as u16;
            if p != n {
                match &mut run {
                    Some((_, acc)) => acc.push(n),
                    None => run = Some((col, vec![n])),
                }
            } else if let Some((started, acc)) = run.take() {
                diffs.push(Patch::new(row, started, acc));
            }
        }
        if let Some((started, acc)) = run.take() {
            diffs.push(Patch::new(row, started, acc));
        }
    }
    diffs
}

impl Patch {
    fn new(row: u16, col: u16, cells: Vec<Cell>) -> Self {
        Self { row, col, cells }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::{Color, Style};

    fn styled(ch: char, fg: Color) -> Cell {
        Cell {
            ch,
            style: Style {
                fg,
                ..Style::default()
            },
        }
    }

    /// Rebuilds what `next` should look like by applying `patches` on top of
    /// `prev`. Used to assert patches are *sufficient* to reconstruct the new
    /// frame exactly, not just individually plausible.
    fn apply(prev: &Buffer, patches: &[Patch]) -> Buffer {
        let mut out = prev.clone();
        for patch in patches {
            for (i, cell) in patch.cells.iter().enumerate() {
                out.set(patch.row, patch.col + i as u16, *cell);
            }
        }
        out
    }

    /// Checks the diff invariants that aren't captured by any single example:
    /// every patch fits inside the buffer, and no two patches (in the same or
    /// different rows) claim overlapping columns.
    fn assert_patches_well_formed(width: u16, patches: &[Patch]) {
        let mut covered: Vec<(u16, u16, u16)> = Vec::new(); // (row, start_col, end_col_exclusive)
        for patch in patches {
            let end = patch.col + patch.cells.len() as u16;
            assert!(
                end <= width,
                "patch at row {} starting col {} with {} cells overflows width {width}",
                patch.row,
                patch.col,
                patch.cells.len()
            );
            for &(row, start, prev_end) in &covered {
                if row == patch.row {
                    let overlaps = patch.col < prev_end && start < end;
                    assert!(
                        !overlaps,
                        "patches on row {row} overlap: [{start}, {prev_end}) and [{}, {end})",
                        patch.col
                    );
                }
            }
            covered.push((patch.row, patch.col, end));
        }
    }

    /// Tiny deterministic xorshift32 PRNG so the property-style test below
    /// stays reproducible without pulling in a `rand` dependency.
    struct XorShift32(u32);
    impl XorShift32 {
        fn next_u32(&mut self) -> u32 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            self.0 = x;
            x
        }
        fn next_range(&mut self, bound: u16) -> u16 {
            (self.next_u32() % bound as u32) as u16
        }
    }

    #[test]
    fn identical_buffers_produce_no_patches() {
        let buf = Buffer::new(10, 4);
        assert_eq!(diff(&buf, &buf), Vec::new());
    }

    #[test]
    fn single_changed_cell_produces_one_single_cell_patch() {
        let prev = Buffer::new(10, 4);
        let mut next = prev.clone();
        next.set(2, 5, styled('x', Color::Rgb(255, 0, 0)));

        let patches = diff(&prev, &next);

        assert_eq!(
            patches,
            vec![Patch {
                row: 2,
                col: 5,
                cells: vec![styled('x', Color::Rgb(255, 0, 0))],
            }]
        );
    }

    #[test]
    fn fully_changed_row_produces_one_patch_spanning_the_row() {
        let prev = Buffer::new(6, 3);
        let mut next = prev.clone();
        for col in 0..6 {
            next.set(1, col, styled('#', Color::Default));
        }

        let patches = diff(&prev, &next);

        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].row, 1);
        assert_eq!(patches[0].col, 0);
        assert_eq!(patches[0].cells.len(), 6);
        assert_eq!(apply(&prev, &patches), next);
    }

    #[test]
    fn two_separate_changed_runs_in_the_same_row_stay_separate_patches() {
        let prev = Buffer::new(10, 1);
        let mut next = prev.clone();
        next.set(0, 0, styled('a', Color::Default));
        next.set(0, 5, styled('b', Color::Default));
        next.set(0, 6, styled('c', Color::Default));
        // columns 1..=4 and 7..=9 stay unchanged, so this must NOT collapse
        // into a single patch spanning column 0..=6.

        let patches = diff(&prev, &next);

        assert_eq!(patches.len(), 2);
        assert_patches_well_formed(10, &patches);
        assert_eq!(apply(&prev, &patches), next);
    }

    #[test]
    fn changes_in_different_rows_produce_one_patch_per_row() {
        let prev = Buffer::new(5, 5);
        let mut next = prev.clone();
        next.set(0, 2, styled('a', Color::Default));
        next.set(3, 2, styled('b', Color::Default));

        let patches = diff(&prev, &next);

        assert_eq!(patches.len(), 2);
        let rows: Vec<u16> = patches.iter().map(|p| p.row).collect();
        assert!(rows.contains(&0));
        assert!(rows.contains(&3));
    }

    #[test]
    fn style_only_change_with_same_character_is_still_detected() {
        let prev = Buffer::new(4, 1);
        let mut next = prev.clone();
        // same char as the default blank cell, only the style differs
        next.set(0, 1, styled(' ', Color::Rgb(0, 255, 0)));

        let patches = diff(&prev, &next);

        assert_eq!(patches.len(), 1);
        assert_eq!(apply(&prev, &patches), next);
    }

    #[test]
    fn changes_at_first_and_last_column_are_detected() {
        let prev = Buffer::new(8, 1);
        let mut next = prev.clone();
        next.set(0, 0, styled('L', Color::Default));
        next.set(0, 7, styled('R', Color::Default));

        let patches = diff(&prev, &next);

        assert_eq!(patches.len(), 2);
        assert_eq!(apply(&prev, &patches), next);
    }

    #[test]
    fn applying_patches_reconstructs_next_exactly_for_randomly_mutated_buffers() {
        const WIDTH: u16 = 20;
        const HEIGHT: u16 = 8;
        const MUTATIONS: usize = 40;

        for seed in [1u32, 42, 1_000_003, 0xdead_beef] {
            let prev = Buffer::new(WIDTH, HEIGHT);
            let mut next = prev.clone();
            let mut rng = XorShift32(seed);

            for _ in 0..MUTATIONS {
                let row = rng.next_range(HEIGHT);
                let col = rng.next_range(WIDTH);
                let ch = char::from_u32('a' as u32 + rng.next_range(26) as u32).unwrap();
                next.set(
                    row,
                    col,
                    styled(ch, Color::Indexed((rng.next_u32() % 256) as u8)),
                );
            }

            let patches = diff(&prev, &next);
            assert_patches_well_formed(WIDTH, &patches);
            assert_eq!(
                apply(&prev, &patches),
                next,
                "seed {seed} failed to reconstruct"
            );
        }
    }

    #[test]
    #[should_panic(expected = "dimensions")]
    fn diff_panics_on_mismatched_dimensions() {
        let a = Buffer::new(5, 5);
        let b = Buffer::new(6, 5);
        diff(&a, &b);
    }
}
