//! A tiny built-in 3x5 font for the frontend's own status text (not the
//! game's font).

/// Each glyph is five rows of three bits (bit 2 = left column).
fn glyph(c: char) -> [u8; 5] {
    match c.to_ascii_uppercase() {
        '0' => [7, 5, 5, 5, 7],
        '1' => [2, 6, 2, 2, 7],
        '2' => [7, 1, 7, 4, 7],
        '3' => [7, 1, 7, 1, 7],
        '4' => [5, 5, 7, 1, 1],
        '5' => [7, 4, 7, 1, 7],
        '6' => [7, 4, 7, 5, 7],
        '7' => [7, 1, 2, 2, 2],
        '8' => [7, 5, 7, 5, 7],
        '9' => [7, 5, 7, 1, 7],
        'A' => [2, 5, 7, 5, 5],
        'B' => [6, 5, 6, 5, 6],
        'C' => [3, 4, 4, 4, 3],
        'D' => [6, 5, 5, 5, 6],
        'E' => [7, 4, 6, 4, 7],
        'F' => [7, 4, 6, 4, 4],
        'G' => [3, 4, 5, 5, 3],
        'H' => [5, 5, 7, 5, 5],
        'I' => [7, 2, 2, 2, 7],
        'J' => [1, 1, 1, 5, 2],
        'K' => [5, 5, 6, 5, 5],
        'L' => [4, 4, 4, 4, 7],
        'M' => [5, 7, 7, 5, 5],
        'N' => [6, 5, 5, 5, 5],
        'O' => [2, 5, 5, 5, 2],
        'P' => [6, 5, 6, 4, 4],
        'Q' => [2, 5, 5, 6, 3],
        'R' => [6, 5, 6, 5, 5],
        'S' => [3, 4, 2, 1, 6],
        'T' => [7, 2, 2, 2, 2],
        'U' => [5, 5, 5, 5, 7],
        'V' => [5, 5, 5, 5, 2],
        'W' => [5, 5, 7, 7, 5],
        'X' => [5, 5, 2, 5, 5],
        'Y' => [5, 5, 2, 2, 2],
        'Z' => [7, 1, 2, 4, 7],
        '.' => [0, 0, 0, 0, 2],
        ',' => [0, 0, 0, 2, 4],
        ':' => [0, 2, 0, 2, 0],
        ';' => [0, 2, 0, 2, 4],
        '-' => [0, 0, 7, 0, 0],
        '_' => [0, 0, 0, 0, 7],
        '/' => [1, 1, 2, 4, 4],
        '\\' => [4, 4, 2, 1, 1],
        '(' => [1, 2, 2, 2, 1],
        ')' => [4, 2, 2, 2, 4],
        '[' => [3, 2, 2, 2, 3],
        ']' => [6, 2, 2, 2, 6],
        '{' => [3, 2, 6, 2, 3],
        '}' => [6, 2, 3, 2, 6],
        '#' => [5, 7, 5, 7, 5],
        '+' => [0, 2, 7, 2, 0],
        '=' => [0, 7, 0, 7, 0],
        '?' => [7, 1, 3, 0, 2],
        '!' => [2, 2, 2, 0, 2],
        '%' => [5, 1, 2, 4, 5],
        '>' => [4, 2, 1, 2, 4],
        '<' => [1, 2, 4, 2, 1],
        '\'' => [2, 2, 0, 0, 0],
        '"' => [5, 5, 0, 0, 0],
        '*' => [0, 5, 2, 5, 0],
        '|' => [2, 2, 2, 2, 2],
        '`' => [4, 2, 0, 0, 0],
        '&' => [2, 5, 2, 5, 3],
        '@' => [7, 5, 7, 4, 7],
        '$' => [3, 6, 2, 3, 6],
        '^' => [2, 5, 0, 0, 0],
        '~' => [0, 3, 6, 0, 0],
        _ => [0, 0, 0, 0, 0],
    }
}

/// Characters per line at `width` pixels.
pub fn columns(width: usize) -> usize {
    width / 4
}

/// Draw `text` at (x, y) into a BGR555 frame of the given width, over a
/// dark box. Lines wrap at the frame edge.
pub fn draw(frame: &mut [u16], width: usize, x: usize, y: usize, text: &str, color: u16) -> usize {
    let height = frame.len() / width;
    let cols = columns(width.saturating_sub(x)).max(1);
    let mut lines = 0;
    for line in text.lines() {
        let chars: Vec<char> = line.chars().collect();
        for chunk in chars.chunks(cols).chain(if chars.is_empty() { Some(&[][..]) } else { None }) {
            let ly = y + lines * 6;
            lines += 1;
            if ly + 6 > height {
                return lines;
            }
            for dy in 0..6 {
                for dx in 0..chunk.len() * 4 + 1 {
                    if x + dx < width {
                        let p = &mut frame[(ly + dy) * width + x + dx];
                        *p = (*p >> 2) & 0x1CE7;
                    }
                }
            }
            for (i, &c) in chunk.iter().enumerate() {
                let g = glyph(c);
                for (row, bits) in g.iter().enumerate() {
                    for col in 0..3 {
                        if bits & (4 >> col) != 0 {
                            let px = x + 1 + i * 4 + col;
                            if px < width {
                                frame[(ly + 1 + row) * width + px] = color;
                            }
                        }
                    }
                }
            }
        }
    }
    lines
}

#[cfg(test)]
mod tests {
    #[test]
    fn draws_and_wraps() {
        let mut f = vec![0x7FFFu16; 40 * 20];
        let lines = super::draw(&mut f, 40, 0, 0, "HELLO WORLD 12345", 0x001F);
        assert_eq!(lines, 2);
        // 'H' left column, first row.
        assert_eq!(f[40 + 1], 0x001F);
    }
}
