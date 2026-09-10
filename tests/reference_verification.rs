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

#[test]
fn test_random_clue_placements() {
    use rand::rngs::StdRng;
    use rand::seq::SliceRandom;
    use rand::{Rng, SeedableRng};

    // Include unsatisfiable instances: comparing every answer detects unsound
    // separator deductions as well as failures to find the intended solution.
    let mut rng = StdRng::seed_from_u64(0x766572746578);
    for (height, width) in [(2, 3), (3, 2), (3, 3), (3, 4), (4, 3), (4, 4)] {
        for _ in 0..40 {
            let mut positions: Vec<_> = (0..height)
                .flat_map(|y| (0..width).map(move |x| (y, x)))
                .collect();
            positions.shuffle(&mut rng);
            let mut problem = Problem::new(height, width, None);
            for clue in 0..rng.gen_range(0..=positions.len() / 2) {
                problem[positions[2 * clue]] = Some(clue as i32);
                problem[positions[2 * clue + 1]] = Some(clue as i32);
            }
            reference_solver::compare_answers(&problem);
        }
    }
}
