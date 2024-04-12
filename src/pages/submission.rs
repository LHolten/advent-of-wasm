use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Html,
};
use maud::html;
use rust_query::{client::QueryBuilder, value::Value};

use crate::{
    hash::FileHash,
    pages::{header, Location, ProblemPage},
    tables, AppState,
};

// information about a solution and its performance on a problem
pub async fn submission(
    State(app): State<AppState>,
    Path((problem, solution_hash)): Path<(String, String)>,
) -> Result<Html<String>, StatusCode> {
    println!("got user for {problem}");

    let problem_hash = *app
        .problem_dir
        .mapping
        .get(&problem)
        .ok_or(StatusCode::NOT_FOUND)?;
    let solution_hash: FileHash = solution_hash.parse().map_err(|_| StatusCode::NOT_FOUND)?;

    struct SolutionStats {
        seed: u64,
        fuel: i64,
        status: String,
    }

    let data = app
        .conn
        .call(move |conn| {
            // list solutions for this problem
            conn.new_query(|q| {
                let exec = q.table(tables::Execution);
                q.filter(exec.instance.problem.file_hash.eq(i64::from(problem_hash)));
                q.filter(exec.solution.file_hash.eq(i64::from(solution_hash)));
                q.into_vec(u32::MAX, |row| SolutionStats {
                    seed: row.get(exec.instance.seed) as u64,
                    fuel: row.get(exec.fuel_used),
                    status: if let Some(answer) = row.get(exec.answer) {
                        if answer == row.get(exec.instance.answer) {
                            "correct"
                        } else {
                            "wrong"
                        }
                    } else {
                        "error"
                    }
                    .to_owned(),
                })
            })
        })
        .await;

    let location = Location::Problem(
        problem.clone(),
        ProblemPage::Solution(solution_hash.to_string()),
    );
    let res = html! {
        (header(location))
        table {
            // caption { "Scores" }
            thead {
                tr {
                    th { "Instance Seed" }
                    th { "Fuel Used" }
                    th { "Status" }
                }
            }
            tbody {
                @for solution in &data {
                    tr {
                        td {(solution.seed)}
                        td {(solution.fuel)}
                        td {(solution.status)}
                    }
                }
            }
        }
    };
    Ok(Html(res.into_string()))
}
