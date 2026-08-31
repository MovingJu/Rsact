use std::io::{self, Read};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Enter,
    Esc,
    Backspace,
    Up,
    Down,
    Left,
    Right,
    Ctrl(char),
}

pub struct InputReader<R: Read> {
    inner: R,
}

impl<R: Read> InputReader<R> {
    pub fn new(inner: R) -> Self {
        Self { inner }
    }

    /// Reads and decodes the next key from raw bytes.
    ///
    /// - `ESC [ A/B/C/D` decodes as an arrow key; a bare `ESC` (nothing else
    ///   available right now) decodes as `Key::Esc`.
    /// - `\r` and `\n` both decode as `Key::Enter`.
    /// - `0x7f` (DEL) and `0x08` (BS) both decode as `Key::Backspace`.
    /// - Any other byte in `0x01..=0x1a` decodes as `Key::Ctrl('a'..='z')`.
    /// - Everything else is decoded as a `char`, reading UTF-8 continuation
    ///   bytes as needed (so multi-byte characters, e.g. Korean, come back
    ///   as a single `Key::Char`, not one `Key` per byte).
    ///
    /// Returns `Ok(None)` on EOF (no bytes left to read at all).
    pub fn read_key(&mut self) -> io::Result<Option<Key>> {
        let Some(ch) = self.read_char()? else {
            return Ok(None);
        };
        self.start_to_read(ch)
    }
    fn start_to_read(&mut self, ch: u8) -> io::Result<Option<Key>> {
        if ch.is_ascii() {
            self.ch_is_ascii(ch as char)
        } else {
            self.ch_is_not_ascii(ch as char)
        }
    }
    fn ch_is_ascii(&mut self, ch: char) -> io::Result<Option<Key>> {
        if ch != 0x1b as char {
            Ok(Some(simple_ascii_matching(ch)))
        } else {
            let Some(ch) = self.read_char()? else {
                return Ok(Some(Key::Esc));
            };
            match ch as char {
                '[' => self.ch_is_arrow(),
                _ => self.start_to_read(ch),
            }
        }
    }
    fn ch_is_arrow(&mut self) -> io::Result<Option<Key>> {
        let Some(ch) = self.read_char()? else {
            return Err(io::ErrorKind::InvalidInput.into());
        };
        self.match_arrow_keys(ch as char)
    }
    fn match_arrow_keys(&mut self, ch: char) -> io::Result<Option<Key>> {
        match ch {
            'A' => Ok(Some(Key::Up)),
            'B' => Ok(Some(Key::Down)),
            'C' => Ok(Some(Key::Right)),
            'D' => Ok(Some(Key::Left)),
            _ => self.start_to_read(ch as u8),
        }
    }
    fn read_char(&mut self) -> io::Result<Option<u8>> {
        let mut bytes = [0u8; 1];
        match self.inner.read(&mut bytes)? {
            0 => Ok(None),
            _ => Ok(Some(bytes[0])),
        }
    }
    fn ch_is_not_ascii(&mut self, ch: char) -> io::Result<Option<Key>> {
        let n_bytes = (ch as u8).leading_ones();
        let mut buffer: Vec<u8> = Vec::with_capacity(n_bytes as usize);
        buffer.push(ch as u8);
        for _ in 0..(n_bytes - 1) {
            let Some(ch) = self.read_char()? else {
                return Err(io::ErrorKind::InvalidInput.into());
            };
            buffer.push(ch);
        }
        let buffer = str::from_utf8(&buffer).map_err(|_| io::ErrorKind::InvalidInput)?;
        let ch = buffer
            .chars()
            .next()
            .expect("Can't convert valid utf to char");
        Ok(Some(Key::Char(ch)))
    }
}

/// ascii matching expect for `0x1b`
fn simple_ascii_matching(ch: char) -> Key {
    match ch as u8 {
        b'\r' => Key::Enter,
        b'\n' => Key::Enter,
        0x7f => Key::Backspace,
        0x08 => Key::Backspace,
        0x01..=0x1a => hex_to_key_ctrl(ch as u8),
        _ => Key::Char(ch),
    }
}

