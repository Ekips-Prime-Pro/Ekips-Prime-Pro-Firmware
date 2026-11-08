//! 5x5 light matrix: images, a small font and scrolling text.
//!
//! The font is this project's own 3x5 pixel font (digits, A–Z and a few
//! symbols). Lower-case letters are shown as upper case.

/// Matrix width and height.
pub const SIZE: usize = 5;

/// One frame: brightness of each pixel in percent (0..=100), row-major,
/// index 0 = top left in the current orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Image(pub [u8; SIZE * SIZE]);

impl Image {
    /// All pixels off.
    pub const BLANK: Self = Self([0; SIZE * SIZE]);

    /// Brightness at column `x`, row `y`.
    pub fn get(&self, x: usize, y: usize) -> u8 {
        self.0[y * SIZE + x]
    }

    /// Sets brightness at column `x`, row `y` (clamped to 100).
    pub fn set(&mut self, x: usize, y: usize, brightness: u8) {
        self.0[y * SIZE + x] = brightness.min(100);
    }

    /// Rotates the image by `orientation`, clockwise.
    pub fn rotated(&self, orientation: Orientation) -> Self {
        let mut out = Self::BLANK;
        for y in 0..SIZE {
            for x in 0..SIZE {
                let (nx, ny) = match orientation {
                    Orientation::Up => (x, y),
                    Orientation::Right => (SIZE - 1 - y, x),
                    Orientation::Down => (SIZE - 1 - x, SIZE - 1 - y),
                    Orientation::Left => (y, SIZE - 1 - x),
                };
                out.set(nx, ny, self.get(x, y));
            }
        }
        out
    }
}

/// Rotation of the displayed image, clockwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Orientation {
    /// No rotation.
    #[default]
    Up,
    /// 90° clockwise.
    Right,
    /// 180°.
    Down,
    /// 270° clockwise.
    Left,
}

/// Width of a glyph in columns.
pub const GLYPH_WIDTH: usize = 3;

/// Returns the 3x5 glyph of `c`: five rows, 3 bits each, bit 2 = left.
pub fn glyph(c: char) -> [u8; SIZE] {
    match c.to_ascii_uppercase() {
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b111, 0b001, 0b111, 0b100, 0b111],
        '3' => [0b111, 0b001, 0b111, 0b001, 0b111],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b111, 0b001, 0b111],
        '6' => [0b111, 0b100, 0b111, 0b101, 0b111],
        '7' => [0b111, 0b001, 0b010, 0b010, 0b010],
        '8' => [0b111, 0b101, 0b111, 0b101, 0b111],
        '9' => [0b111, 0b101, 0b111, 0b001, 0b111],
        'A' => [0b010, 0b101, 0b111, 0b101, 0b101],
        'B' => [0b110, 0b101, 0b110, 0b101, 0b110],
        'C' => [0b011, 0b100, 0b100, 0b100, 0b011],
        'D' => [0b110, 0b101, 0b101, 0b101, 0b110],
        'E' => [0b111, 0b100, 0b110, 0b100, 0b111],
        'F' => [0b111, 0b100, 0b110, 0b100, 0b100],
        'G' => [0b011, 0b100, 0b101, 0b101, 0b011],
        'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
        'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'J' => [0b001, 0b001, 0b001, 0b101, 0b010],
        'K' => [0b101, 0b101, 0b110, 0b101, 0b101],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
        'M' => [0b101, 0b111, 0b111, 0b101, 0b101],
        'N' => [0b110, 0b101, 0b101, 0b101, 0b101],
        'O' => [0b010, 0b101, 0b101, 0b101, 0b010],
        'P' => [0b110, 0b101, 0b110, 0b100, 0b100],
        'Q' => [0b010, 0b101, 0b101, 0b110, 0b011],
        'R' => [0b110, 0b101, 0b110, 0b101, 0b101],
        'S' => [0b011, 0b100, 0b010, 0b001, 0b110],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'U' => [0b101, 0b101, 0b101, 0b101, 0b111],
        'V' => [0b101, 0b101, 0b101, 0b101, 0b010],
        'W' => [0b101, 0b101, 0b111, 0b111, 0b101],
        'X' => [0b101, 0b101, 0b010, 0b101, 0b101],
        'Y' => [0b101, 0b101, 0b010, 0b010, 0b010],
        'Z' => [0b111, 0b001, 0b010, 0b100, 0b111],
        '-' => [0b000, 0b000, 0b111, 0b000, 0b000],
        '+' => [0b000, 0b010, 0b111, 0b010, 0b000],
        '.' => [0b000, 0b000, 0b000, 0b000, 0b010],
        ',' => [0b000, 0b000, 0b000, 0b010, 0b100],
        ':' => [0b000, 0b010, 0b000, 0b010, 0b000],
        '!' => [0b010, 0b010, 0b010, 0b000, 0b010],
        ' ' => [0; SIZE],
        _ => [0b110, 0b001, 0b010, 0b000, 0b010], // '?'
    }
}

