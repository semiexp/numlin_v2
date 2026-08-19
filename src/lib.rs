use std::ops::Index;
pub mod board;
pub mod grid;
pub mod solver;

use grid::Grid;

pub type Problem = Grid<Option<i32>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeState {
    Undecided,
    Line,
    NoLine,
}

pub struct Answer {
    edge_state: Grid<EdgeState>,
}

impl Answer {
    fn new(edge_state: Grid<EdgeState>) -> Self {
        Self { edge_state }
    }

    pub fn get(&self, y: usize, x: usize) -> EdgeState {
        assert!(y < self.edge_state.height() * 2 - 1);
        assert!(x < self.edge_state.width() * 2 - 1);
        assert!(y % 2 != x % 2);
        self.edge_state[(y, x)]
    }

    pub fn get_horizontal(&self, y: usize, x: usize) -> EdgeState {
        assert!(y < self.edge_state.height() && x < self.edge_state.width() - 1);
        self.edge_state[(y * 2, x * 2 + 1)]
    }

    pub fn get_vertical(&self, y: usize, x: usize) -> EdgeState {
        assert!(y < self.edge_state.height() - 1 && x < self.edge_state.width());
        self.edge_state[(y * 2 + 1, x * 2)]
    }
}

pub struct Answers {
    answers: Vec<Answer>,
}

impl Answers {
    fn new() -> Self {
        Self {
            answers: Vec::new(),
        }
    }

    fn add_answer(&mut self, answer: Answer) {
        self.answers.push(answer);
    }

    pub fn len(&self) -> usize {
        self.answers.len()
    }
}

impl Index<usize> for Answers {
    type Output = Answer;

    fn index(&self, index: usize) -> &Self::Output {
        &self.answers[index]
    }
}
