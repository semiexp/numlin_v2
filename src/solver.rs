use crate::board::Board;
use crate::{Answers, EdgeState, Problem};

fn backtrack(board: &mut Board, answers: &mut Answers, y: usize, x: usize) {
    if y == board.height() {
        // We've reached the end of the board, so we have a complete solution
        answers.add_answer(board.to_answer());
        return;
    }

    let (next_y, next_x) = if x + 1 < board.width() {
        (y, x + 1)
    } else {
        (y + 1, 0)
    };

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
            backtrack(board, answers, next_y, next_x);
        }

        board.undo();
    }
}

pub fn solve(problem: Problem) -> Answers {
    let mut board = Board::new(problem);

    // Implement a simple backtracking algorithm to solve the problem
    let mut answers = Answers::new();
    backtrack(&mut board, &mut answers, 0, 0);
    answers
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

        let answers = solve(problem);
        assert_eq!(answers.len(), 1); // There is one valid solution for this
    }
}
