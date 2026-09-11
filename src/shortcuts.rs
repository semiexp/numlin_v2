use crate::grid::Grid;
use crate::EdgeState;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Shortcut {
    Join((usize, usize)),
    Conflict,
}

// A new shortcut must touch a newly labeled or newly unused vertex. Inspect only
// straight runs from changed vertices. Keep unfinished scans pending, including
// after a conflict, so undo cannot lose a required check.
pub(crate) struct Shortcuts {
    height: usize,
    width: usize,
    dirty: Vec<bool>,
    pending: Vec<(usize, usize)>,
}

impl Shortcuts {
    pub(crate) fn new(height: usize, width: usize) -> Self {
        Self {
            height,
            width,
            dirty: vec![true; height * width],
            pending: (0..height).flat_map(|y| (0..width).map(move |x| (y, x))).collect(),
        }
    }

    pub(crate) fn vertex_changed(&mut self, y: usize, x: usize) {
        let p = y * self.width + x;
        if !self.dirty[p] {
            self.dirty[p] = true;
            self.pending.push((y, x));
        }
    }

    fn visible_clue(
        clues: &Grid<i32>, usable: &Grid<u8>, p: i32, step: i32, length: usize,
    ) -> Option<(i32, usize)> {
        for distance in 1..=length {
            let q = p + step * distance as i32;
            if clues[q] >= 0 {
                return Some((clues[q], distance));
            }
            // An unknown or occupied cell can still carry another line.
            if usable[q] != 0 {
                break;
            }
        }
        None
    }

    pub(crate) fn next(
        &mut self,
        clues: &Grid<i32>,
        usable: &Grid<u8>,
        edges: &Grid<EdgeState>,
    ) -> Option<Shortcut> {
        while let Some(&(y, x)) = self.pending.last() {
            let p = (y * self.width + x) as i32;
            let clue = clues[p];
            let directions = [
                (-1, x), (1, self.width - 1 - x),
                (-(self.width as i32), y), (self.width as i32, self.height - 1 - y),
            ];
            if clue >= 0 {
                for (direction, &(step, length)) in directions.iter().enumerate() {
                    if let Some((other_clue, distance)) = Self::visible_clue(clues, usable, p, step, length) {
                        if clue != other_clue {
                            continue;
                        }
                        if distance > 1 {
                            return Some(Shortcut::Conflict);
                        }
                        let edge = match direction {
                            0 => (2 * y, 2 * x - 1),
                            1 => (2 * y, 2 * x + 1),
                            2 => (2 * y - 1, 2 * x),
                            3 => (2 * y + 1, 2 * x),
                            _ => unreachable!(),
                        };
                        match edges[edge] {
                            EdgeState::Undecided => return Some(Shortcut::Join(edge)),
                            EdgeState::NoLine => return Some(Shortcut::Conflict),
                            EdgeState::Line => {}
                        }
                    }
                }
            } else if usable[p] == 0 {
                // The newly unused cell can complete a gap between two older
                // labeled vertices, neither of which changed in this decision.
                for axis in [0, 2] {
                    let (step, length) = directions[axis];
                    if let Some((left, _)) = Self::visible_clue(clues, usable, p, step, length) {
                        let (step, length) = directions[axis + 1];
                        if let Some((right, _)) = Self::visible_clue(clues, usable, p, step, length) {
                            if left == right {
                                return Some(Shortcut::Conflict);
                            }
                        }
                    }
                }
            }
            self.pending.pop();
            self.dirty[p as usize] = false;
        }
        #[cfg(any(test, feature = "verify-cut-capacity"))]
        self.verify_complete(clues, usable, edges);
        None
    }

