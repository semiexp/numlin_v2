mod reference_solver;

use numlin_v2::Problem;

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
fn test_simple_problem() {
    let problem = problem_from_array(&[
        vec![2, 1, 0, 2],
        vec![0, 0, 0, 3],
        vec![0, 1, 0, 0],
        vec![0, 0, 0, 3],
    ]);

    reference_solver::compare_answers(&problem);
}

#[test]
fn test_non_unique() {
    let problem = problem_from_array(&[
        vec![1, 0, 2, 0],
        vec![0, 0, 0, 0],
        vec![0, 1, 0, 0],
        vec![0, 0, 0, 0],
    ]);

    reference_solver::compare_answers(&problem);
}

#[test]
fn test_no_cycle() {
    let problem = problem_from_array(&[
        vec![0, 0, 0, 0, 0],
        vec![0, 1, 0, 1, 0],
        vec![0, 0, 0, 0, 0],
    ]);

    reference_solver::compare_answers(&problem);
}
