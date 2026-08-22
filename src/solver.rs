use crate::board::Board;
use crate::{Answers, EdgeState, Problem, SearchStats, SolveResult};

fn get_next_cell(board: &Board, y: usize, x: usize) -> (usize, usize) {
    let mut y = y;
    let mut x = x;

    while y < board.height() - 1 {
        if board.get_edge(y * 2, x * 2 + 1) == EdgeState::Undecided {
            return (y, x);
        }
        if x == board.width() - 2 {
            y += 1;
            x = 0;
        } else {
            x += 1;
        }
    }

    (y, x)
}

fn backtrack(
    board: &mut Board,
    answers: &mut Answers,
    stats: &mut SearchStats,
    y: usize,
    x: usize,
) {
    // Decide the edges around the vertex at (y, x) and recursively backtrack to find all valid solutions.
    // Precondition: the up-edge and left-edge of the vertex at (y, x) have already been decided.
    //
    // From this precondition, for the rightmost and the bottommost vertices, the right-edge and the down-edge should
    // have already been decided. Thus we can skip the rightmost and the bottommost vertices in the backtracking process.
    // This is why `get_next_cell` skips the rightmost and the bottommost vertices.

    stats.visited_boards += 1;

    if y == board.height() - 1 {
        // We've reached the end of the board, so we have a complete solution
        answers.add_answer(board.to_answer());
        return;
    }

    let cur_degree = if board.has_clue(y, x) { 1 } else { 0 }
        + if x > 0 && board.get_edge(y * 2, x * 2 - 1) == EdgeState::Line {
            1
        } else {
            0
        }
        + if y > 0 && board.get_edge(y * 2 - 1, x * 2) == EdgeState::Line {
            1
        } else {
            0
        };

    for m in 0..4 {
        let right = m & 1 != 0;
        let down = m & 2 != 0;

        if x == board.width() - 1 && right {
            continue; // Can't go right at the last column
        }
        if y == board.height() - 1 && down {
            continue; // Can't go down at the last row
        }

        let total_degree = cur_degree + if right { 1 } else { 0 } + if down { 1 } else { 0 };
        if total_degree != 0 && total_degree != 2 {
            continue; // Must have either 0 or 2 edges connected to a vertex
        }

        board.add_checkpoint();
        if x < board.width() - 1 {
            board.decide_edge(
                y * 2,
                x * 2 + 1,
                if right {
                    EdgeState::Line
                } else {
                    EdgeState::NoLine
                },
            );
        }
        if y < board.height() - 1 {
            board.decide_edge(
                y * 2 + 1,
                x * 2,
                if down {
                    EdgeState::Line
                } else {
                    EdgeState::NoLine
                },
            );
        }

        if !board.inconsistent() {
            board.cut_based_propagation();
        }

        if !board.inconsistent() {
            let (next_y, next_x) = get_next_cell(board, y, x);
            backtrack(board, answers, stats, next_y, next_x);
        }

        board.undo();
    }
}

pub fn solve(problem: Problem) -> SolveResult {
    let mut board = Board::new(problem);

    // Implement a simple backtracking algorithm to solve the problem
    let mut answers = Answers::new();
    let mut stats = SearchStats::new();
    backtrack(&mut board, &mut answers, &mut stats, 0, 0);
    SolveResult { answers, stats }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem_from_array(array: &[Vec<i32>]) -> Problem {
        let height = array.len();
        let width = array[0].len();
        let mut problem = Problem::new(height, width, None);
        for (y, row) in array.iter().enumerate() {
            for (x, &value) in row.iter().enumerate() {
                if value > 0 {
                    problem[(y, x)] = Some(value - 1); // Store as zero-based internally
                }
            }
        }
        problem
    }

    #[test]
    fn test_solver_simple_problem() {
        let problem = problem_from_array(&[
            vec![2, 1, 0, 2],
            vec![0, 0, 0, 3],
            vec![0, 1, 0, 0],
            vec![0, 0, 0, 3],
        ]);

        let result = solve(problem);
        assert_eq!(result.answers().len(), 1); // There is one valid solution for this
        assert!(result.stats().visited_boards() > 0);
    }

    #[test]
    fn test_visited_boards_counts_backtrack_calls() {
        let problem = problem_from_array(&[vec![0]]);

        let result = solve(problem);

        assert_eq!(result.answers().len(), 1);
        assert_eq!(result.stats().visited_boards(), 1);
    }
}
