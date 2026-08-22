use crate::grid::Grid;
use crate::{Answer, EdgeState, Problem};
use std::fmt::Debug;

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

    // disallow_dl and disallow_dr indicate whether having 2 outgoing edges in down and left/right directions
    // is disallowed for each vertex (due to L-shape canonization). These are used to detect incosistency quickly.
    disallow_dl: Grid<bool>,
    disallow_dr: Grid<bool>,

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

                if let Some(value) = problem[(y, x)] {
                    has_clue[(y, x)] = true;
                    another_end[(y, x)] = -value - 2;
                } else {
                    another_end[(y, x)] = idx as i32;
                }
            }
        }

        let mut disallow_dl = Grid::new(height, width, false);
        let mut disallow_dr = Grid::new(height, width, false);
        if crate::OPTIMIZATION_L_SHAPE_CANONIZATION {
            for y in (0..height).rev() {
                for x in 0..width {
                    if has_clue[(y, x)] {
                        disallow_dl[(y, x)] = false;
                        disallow_dr[(y, x)] = false;
                        continue;
                    }

                    if y == height - 1 || x == 0 {
                        disallow_dl[(y, x)] = true;
                    } else {
                        disallow_dl[(y, x)] = disallow_dl[(y + 1, x - 1)];
                    }
                    if y == height - 1 || x == width - 1 {
                        disallow_dr[(y, x)] = true;
                    } else {
                        disallow_dr[(y, x)] = disallow_dr[(y + 1, x + 1)];
                    }
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
            disallow_dl,
            disallow_dr,
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

    pub fn to_answer(&self) -> Answer {
        Answer::new(self.edge_state.clone())
    }

    pub fn decide_edge(&mut self, y: usize, x: usize, state: EdgeState) -> bool {
        debug_assert!(y % 2 != x % 2);
        debug_assert!(y < self.height * 2 - 1 && x < self.width * 2 - 1);

        if self.edge_state[(y, x)] != EdgeState::Undecided {
            if self.edge_state[(y, x)] != state {
                self.set_inconsistent();
            }
            return self.inconsistent();
        }

        self.edge_state[(y, x)] = state;
        self.history.push(History::EdgeState((y, x)));

        let mut another_end1 = None;
        let mut another_end2 = None;

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
                another_end1 = Some(ae1);
            }
            if ae2 >= 0 {
                self.update_another_end(ae2, ae1);
                another_end2 = Some(ae2);
            }
        }

        if crate::OPTIMIZATION_DISALLOW_TRIVIAL_DETOUR {
            if state == EdgeState::Line {
                if y % 2 == 0 {
                    if y > 0 && self.get_edge(y - 1, x - 1) == EdgeState::Line {
                        self.decide_edge(y - 1, x + 1, EdgeState::NoLine);
                        self.decide_edge(y - 2, x, EdgeState::NoLine);
                    }
                    if y > 0 && self.get_edge(y - 1, x + 1) == EdgeState::Line {
                        self.decide_edge(y - 1, x - 1, EdgeState::NoLine);
                        self.decide_edge(y - 2, x, EdgeState::NoLine);
                    }
                    if y > 0 && self.get_edge(y - 2, x) == EdgeState::Line {
                        self.decide_edge(y - 1, x - 1, EdgeState::NoLine);
                        self.decide_edge(y - 1, x + 1, EdgeState::NoLine);
                    }
                    if y < self.height * 2 - 2 && self.get_edge(y + 1, x - 1) == EdgeState::Line {
                        self.decide_edge(y + 1, x + 1, EdgeState::NoLine);
                        self.decide_edge(y + 2, x, EdgeState::NoLine);
                    }
                    if y < self.height * 2 - 2 && self.get_edge(y + 1, x + 1) == EdgeState::Line {
                        self.decide_edge(y + 1, x - 1, EdgeState::NoLine);
                        self.decide_edge(y + 2, x, EdgeState::NoLine);
                    }
                    if y < self.height * 2 - 2 && self.get_edge(y + 2, x) == EdgeState::Line {
                        self.decide_edge(y + 1, x - 1, EdgeState::NoLine);
                        self.decide_edge(y + 1, x + 1, EdgeState::NoLine);
                    }
                } else {
                    if x > 0 && self.get_edge(y - 1, x - 1) == EdgeState::Line {
                        self.decide_edge(y + 1, x - 1, EdgeState::NoLine);
                        self.decide_edge(y, x - 2, EdgeState::NoLine);
                    }
                    if x > 0 && self.get_edge(y + 1, x - 1) == EdgeState::Line {
                        self.decide_edge(y - 1, x - 1, EdgeState::NoLine);
                        self.decide_edge(y, x - 2, EdgeState::NoLine);
                    }
                    if x > 0 && self.get_edge(y, x - 2) == EdgeState::Line {
                        self.decide_edge(y - 1, x - 1, EdgeState::NoLine);
                        self.decide_edge(y + 1, x - 1, EdgeState::NoLine);
                    }
                    if x < self.width * 2 - 2 && self.get_edge(y - 1, x + 1) == EdgeState::Line {
                        self.decide_edge(y + 1, x + 1, EdgeState::NoLine);
                        self.decide_edge(y, x + 2, EdgeState::NoLine);
                    }
                    if x < self.width * 2 - 2 && self.get_edge(y + 1, x + 1) == EdgeState::Line {
                        self.decide_edge(y - 1, x + 1, EdgeState::NoLine);
                        self.decide_edge(y, x + 2, EdgeState::NoLine);
                    }
                    if x < self.width * 2 - 2 && self.get_edge(y, x + 2) == EdgeState::Line {
                        self.decide_edge(y - 1, x + 1, EdgeState::NoLine);
                        self.decide_edge(y + 1, x + 1, EdgeState::NoLine);
                    }
                }
            }
        }

        if crate::OPTIMIZATION_L_SHAPE_CANONIZATION {
            if state == EdgeState::Line {
                if y % 2 == 0 {
                    if y < self.height * 2 - 2
                        && !self.has_clue(y / 2 + 1, x / 2 + 1)
                        && self.get_edge(y + 1, x - 1) == EdgeState::Line
                    {
                        if y + 3 < self.height * 2 - 1 && x + 2 < self.width * 2 - 1 {
                            self.decide_edge(y + 2, x + 2, EdgeState::Line);
                            self.decide_edge(y + 3, x + 1, EdgeState::Line);
                        } else {
                            self.set_inconsistent();
                            return self.inconsistent();
                        }
                    }
                    if y < self.height * 2 - 2
                        && !self.has_clue(y / 2 + 1, x / 2)
                        && self.get_edge(y + 1, x + 1) == EdgeState::Line
                    {
                        if y + 3 < self.height * 2 - 1 && x >= 2 {
                            self.decide_edge(y + 2, x - 2, EdgeState::Line);
                            self.decide_edge(y + 3, x - 1, EdgeState::Line);
                        } else {
                            self.set_inconsistent();
                            return self.inconsistent();
                        }
                    }
                } else {
                    if x > 0
                        && !self.has_clue(y / 2 + 1, x / 2 - 1)
                        && self.get_edge(y - 1, x - 1) == EdgeState::Line
                    {
                        if y + 2 < self.height * 2 - 1 && x >= 3 {
                            self.decide_edge(y + 1, x - 3, EdgeState::Line);
                            self.decide_edge(y + 2, x - 2, EdgeState::Line);
                        } else {
                            self.set_inconsistent();
                            return self.inconsistent();
                        }
                    }
                    if x < self.width * 2 - 2
                        && !self.has_clue(y / 2 + 1, x / 2 + 1)
                        && self.get_edge(y - 1, x + 1) == EdgeState::Line
                    {
                        if y + 2 < self.height * 2 - 1 && x + 3 < self.width * 2 - 1 {
                            self.decide_edge(y + 1, x + 3, EdgeState::Line);
                            self.decide_edge(y + 2, x + 2, EdgeState::Line);
                        } else {
                            self.set_inconsistent();
                            return self.inconsistent();
                        }
                    }
                }

                // disallow_dl and disallow_dr
                if y % 2 == 0 {
                    if self.disallow_dr[(y / 2, x / 2)]
                        && y + 1 < self.height * 2 - 1
                        && x + 1 < self.width * 2 - 1
                    {
                        if self.decide_edge(y + 1, x - 1, EdgeState::NoLine) {
                            return self.inconsistent();
                        }
                    }
                    if self.disallow_dl[(y / 2, x / 2 + 1)] && y + 1 < self.height * 2 - 1 && x >= 1
                    {
                        if self.decide_edge(y + 1, x + 1, EdgeState::NoLine) {
                            return self.inconsistent();
                        }
                    }
                } else {
                    if self.disallow_dl[(y / 2, x / 2)] && y >= 1 && x >= 1 {
                        if self.decide_edge(y - 1, x - 1, EdgeState::NoLine) {
                            return self.inconsistent();
                        }
                    }
                    if self.disallow_dr[(y / 2, x / 2)] && y >= 1 && x + 1 < self.width * 2 - 1 {
                        if self.decide_edge(y - 1, x + 1, EdgeState::NoLine) {
                            return self.inconsistent();
                        }
                    }
                }
            }
        }

        if self.check_vertex(y / 2, x / 2) {
            return self.inconsistent();
        }
        if self.check_vertex((y + 1) / 2, (x + 1) / 2) {
            return self.inconsistent();
        }
        if let Some(ae) = another_end1 {
            let ay = ae as usize / self.width;
            let ax = ae as usize % self.width;
            if self.check_vertex(ay, ax) {
                return self.inconsistent();
            }
        }
        if let Some(ae) = another_end2 {
            let ay = ae as usize / self.width;
            let ax = ae as usize % self.width;
            if self.check_vertex(ay, ax) {
                return self.inconsistent();
            }
        }

        self.inconsistent()
    }

    fn check_vertex(&mut self, y: usize, x: usize) -> bool {
        let mut degree = if self.has_clue(y, x) { 1 } else { 0 };
        let mut undet = 0;

        let ae = self.another_end[(y, x)];
        if ae <= -2 {
            if x > 0 && self.get_edge(y * 2, x * 2 - 1) == EdgeState::Undecided {
                let ae2 = self.another_end[(y, x - 1)];
                if ae2 <= -2 && ae != ae2 {
                    if self.decide_edge(y * 2, x * 2 - 1, EdgeState::NoLine) {
                        return self.inconsistent();
                    }
                }
            }
            if x < self.width - 1 && self.get_edge(y * 2, x * 2 + 1) == EdgeState::Undecided {
                let ae2 = self.another_end[(y, x + 1)];
                if ae2 <= -2 && ae != ae2 {
                    if self.decide_edge(y * 2, x * 2 + 1, EdgeState::NoLine) {
                        return self.inconsistent();
                    }
                }
            }
            if y > 0 && self.get_edge(y * 2 - 1, x * 2) == EdgeState::Undecided {
                let ae2 = self.another_end[(y - 1, x)];
                if ae2 <= -2 && ae != ae2 {
                    if self.decide_edge(y * 2 - 1, x * 2, EdgeState::NoLine) {
                        return self.inconsistent();
                    }
                }
            }
            if y < self.height - 1 && self.get_edge(y * 2 + 1, x * 2) == EdgeState::Undecided {
                let ae2 = self.another_end[(y + 1, x)];
                if ae2 <= -2 && ae != ae2 {
                    if self.decide_edge(y * 2 + 1, x * 2, EdgeState::NoLine) {
                        return self.inconsistent();
                    }
                }
            }
        }

        if x > 0 {
            match self.get_edge(y * 2, x * 2 - 1) {
                EdgeState::Undecided => undet += 1,
                EdgeState::Line => degree += 1,
                EdgeState::NoLine => {}
            }
        }
        if x < self.width - 1 {
            match self.get_edge(y * 2, x * 2 + 1) {
                EdgeState::Undecided => undet += 1,
                EdgeState::Line => degree += 1,
                EdgeState::NoLine => {}
            }
        }
        if y > 0 {
            match self.get_edge(y * 2 - 1, x * 2) {
                EdgeState::Undecided => undet += 1,
                EdgeState::Line => degree += 1,
                EdgeState::NoLine => {}
            }
        }
        if y < self.height - 1 {
            match self.get_edge(y * 2 + 1, x * 2) {
                EdgeState::Undecided => undet += 1,
                EdgeState::Line => degree += 1,
                EdgeState::NoLine => {}
            }
        }

        if degree > 2 || (degree == 1 && undet == 0) {
            self.set_inconsistent();
            return self.inconsistent();
        }

        if undet == 1 {
            let new_state = if degree == 1 {
                EdgeState::Line
            } else {
                EdgeState::NoLine
            };
            if x > 0 && self.get_edge(y * 2, x * 2 - 1) == EdgeState::Undecided {
                if self.decide_edge(y * 2, x * 2 - 1, new_state) {
                    return self.inconsistent();
                }
            } else if x < self.width - 1 && self.get_edge(y * 2, x * 2 + 1) == EdgeState::Undecided
            {
                if self.decide_edge(y * 2, x * 2 + 1, new_state) {
                    return self.inconsistent();
                }
            } else if y > 0 && self.get_edge(y * 2 - 1, x * 2) == EdgeState::Undecided {
                if self.decide_edge(y * 2 - 1, x * 2, new_state) {
                    return self.inconsistent();
                }
            } else if y < self.height - 1 && self.get_edge(y * 2 + 1, x * 2) == EdgeState::Undecided
            {
                if self.decide_edge(y * 2 + 1, x * 2, new_state) {
                    return self.inconsistent();
                }
            }
        }

        self.inconsistent()
    }

    pub fn cut_based_propagation(&mut self) -> bool {
        loop {
            let mut max_clue = 0;
            for y in 0..self.height {
                for x in 0..self.width {
                    if let Some(value) = self.problem[(y, x)] {
                        max_clue = max_clue.max(value);
                    }
                }
            }

            let mut row_capacity_diff = vec![0; self.height];
            let mut col_capacity_diff = vec![0; self.width];

            let mut clue_pos = vec![(None, None); max_clue as usize + 1];
            for y in 0..self.height {
                for x in 0..self.width {
                    let ae = self.another_end[(y, x)];
                    if ae <= -2 {
                        let clue_value = -ae - 2;
                        if clue_pos[clue_value as usize].0.is_none() {
                            clue_pos[clue_value as usize].0 = Some((y, x));
                        } else {
                            assert!(clue_pos[clue_value as usize].1.is_none());
                            clue_pos[clue_value as usize].1 = Some((y, x));
                        }
                    } else if ae >= 0 {
                        if y * self.width + x < ae as usize {
                            let y2 = ae as usize / self.width;
                            let x2 = ae as usize % self.width;

                            row_capacity_diff[y.min(y2)] += 1;
                            row_capacity_diff[y.max(y2)] -= 1;
                            col_capacity_diff[x.min(x2)] += 1;
                            col_capacity_diff[x.max(x2)] -= 1;
                        }
                    }
                }
            }
            for c in 0..=max_clue {
                if let (Some((y1, x1)), Some((y2, x2))) = clue_pos[c as usize] {
                    row_capacity_diff[y1.min(y2)] -= 1;
                    row_capacity_diff[y1.max(y2)] += 1;
                    col_capacity_diff[x1.min(x2)] -= 1;
                    col_capacity_diff[x1.max(x2)] += 1;
                }
            }
            for y in 0..self.height {
                for x in 0..self.width {
                    if x < self.width - 1 && self.get_edge(y * 2, x * 2 + 1) == EdgeState::Undecided
                    {
                        col_capacity_diff[x] += 1;
                        col_capacity_diff[x + 1] -= 1;
                    }
                    if y < self.height - 1
                        && self.get_edge(y * 2 + 1, x * 2) == EdgeState::Undecided
                    {
                        row_capacity_diff[y] += 1;
                        row_capacity_diff[y + 1] -= 1;
                    }
                }
            }
            let mut row_capacity = row_capacity_diff;
            let mut col_capacity = col_capacity_diff;
            for y in 1..self.height {
                row_capacity[y] += row_capacity[y - 1];
            }
            for x in 1..self.width {
                col_capacity[x] += col_capacity[x - 1];
            }

            for y in 0..(self.height - 1) {
                if row_capacity[y] < 0 {
                    self.set_inconsistent();
                    return self.inconsistent();
                }
            }
            for x in 0..(self.width - 1) {
                if col_capacity[x] < 0 {
                    self.set_inconsistent();
                    return self.inconsistent();
                }
            }

            let mut has_update = false;
            for y in 0..(self.height - 1) {
                if row_capacity[y] == 0 {
                    for x in 0..self.width {
                        if self.get_edge(y * 2 + 1, x * 2) == EdgeState::Undecided {
                            if self.decide_edge(y * 2 + 1, x * 2, EdgeState::NoLine) {
                                return self.inconsistent();
                            }
                            has_update = true;
                        }
                    }
                }
            }
            for x in 0..(self.width - 1) {
                if col_capacity[x] == 0 {
                    for y in 0..self.height {
                        if self.get_edge(y * 2, x * 2 + 1) == EdgeState::Undecided {
                            if self.decide_edge(y * 2, x * 2 + 1, EdgeState::NoLine) {
                                return self.inconsistent();
                            }
                            has_update = true;
                        }
                    }
                }
            }
            if !has_update {
                break;
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
                    if let Some(n) = self.problem[(y / 2, x / 2)] {
                        let v = if n < 10 {
                            n as u8 + b'0'
                        } else {
                            n as u8 - 10 + b'A'
                        };
                        write!(f, "{}", v as char)?;
                    } else {
                        write!(f, "+")?;
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
