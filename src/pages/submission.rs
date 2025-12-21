use axum::{
    extract::{Path, State},
    response::Html,
};
use axum_extra::extract::CookieJar;
use maud::html;
use rust_query::Select;

use crate::{
    db,
    hash::FileHash,
    migration::{Execution, Failure, File, Solution, Submission},
    pages::{header, Location, ProblemPage},
    AppState,
};

// information about a solution and its performance on a problem
pub async fn submission(
    app: State<AppState>,
    Path((problem_name, program_hash)): Path<(String, String)>,
    jar: CookieJar,
) -> Result<Html<String>, String> {
    println!("got user for {problem_name}");

    app.read_transaction(move |db| {
        let problem = db::get_problem(&db, &problem_name)?;

        let program_hash: FileHash = program_hash
            .parse()
            .map_err(|_| "program hash is not formatted correctly")?;
        let program = db
            .query_one(File.file_hash(i64::from(program_hash)))
            .ok_or("could not find program")?;

        let solution = db
            .query_one(Solution.problem(problem).program(program))
            .ok_or("program was never submitted for problem")?;

        #[derive(Select)]
        struct ExecutionStats {
            seed: i64,
            fuel: i64,
        }

        // list executions for this problem
        let data = db.query(|q| {
            let exec = q.join(Execution);
            q.filter(exec.instance.problem.eq(problem));
            q.filter(exec.solution.program.eq(program));
            q.into_vec(ExecutionStatsSelect {
                seed: &exec.instance.seed,
                fuel: &exec.fuel_used,
            })
        });

        let failure = db.lazy(Failure.solution(solution));

        let users: Vec<_> = db.query(|q| {
            let submission = q.join(Submission.solution(program));
            q.into_vec((&submission.timestamp, &submission.user.github_login))
                .into_iter()
                .map(|x| x.1)
                .collect()
        });

        let location = Location::Problem(
            problem_name,
            ProblemPage::Solution(program_hash.to_string()),
        );
        let res = html! {
            @if let Some(fail) = failure {
                p class="notice" {
                    "Failed for seed " (fail.seed as u64)
                    pre{(&fail.message)}
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
                            td {(solution.seed as u64)}
                            td {(solution.fuel)}
                        }
                    }
                }
            }
        };
        let res = header(location, &jar, res);
        Ok(Html(res.into_string()))
    })
    .await
}
