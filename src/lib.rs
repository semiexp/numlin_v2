pub mod board;
pub mod grid;
pub mod problem;
pub mod solver;

pub type Problem = grid::Grid<Option<i32>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeState {
    Undecided,
    Line,
    NoLine,
}
