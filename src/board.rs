use crate::grid::Grid;
use crate::vertex_cut::VertexCuts;
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

    // Cut slack = undecided crossing edges + crossings of clue-free paths
    //            - crossings required between the two endpoints of each clue.
    // Range contributions are stored as differences, so an endpoint change costs O(1).
    row_capacity_diff: Vec<i32>,
    col_capacity_diff: Vec<i32>,
    cut_clue_pos: Vec<[i32; 2]>,

    // Reused snapshots: decisions during a propagation pass must not change the
    // capacities being scanned in that pass (matching full recomputation).
    row_capacity: Vec<i32>,
    col_capacity: Vec<i32>,

    // Packed undecided edges let saturated straight cuts skip decided edges.
    row_undecided: Vec<u64>,
    col_undecided: Vec<u64>,
    vertex_cuts: VertexCuts,

    inconsistent: bool,

    history: Vec<History>, // History of decisions made on the board, used for backtracking.
}

impl Board {
    pub fn new(problem: Problem) -> Self {
        let height = problem.height();
        let width = problem.width();
        let mut has_clue = Grid::new(height, width, false);
        let mut another_end = Grid::new(height, width, -1);
        let mut max_clue = 0;

        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;

                if let Some(value) = problem[(y, x)] {
                    max_clue = max_clue.max(value);
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

        let mut board = Self {
            height,
            width,
            problem,
            has_clue,
            edge_state: Grid::new(2 * height - 1, 2 * width - 1, EdgeState::Undecided),
            another_end,
            disallow_dl,
            disallow_dr,
            row_capacity_diff: vec![0; height],
            col_capacity_diff: vec![0; width],
            cut_clue_pos: vec![[-1; 2]; max_clue as usize + 1],
            row_capacity: vec![0; height],
            col_capacity: vec![0; width],
            row_undecided: Self::undecided_cut_edges(height - 1, width),
            col_undecided: Self::undecided_cut_edges(width - 1, height),
            vertex_cuts: VertexCuts::new(height, width, max_clue as usize + 1),
            inconsistent: false,
            history: Vec::new(),
        };
        // Initially every crossing edge is undecided.
        board.row_capacity_diff[0] += width as i32;
        board.row_capacity_diff[height - 1] -= width as i32;
        board.col_capacity_diff[0] += height as i32;
        board.col_capacity_diff[width - 1] -= height as i32;
        for y in 0..height {
            for x in 0..width {
                let p = (y * width + x) as i32;
                board.update_cut_endpoint(p, board.another_end[p], true);
                board.vertex_cuts.endpoint_changed(p);
            }
        }
        #[cfg(any(test, feature = "verify-cut-capacity"))]
        board.verify_cut_capacity();
        board
    }

    fn undecided_cut_edges(cuts: usize, length: usize) -> Vec<u64> {
        let words = length.div_ceil(64);
        let mut edges = vec![u64::MAX; cuts * words];
        if length % 64 != 0 {
            for cut in 0..cuts {
                edges[cut * words + words - 1] = (1 << (length % 64)) - 1;
            }
        }
        edges
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
                    if self.edge_state[(y, x)] != EdgeState::Undecided {
                        self.update_cut_edge(y, x, 1);
                    }
                    self.edge_state[(y, x)] = EdgeState::Undecided;
                }
                History::AnotherEnd(p, v) => {
                    self.replace_another_end(p, v);
                }
                History::Inconsistent => {
                    self.inconsistent = false;
                }
            }
        }
        #[cfg(any(test, feature = "verify-cut-capacity"))]
        self.verify_cut_capacity();
    }

    // Update one undecided crossing edge, including the inverse operation on undo.
    fn update_cut_edge(&mut self, y: usize, x: usize, delta: i32) {
        let (diff, index) = if y % 2 == 0 {
            (&mut self.col_capacity_diff, x / 2)
        } else {
            (&mut self.row_capacity_diff, y / 2)
        };
        diff[index] += delta;
        diff[index + 1] -= delta;
        let (mask, index, bit) = if y % 2 == 0 {
            (
                &mut self.col_undecided,
                x / 2 * self.height.div_ceil(64) + y / 2 / 64,
                y / 2 % 64,
            )
        } else {
            (
                &mut self.row_undecided,
                y / 2 * self.width.div_ceil(64) + x / 2 / 64,
                x / 2 % 64,
            )
        };
        if delta > 0 {
            mask[index] |= 1 << bit;
        } else {
            mask[index] &= !(1 << bit);
        }
        self.vertex_cuts.edge_changed(y, x);
    }

    fn update_cut_pair(&mut self, p: i32, q: i32, delta: i32) {
        let (y1, x1) = (p as usize / self.width, p as usize % self.width);
        let (y2, x2) = (q as usize / self.width, q as usize % self.width);
        self.row_capacity_diff[y1.min(y2)] += delta;
        self.row_capacity_diff[y1.max(y2)] -= delta;
        self.col_capacity_diff[x1.min(x2)] += delta;
        self.col_capacity_diff[x1.max(x2)] -= delta;
    }

    fn update_cut_endpoint(&mut self, p: i32, ae: i32, adding: bool) {
        if ae >= 0 {
            // Count a clue-free path only at its smaller endpoint.
            if p < ae {
                self.update_cut_pair(p, ae, if adding { 1 } else { -1 });
            }
        } else if ae <= -2 {
            let clue = (-ae - 2) as usize;
            let mut pos = self.cut_clue_pos[clue];
            if adding {
                if pos[0] == -1 {
                    pos[0] = p;
                } else {
                    assert_eq!(pos[1], -1, "more than two endpoints for a clue");
                    pos[1] = p;
                }
                if pos[0] >= 0 && pos[1] >= 0 {
                    self.update_cut_pair(pos[0], pos[1], -1);
                }
            } else {
                if pos[0] >= 0 && pos[1] >= 0 {
                    self.update_cut_pair(pos[0], pos[1], 1);
                }
                if pos[0] == p {
                    pos[0] = -1;
                } else {
                    debug_assert_eq!(pos[1], p);
                    pos[1] = -1;
                }
            }
            self.cut_clue_pos[clue] = pos;
        }
    }

    fn replace_another_end(&mut self, p: i32, new_value: i32) {
        self.update_cut_endpoint(p, self.another_end[p], false);
        self.another_end[p] = new_value;
        self.vertex_cuts.endpoint_changed(p);
        self.update_cut_endpoint(p, new_value, true);
    }

    fn update_another_end(&mut self, p: i32, new_value: i32) {
        let old_value = self.another_end[p];
        if old_value != new_value {
            self.history.push(History::AnotherEnd(p, old_value));
            self.replace_another_end(p, new_value);
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

        if state != EdgeState::Undecided {
            self.update_cut_edge(y, x, -1);
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

    // Deliberately retain the original full-board calculation as an independent
    // oracle. Integration tests can opt in; ordinary builds contain none of it.
    #[cfg(any(test, feature = "verify-cut-capacity"))]
    fn verify_cut_capacity(&mut self) {
        self.vertex_cuts
            .sync(&self.another_end, &self.edge_state, &self.cut_clue_pos);
        self.vertex_cuts.verify(&self.another_end, &self.edge_state);
        for y in 0..self.height {
            for x in 0..self.width {
                if y + 1 < self.height {
                    let mask = self.row_undecided[y * self.width.div_ceil(64) + x / 64];
                    assert_eq!(
                        mask & (1 << (x % 64)) != 0,
                        self.get_edge(2 * y + 1, 2 * x) == EdgeState::Undecided
                    );
                }
                if x + 1 < self.width {
                    let mask = self.col_undecided[x * self.height.div_ceil(64) + y / 64];
                    assert_eq!(
                        mask & (1 << (y % 64)) != 0,
                        self.get_edge(2 * y, 2 * x + 1) == EdgeState::Undecided
                    );
                }
            }
        }
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
        assert_eq!(
            self.row_capacity_diff, row_capacity_diff,
            "row cut mismatch: {self:?}"
        );
        assert_eq!(
            self.col_capacity_diff, col_capacity_diff,
            "column cut mismatch: {self:?}"
        );
        for (clue, (first, second)) in clue_pos.into_iter().enumerate() {
            let mut expected =
                [first, second].map(|pos| pos.map_or(-1, |(y, x)| (y * self.width + x) as i32));
            let mut actual = self.cut_clue_pos[clue];
            expected.sort_unstable();
            actual.sort_unstable();
            assert_eq!(actual, expected, "clue endpoint mismatch for {clue}: {self:?}");
        }
    }

    pub fn cut_based_propagation(&mut self) -> bool {
        loop {
            #[cfg(any(test, feature = "verify-cut-capacity"))]
            self.verify_cut_capacity();

            let mut capacity = 0;
            for y in 0..self.height {
                capacity += self.row_capacity_diff[y];
                self.row_capacity[y] = capacity;
            }
            capacity = 0;
            for x in 0..self.width {
                capacity += self.col_capacity_diff[x];
                self.col_capacity[x] = capacity;
            }

            for y in 0..(self.height - 1) {
                if self.row_capacity[y] < 0 {
                    self.set_inconsistent();
                    return self.inconsistent();
                }
            }
            for x in 0..(self.width - 1) {
                if self.col_capacity[x] < 0 {
                    self.set_inconsistent();
                    return self.inconsistent();
                }
            }

            let mut has_update = false;
            for y in 0..(self.height - 1) {
                if self.row_capacity[y] == 0 {
                    let words = self.width.div_ceil(64);
                    for word in 0..words {
                        while self.row_undecided[y * words + word] != 0 {
                            let x = word * 64
                                + self.row_undecided[y * words + word].trailing_zeros() as usize;
                            if self.decide_edge(y * 2 + 1, x * 2, EdgeState::Line) {
                                return self.inconsistent();
                            }
                            has_update = true;
                        }
                    }
                }
            }
            for x in 0..(self.width - 1) {
                if self.col_capacity[x] == 0 {
                    let words = self.height.div_ceil(64);
                    for word in 0..words {
                        while self.col_undecided[x * words + word] != 0 {
                            let y = word * 64
                                + self.col_undecided[x * words + word].trailing_zeros() as usize;
                            if self.decide_edge(y * 2, x * 2 + 1, EdgeState::Line) {
                                return self.inconsistent();
                            }
                            has_update = true;
                        }
                    }
                }
            }
            if has_update {
                continue;
            }
            self.vertex_cuts
                .sync(&self.another_end, &self.edge_state, &self.cut_clue_pos);
            if !self.vertex_cuts.propagate(&self.another_end, &self.edge_state) {
                self.set_inconsistent();
                return true;
            }
            while let Some(((y, x), state)) = self.vertex_cuts.forced.pop() {
                if self.get_edge(y, x) == EdgeState::Undecided {
                    has_update = true;
                }
                if self.decide_edge(y, x, state) {
                    return true;
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

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::seq::SliceRandom;
    use rand::{Rng, SeedableRng};

    #[test]
    fn saturated_cuts_complete_paths_and_undo() {
        for (height, width) in [(1, 5), (5, 1), (1, 70), (70, 1)] {
            let mut problem = Problem::new(height, width, None);
            problem[(0, 0)] = Some(17); // Sparse clue IDs are supported.
            problem[(height - 1, width - 1)] = Some(17);
            let mut board = Board::new(problem);
            let initial_edges = board.edge_state.clone();
            let initial_endpoints = board.another_end.clone();
            board.add_checkpoint();
            assert!(!board.cut_based_propagation());
            board.verify_cut_capacity();
            for i in 0..height.max(width) - 1 {
                let edge = if height == 1 {
                    (0, 2 * i + 1)
                } else {
                    (2 * i + 1, 0)
                };
                assert_eq!(board.get_edge(edge.0, edge.1), EdgeState::Line);
            }
            assert_eq!(board.cut_clue_pos[17], [-1, -1]);
            board.undo();
            assert_eq!(board.edge_state, initial_edges);
            assert_eq!(board.another_end, initial_endpoints);
            assert!(!board.inconsistent());
        }
    }

    #[test]
    fn negative_cuts_reject_and_undo() {
        for (height, width) in [(1, 5), (5, 1)] {
            let mut problem = Problem::new(height, width, None);
            for (i, clue) in [(0, 0), (1, 7), (3, 0), (4, 7)] {
                let pos = if height == 1 { (0, i) } else { (i, 0) };
                problem[pos] = Some(clue);
            }
            let mut board = Board::new(problem);
            board.add_checkpoint();
            assert!(board.cut_based_propagation());
            board.verify_cut_capacity();
            board.undo();
            assert!(!board.inconsistent());
            assert!(board.cut_based_propagation());
        }
    }

    #[test]
    fn random_decisions_propagation_and_nested_undo_match_full_recomputation() {
        let mut rng = StdRng::seed_from_u64(0x637574);
        for (height, width) in [(1, 1), (1, 6), (6, 1), (3, 4), (4, 3), (6, 6)] {
            for _ in 0..40 {
                let mut problem = Problem::new(height, width, None);
                let mut vertices: Vec<_> = (0..height)
                    .flat_map(|y| (0..width).map(move |x| (y, x)))
                    .collect();
                vertices.shuffle(&mut rng);
                let pairs = rng.gen_range(0..=vertices.len() / 2);
                for pair in 0..pairs {
                    problem[vertices[2 * pair]] = Some((pair * 3) as i32);
                    problem[vertices[2 * pair + 1]] = Some((pair * 3) as i32);
                }
                // Also exercise the existing behavior for unpaired clues.
                if 2 * pairs < vertices.len() && rng.gen_bool(0.5) {
                    problem[vertices[2 * pairs]] = Some((pairs * 3) as i32);
                }
                let edges: Vec<_> = (0..2 * height - 1)
                    .flat_map(|y| (0..2 * width - 1).map(move |x| (y, x)))
                    .filter(|&(y, x)| y % 2 != x % 2)
                    .collect();
                let mut board = Board::new(problem);
                let mut snapshots = Vec::new();
                for _ in 0..120 {
                    if !snapshots.is_empty() && (board.inconsistent() || rng.gen_bool(0.3)) {
                        board.undo();
                        assert_eq!(
                            (
                                board.edge_state.clone(),
                                board.another_end.clone(),
                                board.inconsistent(),
                            ),
                            snapshots.pop().unwrap()
                        );
                        continue;
                    }
                    snapshots.push((
                        board.edge_state.clone(),
                        board.another_end.clone(),
                        board.inconsistent(),
                    ));
                    board.add_checkpoint();
                    if !edges.is_empty() && rng.gen_bool(0.8) {
                        let &(y, x) = edges.choose(&mut rng).unwrap();
                        let state = if rng.gen_bool(0.5) {
                            EdgeState::Line
                        } else {
                            EdgeState::NoLine
                        };
                        // Include already decided edges, contradictory decisions and
                        // recursive vertex propagation, then check even failed branches.
                        board.decide_edge(y, x, state);
                        board.verify_cut_capacity();
                    }
                    if !board.inconsistent() && rng.gen_bool(0.5) {
                        board.cut_based_propagation();
                        board.verify_cut_capacity();
                    }
                }
                while let Some(snapshot) = snapshots.pop() {
                    board.undo();
                    assert_eq!(
                        (
                            board.edge_state.clone(),
                            board.another_end.clone(),
                            board.inconsistent(),
                        ),
                        snapshot
                    );
                }
                assert!(board.history.is_empty());
            }
        }
    }

    #[test]
    #[should_panic(expected = "row cut mismatch")]
    fn verifier_detects_corrupted_capacity() {
        let mut board = Board::new(Problem::new(2, 3, None));
        board.row_capacity_diff[0] += 1;
        board.verify_cut_capacity();
    }
}
