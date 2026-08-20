use cspuz_rs::graph;
use cspuz_rs::solver::{Solver, TRUE};

use numlin_v2::{Answer, EdgeState, Problem};

fn solve_by_csp(problem: &Problem) -> Vec<graph::BoolGridEdgesModel> {
    let height = problem.height();
    let width = problem.width();

    let mut solver = Solver::new();

    let is_line = graph::GridEdges::new(&mut solver, (height - 1, width - 1));
    solver.add_answer_key_bool(&is_line.horizontal);
    solver.add_answer_key_bool(&is_line.vertical);

    for y in 0..height {
        for x in 0..width {
            if problem[(y, x)].is_some() {
                solver.add_expr(is_line.vertex_neighbors((y, x)).count_true().eq(1));
            } else {
                solver.add_expr(
                    is_line.vertex_neighbors((y, x)).count_true().eq(0)
                        | is_line.vertex_neighbors((y, x)).count_true().eq(2),
                );
            }
        }
    }

    // forbid trivial detour
    if numlin_v2::OPTIMIZATION_DISALLOW_TRIVIAL_DETOUR {
        for y in 0..(height - 1) {
            for x in 0..(width - 1) {
                solver.add_expr(is_line.cell_neighbors((y, x)).count_true().le(2));
            }
        }
    }

    // L-shape canonization
    if numlin_v2::OPTIMIZATION_L_SHAPE_CANONIZATION {
        for y in 0..(height - 1) {
            for x in 0..(width - 1) {
                if problem[(y + 1, x + 1)].is_none() {
                    if y == height - 2 || x == width - 2 {
                        solver.add_expr(
                            !(is_line.horizontal.at((y, x)) & is_line.vertical.at((y, x))),
                        );
                    } else {
                        solver.add_expr(
                            (is_line.horizontal.at((y, x)) & is_line.vertical.at((y, x))).imp(
                                is_line.horizontal.at((y + 1, x + 1))
                                    & is_line.vertical.at((y + 1, x + 1)),
                            ),
                        );
                    }
                }
                if problem[(y + 1, x)].is_none() {
                    if y == height - 2 || x == 0 {
                        solver.add_expr(
                            !(is_line.horizontal.at((y, x)) & is_line.vertical.at((y, x + 1))),
                        );
                    } else {
                        solver.add_expr(
                            (is_line.horizontal.at((y, x)) & is_line.vertical.at((y, x + 1))).imp(
                                is_line.horizontal.at((y + 1, x - 1))
                                    & is_line.vertical.at((y + 1, x)),
                            ),
                        );
                    }
                }
            }
        }
    }

    let mut max_clue = 0;
    for y in 0..height {
        for x in 0..width {
            if let Some(clue) = problem[(y, x)] {
                max_clue = max_clue.max(clue);
            }
        }
    }

    let cell_clue_id = &solver.int_var_2d((height, width), 0, max_clue);
    for y in 0..height {
        for x in 0..width {
            if let Some(clue) = problem[(y, x)] {
                solver.add_expr(cell_clue_id.at((y, x)).eq(clue));
            }
        }
    }

    solver.add_expr(
        is_line.horizontal.imp(
            cell_clue_id
                .slice((.., ..(width - 1)))
                .eq(cell_clue_id.slice((.., 1..))),
        ),
    );
    solver.add_expr(
        is_line.vertical.imp(
            cell_clue_id
                .slice((..(height - 1), ..))
                .eq(cell_clue_id.slice((1.., ..))),
        ),
    );

    // acyclic
    let mut graph = graph::Graph::new((height - 1) * (width - 1) + 1);
    let outside = (height - 1) * (width - 1);
    let mut indicator = vec![];

    for y in 0..height {
        for x in 0..(width - 1) {
            let v1 = if y == 0 {
                outside
            } else {
                (y - 1) * (width - 1) + x
            };
            let v2 = if y == height - 1 {
                outside
            } else {
                y * (width - 1) + x
            };
            graph.add_edge(v1, v2);
            indicator.push(!is_line.horizontal.at((y, x)));
        }
    }
    for y in 0..(height - 1) {
        for x in 0..width {
            let v1 = if x == 0 {
                outside
            } else {
                y * (width - 1) + (x - 1)
            };
            let v2 = if x == width - 1 {
                outside
            } else {
                y * (width - 1) + x
            };
            graph.add_edge(v1, v2);
            indicator.push(!is_line.vertical.at((y, x)));
        }
    }
    let is_active = vec![TRUE; graph.n_vertices()];
    graph::active_vertices_connected_via_active_edges(&mut solver, &is_active, &indicator, &graph);

    solver
        .answer_iter()
        .map(|ans| ans.get_unwrap(&is_line))
        .collect()
}

fn flatten_csp_answer(ans: &graph::BoolGridEdgesModel) -> Vec<bool> {
    let mut result = vec![];
    let height = ans.horizontal.len();
    let width = ans.horizontal[0].len() + 1;

    for y in 0..height {
        for x in 0..(width - 1) {
            result.push(ans.horizontal[y][x]);
        }
    }
    for y in 0..(height - 1) {
        for x in 0..width {
            result.push(ans.vertical[y][x]);
        }
    }

    result
}

fn flatten_numlin_answer(ans: &Answer) -> Vec<bool> {
    let mut result = vec![];
    let height = ans.height();
    let width = ans.width();

    for y in 0..height {
        for x in 0..(width - 1) {
            let v = match ans.get_horizontal(y, x) {
                EdgeState::Line => true,
                EdgeState::NoLine => false,
                EdgeState::Undecided => panic!(),
            };
            result.push(v);
        }
    }
    for y in 0..(height - 1) {
        for x in 0..width {
            let v = match ans.get_vertical(y, x) {
                EdgeState::Line => true,
                EdgeState::NoLine => false,
                EdgeState::Undecided => panic!(),
            };
            result.push(v);
        }
    }

    result
}

pub fn compare_answers(problem: &Problem) {
    let csp_answers = solve_by_csp(problem);
    let numlin_answers = numlin_v2::solver::solve(problem.clone());

    assert_eq!(csp_answers.len(), numlin_answers.len());

    let mut csp_answers = csp_answers
        .iter()
        .map(|ans| flatten_csp_answer(ans))
        .collect::<Vec<_>>();
    let mut numlin_answers = (0..numlin_answers.len())
        .map(|i| flatten_numlin_answer(&numlin_answers[i]))
        .collect::<Vec<_>>();

    csp_answers.sort();
    numlin_answers.sort();

    assert!(
        csp_answers == numlin_answers,
        "CSP and Numlin answers do not match"
    );
}