/// # Warning
/// No bound check.
///
/// ch should be within `0x01` to `0x1a`
fn hex_to_key_ctrl(num: u8) -> Key {
    Key::Ctrl((num + b'a' - 1) as char)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys_from(bytes: &[u8]) -> Vec<Key> {
        let mut reader = InputReader::new(bytes);
        let mut out = Vec::new();
        while let Some(key) = reader.read_key().unwrap() {
            out.push(key);
        }
        out
    }

    #[test]
    fn eof_on_empty_input_returns_none() {
        let mut reader = InputReader::new(&b""[..]);
        assert_eq!(reader.read_key().unwrap(), None);
    }

    #[test]
    fn plain_ascii_letter_is_decoded_as_char() {
        assert_eq!(keys_from(b"a"), vec![Key::Char('a')]);
    }

    #[test]
    fn multiple_ascii_chars_are_read_in_order() {
        assert_eq!(
            keys_from(b"abc"),
            vec![Key::Char('a'), Key::Char('b'), Key::Char('c')]
        );
    }

    #[test]
    fn carriage_return_and_line_feed_both_map_to_enter() {
        assert_eq!(keys_from(b"\r"), vec![Key::Enter]);
        assert_eq!(keys_from(b"\n"), vec![Key::Enter]);
    }

    #[test]
    fn del_and_backspace_bytes_both_map_to_backspace() {
        assert_eq!(keys_from(&[0x7f]), vec![Key::Backspace]);
        assert_eq!(keys_from(&[0x08]), vec![Key::Backspace]);
    }

    #[test]
    fn bare_esc_with_nothing_following_is_the_esc_key() {
        assert_eq!(keys_from(&[0x1b]), vec![Key::Esc]);
    }

    #[test]
    fn arrow_keys_are_decoded_from_three_byte_escape_sequences() {
        assert_eq!(keys_from(b"\x1b[A"), vec![Key::Up]);
        assert_eq!(keys_from(b"\x1b[B"), vec![Key::Down]);
        assert_eq!(keys_from(b"\x1b[C"), vec![Key::Right]);
        assert_eq!(keys_from(b"\x1b[D"), vec![Key::Left]);
    }

    #[test]
    fn arrow_sequence_followed_by_an_ascii_char_reads_both_correctly() {
        assert_eq!(keys_from(b"\x1b[Aq"), vec![Key::Up, Key::Char('q')]);
    }

    #[test]
    fn ctrl_letter_is_decoded_from_control_byte() {
        // Ctrl+A is 0x01 .. Ctrl+Z is 0x1a
        assert_eq!(keys_from(&[0x01]), vec![Key::Ctrl('a')]);
        assert_eq!(keys_from(&[0x18]), vec![Key::Ctrl('x')]);
    }

    #[test]
    fn ctrl_y_and_ctrl_z_are_decoded_from_control_bytes() {
        // regression test for the 0x01..=0x18 vs 0x01..=0x1a range bug
        assert_eq!(keys_from(&[0x19]), vec![Key::Ctrl('y')]);
        assert_eq!(keys_from(&[0x1a]), vec![Key::Ctrl('z')]);
    }

    #[test]
    fn two_byte_utf8_character_is_decoded_whole() {
        // 'é' = U+00E9 = 0xC3 0xA9 in UTF-8
        assert_eq!(keys_from(&[0xC3, 0xA9]), vec![Key::Char('é')]);
    }

    #[test]
    fn three_byte_utf8_korean_character_is_decoded_whole() {
        // '한' = U+D55C = 0xED 0x95 0x9C in UTF-8
        assert_eq!(keys_from(&[0xED, 0x95, 0x9C]), vec![Key::Char('한')]);
    }

    #[test]
    fn four_byte_utf8_emoji_is_decoded_whole() {
        // '😀' = U+1F600 = 0xF0 0x9F 0x98 0x80 in UTF-8
        assert_eq!(keys_from(&[0xF0, 0x9F, 0x98, 0x80]), vec![Key::Char('😀')]);
    }

    #[test]
    fn multi_byte_utf8_sequence_followed_by_an_ascii_char_reads_both_correctly() {
        // makes sure the decoder consumes exactly n_bytes and doesn't over/under-read
        let mut bytes = vec![0xF0, 0x9F, 0x98, 0x80];
        bytes.push(b'q');
        assert_eq!(keys_from(&bytes), vec![Key::Char('😀'), Key::Char('q')]);
    }

    #[test]
    fn korean_word_decodes_as_one_char_per_syllable_block() {
        // "안녕" (2 Korean syllable blocks, 3 bytes each = 6 bytes total)
        let bytes = "안녕".as_bytes();
        assert_eq!(keys_from(bytes), vec![Key::Char('안'), Key::Char('녕')]);
    }
}
