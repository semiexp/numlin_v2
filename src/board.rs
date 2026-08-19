use crate::grid::Grid;
use crate::problem::Problem;
use std::fmt::Debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeState {
    Undecided,
    Line,
    NoLine,
}

// Information about the history of decisions made on the board, used for backtracking.
enum History {
    // Checkpoint represents a point in the history to which we can backtrack.
    Checkpoint,

    // If an update occurs to the edge state, the original state would always be EdgeState::Undecided,
    // so we don't need to store it in the history.
    EdgeState((usize, usize)),

    // AnotherEnd(p, v) means that the vertex at position p was previously connected to vertex v, and is now being updated.
    AnotherEnd(i32, i32),

    // Inconsistent means that the board was previously inconsistent, and is now being updated to be consistent.
    Inconsistent,
}

pub struct Board {
    height: usize,
    width: usize,
    problem: Problem,

    has_clue: Grid<bool>, // height * width grid indicating whether each cell has a clue

    edge_state: Grid<EdgeState>, // (2 * height - 1) * (2 * width - 1) grid of edge states

    // For each vertex, the end of the line that is connected to it.
    // x >= 0 means that the vertex is an endpoint and is connected to the vertex with index x.
    // x <= -2 means that the vertex is an endpoint and is connected to "clue" of value -x-2.
    // x == -1 means that the vertex is not an endpoint.
    another_end: Grid<i32>,

    inconsistent: bool,

    history: Vec<History>, // History of decisions made on the board, used for backtracking.
}

impl Board {
    pub fn new(problem: Problem) -> Self {
        let height = problem.height();
        let width = problem.width();
        let mut has_clue = Grid::new(height, width, false);
        let mut another_end = Grid::new(height, width, -1);

        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;

                if let Some(value) = problem.get(y, x) {
                    has_clue[(y, x)] = true;
                    another_end[(y, x)] = -value - 2;
                } else {
                    another_end[(y, x)] = idx as i32;
                }
            }
        }

        Self {
            height,
            width,
            problem,
            has_clue,
            edge_state: Grid::new(2 * height - 1, 2 * width - 1, EdgeState::Undecided),
            another_end,
            inconsistent: false,
            history: Vec::new(),
        }
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn inconsistent(&self) -> bool {
        self.inconsistent
    }

    pub fn has_clue(&self, y: usize, x: usize) -> bool {
        self.has_clue[(y, x)]
    }

    pub fn add_checkpoint(&mut self) {
        self.history.push(History::Checkpoint);
    }

    pub fn get_edge(&self, y: usize, x: usize) -> EdgeState {
        debug_assert!(y % 2 != x % 2);
        self.edge_state[(y, x)]
    }

    pub fn undo(&mut self) {
        while let Some(history) = self.history.pop() {
            match history {
                History::Checkpoint => break,
                History::EdgeState((y, x)) => {
                    self.edge_state[(y, x)] = EdgeState::Undecided;
                }
                History::AnotherEnd(p, v) => {
                    self.another_end[p] = v;
                }
                History::Inconsistent => {
                    self.inconsistent = false;
                }
            }
        }
    }

    fn update_another_end(&mut self, p: i32, new_value: i32) {
        let old_value = self.another_end[p];
        if old_value != new_value {
            self.history.push(History::AnotherEnd(p, old_value));
            self.another_end[p] = new_value;
        }
    }

    fn set_inconsistent(&mut self) {
        if !self.inconsistent {
            self.history.push(History::Inconsistent);
            self.inconsistent = true;
        }
    }

    pub fn decide_edge(&mut self, y: usize, x: usize, state: EdgeState) -> bool {
        debug_assert!(y % 2 != x % 2);

        if self.edge_state[(y, x)] != EdgeState::Undecided {
            if self.edge_state[(y, x)] != state {
                self.set_inconsistent();
            }
            return self.inconsistent();
        }

        self.edge_state[(y, x)] = state;
        self.history.push(History::EdgeState((y, x)));

        // Update the another_end grid based on the edge decision
        if state == EdgeState::Line {
            let y1 = y / 2;
            let x1 = x / 2;
            let idx1 = self.another_end.flat_index((y1, x1)) as i32;
            let y2 = (y + 1) / 2;
            let x2 = (x + 1) / 2;
            let idx2 = self.another_end.flat_index((y2, x2)) as i32;

            let ae1 = self.another_end[idx1];
            let ae2 = self.another_end[idx2];

            if ae1 == -1 || ae2 == -1 {
                // If either vertex is not an endpoint, we cannot connect them
                self.set_inconsistent();
                return self.inconsistent();
            }

            if ae1 < 0 && ae2 < 0 && ae1 != ae2 {
                // If both vertices are endpoints connected to different clues, we cannot connect them
                self.set_inconsistent();
                return self.inconsistent();
            }

            if ae1 == idx2 {
                // If both vertices are already connected to each other, we cannot connect them again (this would create a loop)
                self.set_inconsistent();
                return self.inconsistent();
            }

            self.update_another_end(idx1, -1);
            self.update_another_end(idx2, -1);
            if ae1 >= 0 {
                self.update_another_end(ae1, ae2);
            }
            if ae2 >= 0 {
                self.update_another_end(ae2, ae1);
            }
        }

        self.inconsistent()
    }
}

impl Debug for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for y in 0..(self.height * 2 - 1) {
            for x in 0..(self.width * 2 - 1) {
                if y % 2 == 0 && x % 2 == 0 {
                    // Vertex
                    let v = self.another_end[(y / 2, x / 2)];
                    if v < 0 {
                        write!(f, "C")?; // Clue
                    } else if v == -1 {
                        write!(f, ".")?; // Not an endpoint
                    } else {
                        write!(f, "E")?; // Endpoint
                    }
                } else if y % 2 == 0 && x % 2 == 1 {
                    // Edge
                    let edge_state = self.edge_state[(y, x)];
                    match edge_state {
                        EdgeState::Undecided => write!(f, ".")?,
                        EdgeState::Line => write!(f, "-")?,
                        EdgeState::NoLine => write!(f, "x")?,
                    }
                } else if y % 2 == 1 && x % 2 == 0 {
                    // Edge
                    let edge_state = self.edge_state[(y, x)];
                    match edge_state {
                        EdgeState::Undecided => write!(f, ".")?,
                        EdgeState::Line => write!(f, "|")?,
                        EdgeState::NoLine => write!(f, "x")?,
                    }
                } else {
                    // Empty space
                    write!(f, " ")?;
                }
            }
            writeln!(f)?;
        }

        Ok(())
    }
}
