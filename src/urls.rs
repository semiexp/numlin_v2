use crate::Problem;
use crate::grid::Grid;

pub fn decode_url(url: &str) -> Problem {
    // The host is intentionally ignored.  Accept both a complete URL and the
    // path starting at `p?numlin/`.
    let encoded = if let Some(encoded) = url.strip_prefix("p?numlin/") {
        encoded
    } else {
        let marker = "/p?numlin/";
        let start = url
            .find(marker)
            .unwrap_or_else(|| panic!("invalid numlin URL: expected p?numlin"));
        &url[start + marker.len()..]
    };

    let mut parts = encoded.split('/');
    let width = parse_dimension(parts.next(), "width");
    let height = parse_dimension(parts.next(), "height");
    let hints = parts
        .next()
        .unwrap_or_else(|| panic!("invalid numlin URL: missing hint data"));
    assert!(
        parts.next().is_none(),
        "invalid numlin URL: too many path components"
    );

    let mut problem = Grid::new(height, width, None);
    let mut cell = 0;
    let mut chars = hints.chars();

    while let Some(ch) = chars.next() {
        if ('g'..='z').contains(&ch) {
            // g..z encode 1..20 consecutive empty cells.
            let empty_cells = (ch as usize) - ('g' as usize) + 1;
            cell += empty_cells;
        } else if let Some(value) = ('1'..='9').chain('a'..='f').position(|digit| digit == ch) {
            // 1..9 and a..f encode clue values 1..15.
            set_clue(&mut problem, cell, value as i32 + 1);
            cell += 1;
        } else if ch == '-' || ch == '+' {
            let digits = if ch == '-' { 2 } else { 3 };
            let mut value = 0u32;
            for _ in 0..digits {
                let digit = chars
                    .next()
                    .and_then(|digit| digit.to_digit(16))
                    .unwrap_or_else(|| panic!("invalid numlin URL: malformed clue value"));
                value = value * 16 + digit;
            }
            assert!(
                value >= 16,
                "invalid numlin URL: extended clue is too small"
            );
            set_clue(&mut problem, cell, value as i32);
            cell += 1;
        } else {
            panic!("invalid numlin URL: unexpected hint character {ch:?}");
        }

        assert!(
            cell <= width * height,
            "invalid numlin URL: hint data exceeds board size"
        );
    }

    assert_eq!(
        cell,
        width * height,
        "invalid numlin URL: hint data does not fill the board"
    );

    problem
}

fn parse_dimension(value: Option<&str>, name: &str) -> usize {
    let value = value.unwrap_or_else(|| panic!("invalid numlin URL: missing {name}"));
    let dimension = value
        .parse::<usize>()
        .unwrap_or_else(|_| panic!("invalid numlin URL: invalid {name}"));
    assert!(dimension > 0, "invalid numlin URL: {name} must be positive");
    dimension
}

fn set_clue(problem: &mut Problem, cell: usize, value: i32) {
    let width = problem.width();
    assert!(
        cell < width * problem.height(),
        "invalid numlin URL: clue is outside the board"
    );
    problem[(cell / width, cell % width)] = Some(value);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_decode() {
        let url = "https://puzz.link/p?numlin/7/6/523i1j5l4i2h3m4g1l";
        let decoded = decode_url(url);

        let expected = [
            [5, 2, 3, 0, 0, 0, 1],
            [0, 0, 0, 0, 5, 0, 0],
            [0, 0, 0, 0, 4, 0, 0],
            [0, 2, 0, 0, 3, 0, 0],
            [0, 0, 0, 0, 0, 4, 0],
            [1, 0, 0, 0, 0, 0, 0],
        ];

        assert_eq!(decoded.height(), expected.len());
        assert_eq!(decoded.width(), expected[0].len());
        for y in 0..decoded.height() {
            for x in 0..decoded.width() {
                assert_eq!(
                    decoded[(y, x)],
                    if expected[y][x] == 0 {
                        None
                    } else {
                        Some(expected[y][x])
                    },
                    "mismatch at ({y}, {x})"
                );
            }
        }
    }
}
