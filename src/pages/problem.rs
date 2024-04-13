use std::fs;

use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::Html,
};
use maud::html;
use rust_query::{
    client::QueryBuilder,
    value::{UnixEpoch, Value},
};

use crate::{
    db::{get_file, get_user},
    hash::{self, FileHash},
    pages::{header, Location, ProblemPage},
    solution::verify_wasm,
    tables::{self, FileDummy, SolutionDummy, SubmissionDummy},
    AppState, DUMMY_USER,
};

pub async fn get_problem(
    State(app): State<AppState>,
    Path(problem): Path<String>,
    // uri: Uri,
) -> Result<Html<String>, StatusCode> {
    println!("got user for {problem}");

    let problem_hash = *app
        .problem_dir
        .mapping
        .get(&problem)
        .ok_or(StatusCode::NOT_FOUND)?;

    struct SolutionStats {
        name: String,
        max_fuel: String,
        file_size: u64,
    }

    let data = app
        .conn
        .call(move |conn| {
            // list solutions for this problem
            conn.new_query(|q| {
                let solution = q.table(tables::Solution);
                q.filter(solution.problem.file_hash.eq(i64::from(problem_hash)));
                let fail = q.query(|q| {
                    let failures = q.table(tables::Failure);
                    q.filter_on(&failures.solution, &solution);
                    q.group().exists()
                });
                let total_instances = q.query(|q| {
                    let instance = q.table(tables::Instance);
                    q.filter(instance.problem.file_hash.eq(i64::from(problem_hash)));
                    q.group().count_distinct(instance)
                });
                let (max_fuel, count) = q.query(|q| {
                    let exec = q.table(tables::Execution);
                    q.filter_on(&exec.solution, &solution);
                    q.filter(exec.instance.problem.file_hash.eq(i64::from(problem_hash)));
                    let group = &q.group();
                    (group.max(exec.fuel_used), group.count_distinct(exec))
                });
                q.into_vec(u32::MAX, |row| SolutionStats {
                    name: FileHash::from(row.get(solution.program.file_hash)).to_string(),
                    max_fuel: if row.get(fail) {
                        "Failed".to_owned()
                    } else if row.get(count) == row.get(total_instances) {
                        row.get(max_fuel).unwrap().to_string()
                    } else {
                        format!("benched {} / {}", row.get(count), row.get(total_instances))
                    },
                    file_size: row.get(solution.program.file_size) as u64,
                })
            })
        })
        .await;

    let location = Location::Problem(problem.clone(), ProblemPage::Home);
    let res = html! {
        (header(location))
        table {
            // caption { "Scores" }
            thead {
                tr {
                    th { "Solution" }
                    th { "File Size" }
                    th { "Max Fuel" }
                }
            }
            tbody {
                @for solution in &data {
                    tr {
                        td { a href={(problem)"/"(solution.name)} {(solution.name)} }
                        td {(solution.file_size)}
                        td {(solution.max_fuel)}
                    }
                }
            }
        }
        // article {
            // h2 {  }
            form method="post" enctype="multipart/form-data" {
                fieldset {
                    legend { "Submit a new program" }
                    aside { "Make sure to upload a " code {".wasm"} " file" }
                    input type="file" name="wasm";
                    button { "Submit!" };
                }
            }
        // }
    };
    Ok(Html(res.into_string()))
}

pub async fn upload(
    State(app): State<AppState>,
    Path(file_name): Path<String>,
    mut multipart: Multipart,
) -> Result<Html<String>, StatusCode> {
    println!("got multipart");

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        let data = field.bytes().await.unwrap();
        let data_len = data.len();

        println!("Length of `{name}` is {data_len} bytes");

        if &name == "wasm" {
            if let Err(e) = verify_wasm(&data) {
                println!("user upload error: {}", e);
                break;
            }

            let solution_hash = hash::FileHash::new(&data);
            let path = format!("solution/{solution_hash}.wasm");
            fs::write(path, data).unwrap();

            let problem_hash = app.problem_dir.mapping[&file_name];

            app.conn
                .call(move |conn| {
                    conn.new_query(|q| {
                        q.insert(FileDummy {
                            file_hash: q.select(i64::from(solution_hash)),
                            file_size: q.select(data_len as i64),
                            timestamp: q.select(UnixEpoch),
                        })
                    });
                    conn.new_query(|q| {
                        let problem = get_file(q, problem_hash);
                        let program = get_file(q, solution_hash);
                        q.insert(SolutionDummy {
                            timestamp: q.select(UnixEpoch),
                            program: q.select(program),
                            problem: q.select(problem),
                            random_tests: q.select(0),
                        })
                    });
                    conn.new_query(|q| {
                        let solution = get_file(q, solution_hash);
                        let user = get_user(q, DUMMY_USER);
                        q.insert(SubmissionDummy {
                            solution: q.select(solution),
                            timestamp: q.select(UnixEpoch),
                            user: q.select(user),
                        })
                    });
                })
                .await;
        }
    }

    get_problem(State(app), Path(file_name)).await
}
