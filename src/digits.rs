/// Returns a 5-line ASCII art representation of a character.
/// Each line is exactly 5 characters wide.
pub fn digit_lines(ch: char) -> [&'static str; 5] {
    match ch {
        '0' => [
            "█████",
            "█   █",
            "█   █",
            "█   █",
            "█████",
        ],
        '1' => [
            "   █ ",
            "  ██ ",
            "   █ ",
            "   █ ",
            "   █ ",
        ],
        '2' => [
            "█████",
            "    █",
            "█████",
            "█    ",
            "█████",
        ],
        '3' => [
            "█████",
            "    █",
            "█████",
            "    █",
            "█████",
        ],
        '4' => [
            "█   █",
            "█   █",
            "█████",
            "    █",
            "    █",
        ],
        '5' => [
            "█████",
            "█    ",
            "█████",
            "    █",
            "█████",
        ],
        '6' => [
            "█████",
            "█    ",
            "█████",
            "█   █",
            "█████",
        ],
        '7' => [
            "█████",
            "    █",
            "   █ ",
            "  █  ",
            "  █  ",
        ],
        '8' => [
            "█████",
            "█   █",
            "█████",
            "█   █",
            "█████",
        ],
        '9' => [
            "█████",
            "█   █",
            "█████",
            "    █",
            "█████",
        ],
        ':' => [
            "  ",
            " █",
            "  ",
            " █",
            "  ",
        ],
        ' ' => [
            "  ",
            "  ",
            "  ",
            "  ",
            "  ",
        ],
        _ => [
            "     ",
            "     ",
            "     ",
            "     ",
            "     ",
        ],
    }
}

/// Compose a string of characters into 5 lines of ASCII art.
/// A single space gap is inserted between each character for readability.
pub fn render_big_text(text: &str) -> Vec<String> {
    let chars: Vec<[&str; 5]> = text.chars().map(digit_lines).collect();
    (0..5)
        .map(|row| {
            chars
                .iter()
                .map(|c| c[row])
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digit_lines_returns_5_lines() {
        for ch in "0123456789: ".chars() {
            let lines = digit_lines(ch);
            assert_eq!(lines.len(), 5, "char '{}' should have 5 lines", ch);
        }
    }

    #[test]
    fn digit_lines_consistent_widths() {
        for ch in "0123456789".chars() {
            let lines = digit_lines(ch);
            let width = lines[0].chars().count();
            for (i, line) in lines.iter().enumerate() {
                assert_eq!(
                    line.chars().count(),
                    width,
                    "char '{}' line {} has inconsistent width",
                    ch,
                    i
                );
            }
        }
    }

    #[test]
    fn render_big_text_produces_5_lines() {
        let output = render_big_text("12:34");
        assert_eq!(output.len(), 5);
    }

    #[test]
    fn render_big_text_single_char() {
        let output = render_big_text("0");
        assert_eq!(output.len(), 5);
        assert_eq!(output[0], digit_lines('0')[0]);
    }

    #[test]
    fn unknown_char_returns_blank() {
        let lines = digit_lines('?');
        for line in &lines {
            assert!(line.trim().is_empty(), "unknown char should be blank");
        }
    }
}
