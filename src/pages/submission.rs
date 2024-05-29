use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Html,
};
use axum_extra::extract::CookieJar;
use maud::html;
use rust_query::value::Value;

use crate::{
    async_sqlite::DB,
    hash::FileHash,
    pages::{header, Location, ProblemPage},
    AppState,
};

// information about a solution and its performance on a problem
pub async fn submission(
    State(app): State<AppState>,
    Path((problem, solution_hash)): Path<(String, String)>,
    jar: CookieJar,
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
    }

    let data = DB
        .call(move |conn| {
            // list solutions for this problem
            conn.new_query(|q| {
                let exec = q.table(&DB.execution);
                q.filter(exec.instance.problem.file_hash.eq(i64::from(problem_hash)));
                q.filter(exec.solution.program.file_hash.eq(i64::from(solution_hash)));
                q.into_vec(u32::MAX, |row| SolutionStats {
                    seed: row.get(exec.instance.seed) as u64,
                    fuel: row.get(exec.fuel_used),
                })
            })
        })
        .await;

    struct Fail {
        seed: u64,
        message: String,
    }

    let failure = DB
        .call(move |conn| {
            conn.new_query(|q| {
                let failure = q.table(&DB.failure);
                let solution = &failure.solution;
                q.filter(solution.program.file_hash.eq(i64::from(solution_hash)));
                q.filter(solution.problem.file_hash.eq(i64::from(problem_hash)));
                q.into_vec(u32::MAX, |row| Fail {
                    seed: row.get(failure.seed) as u64,
                    message: row.get(failure.message),
                })
            })
        })
        .await;

    let users = DB
        .call(move |conn| {
            conn.new_query(|q| {
                let submission = q.table(&DB.submission);
                q.filter(submission.solution.file_hash.eq(i64::from(solution_hash)));
                q.into_vec(u32::MAX, |row| {
                    // sort by timestamp
                    let _ = row.get(submission.timestamp);
                    row.get(submission.user.github_login)
                })
            })
        })
        .await;

    let location = Location::Problem(
        problem.clone(),
        ProblemPage::Solution(solution_hash.to_string()),
    );
    let res = html! {
        @if let Some(fail) = failure.first() {
            p class="notice" {
                "Failed for seed " (fail.seed)
                pre{(fail.message)}
            }
        }
        p {
            "Discovered by " (users.join(", "))
        }
        table {
            // caption { "Scores" }
            thead {
                tr {
                    th { "Instance Seed" }
                    th { "Fuel Used" }
                }
            }
            tbody {
                @for solution in &data {
                    tr {
                        td {(solution.seed)}
                        td {(solution.fuel)}
                    }
                }
            }
        }
    };
    let res = header(location, &jar, res);
    Ok(Html(res.into_string()))
}
