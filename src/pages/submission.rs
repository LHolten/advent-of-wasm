use axum::{extract::Path, response::Html};
use axum_extra::extract::CookieJar;
use maud::html;
use rust_query::Value;

use crate::{
    db,
    hash::FileHash,
    migration::{DB, TABLES},
    pages::{header, Location, ProblemPage},
};

// information about a solution and its performance on a problem
pub async fn submission(
    Path((problem_name, program_hash)): Path<(String, String)>,
    jar: CookieJar,
) -> Result<Html<String>, String> {
    println!("got user for {problem_name}");

    let problem = db::get_problem(&problem_name)?;

    let program_hash: FileHash = program_hash
        .parse()
        .map_err(|_| "program hash is not formatted correctly")?;
    let program = DB
        .get(TABLES.file.unique(i64::from(program_hash)))
        .ok_or("could not find program")?;

    let solution = DB
        .get(TABLES.solution.unique(program, problem))
        .ok_or("program was never submitted for problem")?;

    struct ExecutionStats {
        seed: u64,
        fuel: i64,
    }

    // list executions for this problem
    let data = DB.exec(|q| {
        let exec = q.table(&TABLES.execution);
        q.filter(exec.instance().problem().eq(problem));
        q.filter(exec.solution().program().eq(program));
        q.into_vec(|row| ExecutionStats {
            seed: row.get(exec.instance().seed()) as u64,
            fuel: row.get(exec.fuel_used()),
        })
    });

    let failure = DB.get(TABLES.failure.unique(solution));

    let users = DB.exec(|q| {
        let submission = q.table(&TABLES.submission);
        q.filter(submission.solution().eq(program));
        q.into_vec(|row| {
            // sort by timestamp
            let _ = row.get(submission.timestamp());
            row.get(submission.user().github_login())
        })
    });

    let location = Location::Problem(
        problem_name,
        ProblemPage::Solution(program_hash.to_string()),
    );
    let res = html! {
        @if let Some(fail) = failure {
            p class="notice" {
                "Failed for seed " (DB.get(fail.seed()) as u64)
                pre{(DB.get(fail.message()))}
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