/// Image with a single character centered (columns 1..=3).
pub fn char_image(c: char, brightness: u8) -> Image {
    let g = glyph(c);
    let mut img = Image::BLANK;
    for (y, row) in g.iter().enumerate() {
        for x in 0..GLYPH_WIDTH {
            if row & (0b100 >> x) != 0 {
                img.set(x + 1, y, brightness);
            }
        }
    }
    img
}

/// Produces the frames of a text that scrolls from right to left.
///
/// The text enters from the right edge and leaves at the left edge. Each
/// call to [`Iterator::next`] moves it one column.
#[derive(Debug, Clone)]
pub struct TextScroller<'a> {
    text: &'a str,
    brightness: u8,
    offset: usize,
    total_columns: usize,
}

impl<'a> TextScroller<'a> {
    /// Columns per character including the gap.
    const STRIDE: usize = GLYPH_WIDTH + 1;

    /// Creates a scroller for `text`.
    pub fn new(text: &'a str, brightness: u8) -> Self {
        let chars = text.chars().count();
        Self {
            text,
            brightness,
            offset: 0,
            // blank lead-in + text + blank lead-out
            total_columns: SIZE + chars * Self::STRIDE + SIZE,
        }
    }

    /// Number of frames until the text has left the display.
    pub fn frame_count(&self) -> usize {
        self.total_columns - SIZE + 1
    }

    /// Bit mask (bit 4 = top row) of virtual column `col`.
    fn column(&self, col: usize) -> u8 {
        if col < SIZE {
            return 0;
        }
        let col = col - SIZE;
        let (index, x) = (col / Self::STRIDE, col % Self::STRIDE);
        if x >= GLYPH_WIDTH {
            return 0;
        }
        let Some(c) = self.text.chars().nth(index) else {
            return 0;
        };
        let g = glyph(c);
        let mut bits = 0;
        for (y, row) in g.iter().enumerate() {
            if row & (0b100 >> x) != 0 {
                bits |= 1 << (SIZE - 1 - y);
            }
        }
        bits
    }
}

impl Iterator for TextScroller<'_> {
    type Item = Image;

    fn next(&mut self) -> Option<Image> {
        if self.offset + SIZE > self.total_columns {
            return None;
        }
        let mut img = Image::BLANK;
        for x in 0..SIZE {
            let bits = self.column(self.offset + x);
            for y in 0..SIZE {
                if bits & (1 << (SIZE - 1 - y)) != 0 {
                    img.set(x, y, self.brightness);
                }
            }
        }
        self.offset += 1;
        Some(img)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn char_image_is_centered() {
        let img = char_image('1', 100);
        // '1' row 0 = 010 -> pixel (2, 0)
        assert_eq!(img.get(2, 0), 100);
        assert_eq!(img.get(0, 0), 0);
        assert_eq!(img.get(4, 4), 0);
        assert_eq!(char_image('a', 50), char_image('A', 50));
    }

    #[test]
    fn rotation_round_trip() {
        let mut img = Image::BLANK;
        img.set(0, 0, 100);
        let r = img.rotated(Orientation::Right);
        assert_eq!(r.get(4, 0), 100);
        assert_eq!(img.rotated(Orientation::Down).get(4, 4), 100);
        assert_eq!(img.rotated(Orientation::Left).get(0, 4), 100);
        let back = r
            .rotated(Orientation::Right)
            .rotated(Orientation::Right)
            .rotated(Orientation::Right);
        assert_eq!(back, img);
    }

    #[test]
    fn scroller_starts_and_ends_blank() {
        let s = TextScroller::new("HI", 100);
        let n = s.frame_count();
        let frames: [Option<Image>; 2] = [s.clone().next(), s.clone().nth(n - 1)];
        assert_eq!(frames[0], Some(Image::BLANK));
        assert_eq!(frames[1], Some(Image::BLANK));
        assert_eq!(s.count(), n);
    }

    #[test]
    fn scroller_shows_glyph_when_aligned() {
        // After 4 steps the first glyph occupies columns 1..=3.
        let frame = TextScroller::new("7", 80).nth(4).unwrap();
        assert_eq!(frame, char_image('7', 80));
    }
}
