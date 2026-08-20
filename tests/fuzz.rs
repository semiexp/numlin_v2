mod instance_generator;
mod reference_solver;

fn run_fuzz(instances: &[(usize, usize, u64)]) {
    let mut seed = 0;

    for &(height, width, count) in instances {
        for _ in 0..count {
            let problem = instance_generator::generate_instance_by_csp(height, width, seed);
            if let Some(problem) = problem {
                reference_solver::compare_answers(&problem);
            }
            seed += 1;
        }
    }
}

#[test]
fn test_fuzz() {
    run_fuzz(&[
        (7, 8, 10),
        (8, 7, 10),
        (8, 8, 10),
        (9, 9, 10),
    ]);
}