    // Independently enumerate every pair, without relying on the dirty queue or
    // the nearest-occupied-cell scan. Only run after propagation reaches a fixpoint.
    #[cfg(any(test, feature = "verify-cut-capacity"))]
    fn verify_complete(&self, clues: &Grid<i32>, usable: &Grid<u8>, edges: &Grid<EdgeState>) {
        for y in 0..self.height {
            for x in 0..self.width {
                let clue = clues[(y, x)];
                if clue < 0 {
                    continue;
                }
                for xx in x + 1..self.width {
                    if clues[(y, xx)] == clue
                        && (x + 1..xx).all(|i| clues[(y, i)] < 0 && usable[(y, i)] == 0)
                    {
                        assert!(xx == x + 1 && edges[(2 * y, 2 * x + 1)] == EdgeState::Line,
                            "missed horizontal shortcut at ({y}, {x})..({y}, {xx})");
                    }
                }
                for yy in y + 1..self.height {
                    if clues[(yy, x)] == clue
                        && (y + 1..yy).all(|i| clues[(i, x)] < 0 && usable[(i, x)] == 0)
                    {
                        assert!(yy == y + 1 && edges[(2 * y + 1, 2 * x)] == EdgeState::Line,
                            "missed vertical shortcut at ({y}, {x})..({yy}, {x})");
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjacent_equal_clues_force_the_join_and_recheck_after_undo() {
        for vertical in [false, true] {
            let mut shortcuts = Shortcuts::new(4, 4);
            let mut clues = Grid::new(4, 4, -1);
            let usable = Grid::new(4, 4, 4);
            let mut edges = Grid::new(7, 7, EdgeState::Undecided);
            let q = if vertical { (2, 1) } else { (1, 2) };
            let edge = if vertical { (3, 2) } else { (2, 3) };
            clues[(1, 1)] = 17;
            clues[q] = 17;
            assert_eq!(shortcuts.next(&clues, &usable, &edges), Some(Shortcut::Join(edge)));
            // A rejected decision must not consume the pending scan.
            edges[edge] = EdgeState::NoLine;
            assert_eq!(shortcuts.next(&clues, &usable, &edges), Some(Shortcut::Conflict));
            edges[edge] = EdgeState::Undecided;
            assert_eq!(shortcuts.next(&clues, &usable, &edges), Some(Shortcut::Join(edge)));
            edges[edge] = EdgeState::Line;
            assert_eq!(shortcuts.next(&clues, &usable, &edges), None);
            edges[edge] = EdgeState::Undecided;
            shortcuts.vertex_changed(1, 1);
            assert_eq!(shortcuts.next(&clues, &usable, &edges), Some(Shortcut::Join(edge)));
        }
    }

    #[test]
    fn arbitrary_gaps_require_every_middle_cell_to_be_unused() {
        for distance in [2, 3, 7, 20] {
            for vertical in [false, true] {
                let (height, width) = if vertical { (distance + 3, 3) } else { (3, distance + 3) };
                let pos = |i| if vertical { (i, 1) } else { (1, i) };
                let mut shortcuts = Shortcuts::new(height, width);
                let mut clues = Grid::new(height, width, -1);
                let mut usable = Grid::new(height, width, 4);
                let edges = Grid::new(2 * height - 1, 2 * width - 1, EdgeState::NoLine);
                clues[pos(1)] = 17;
                clues[pos(distance + 1)] = 17;
                for i in 2..distance + 1 {
                    usable[pos(i)] = 0;
                }
                assert_eq!(shortcuts.next(&clues, &usable, &edges), Some(Shortcut::Conflict));
                for i in 2..distance + 1 {
                    // Both an unresolved cell and another line protect the gap.
                    for (label, count) in [(-1, 1), (-1, 2), (23, 2)] {
                        clues[pos(i)] = label;
                        usable[pos(i)] = count;
                        let (y, x) = pos(i);
                        shortcuts.vertex_changed(y, x);
                        assert_eq!(shortcuts.next(&clues, &usable, &edges), None);
                        clues[pos(i)] = -1;
                        usable[pos(i)] = 0;
                        shortcuts.vertex_changed(y, x);
                        assert_eq!(shortcuts.next(&clues, &usable, &edges), Some(Shortcut::Conflict));
                    }
                }
                clues[pos(distance + 1)] = 23;
                assert_eq!(shortcuts.next(&clues, &usable, &edges), None);
            }
        }
    }
}
