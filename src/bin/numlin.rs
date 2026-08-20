use std::env;
use std::process::ExitCode;

use numlin_v2::solver;
use numlin_v2::urls::decode_url;
use numlin_v2::{EdgeState, Problem};

fn main() -> ExitCode {
    let mut args = env::args();
    let program = args.next().unwrap_or_else(|| "numlin".to_string());
    let Some(url) = args.next() else {
        eprintln!("usage: {program} URL");
        return ExitCode::from(2);
    };

    if args.next().is_some() {
        eprintln!("usage: {program} URL");
        return ExitCode::from(2);
    }

    let problem = decode_url(&url);
    let answers = solver::solve(problem.clone());
    println!("solutions: {}", answers.len());

    if answers.len() == 0 {
        return ExitCode::from(1);
    }

    print_answer(&answers[0], &problem);
    ExitCode::SUCCESS
}

fn print_answer(answer: &numlin_v2::Answer, problem: &Problem) {
    for y in 0..answer.height() {
        for x in 0..answer.width() {
            match problem[(y, x)] {
                Some(value) => print!("{}", clue_char(value)),
                None => print!("+"),
            }
            if x + 1 < answer.width() {
                print!(
                    "{}",
                    if answer.get_horizontal(y, x) == EdgeState::Line {
                        '─'
                    } else {
                        ' '
                    }
                );
            }
        }
        println!();

        if y + 1 < answer.height() {
            for x in 0..answer.width() {
                print!(
                    "{}",
                    if answer.get_vertical(y, x) == EdgeState::Line {
                        '│'
                    } else {
                        ' '
                    }
                );
                if x + 1 < answer.width() {
                    print!(" ");
                }
            }
            println!();
        }
    }
}

// Base-62 representation for clue values: 1..9, a..z, A..Z.
fn clue_char(value: i32) -> char {
    const DIGITS: &[u8; 62] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    DIGITS
        .get(value as usize)
        .copied()
        .map(char::from)
        .unwrap_or_else(|| panic!("clue value cannot be represented by one character: {value}"))
}
