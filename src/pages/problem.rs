use std::fs;

use axum::{
    extract::{Multipart, Path, State},
    http::{StatusCode, Uri},
    response::Html,
};
use maud::html;
use rust_query::{client::QueryBuilder, value::Value};

use crate::{
    db::InsertSubmission,
    hash::{self, FileHash},
    solution::verify_wasm,
    tables, AppState, DUMMY_USER,
};

pub async fn get_problem(
    State(app): State<AppState>,
    Path(file_name): Path<String>,
    uri: Uri,
) -> Result<Html<String>, StatusCode> {
    println!("got user for {file_name}");

    let file_hash = *app
        .problem_dir
        .mapping
        .get(&file_name)
        .ok_or(StatusCode::NOT_FOUND)?;

    struct SolutionStats {
        name: String,
        average: i64,
    }

    let data = app
        .conn
        .call(move |conn| {
            // list solutions for this problem
            conn.new_query(|q| {
                let solution = q.table(tables::Solution);
                let is_submitted = q.query(|q| {
                    let submission = q.table(tables::Submission);
                    q.filter(submission.problem.file_hash.eq(i64::from(file_hash)));
                    q.group().exists()
                });
                q.filter(is_submitted);
                let average = q.query(|q| {
                    let exec = q.table(tables::Execution);
                    q.filter_on(&exec.solution, &solution);
                    q.filter(exec.instance.problem.file_hash.eq(i64::from(file_hash)));
                    q.group().max(exec.fuel_used)
                });
                q.into_vec(u32::MAX, |row| SolutionStats {
                    name: FileHash::from(row.get(solution.file_hash)).to_string(),
                    average: row.get(average).unwrap(),
                })
            })
        })
        .await;

    let res = html! {
        style { (include_str!("style.css")) }
        p.test {
            "The problem name is "
            b {(file_name)}
        }
        table {
            // caption { "Scores" }
            thead {
                tr {
                    th { "Solution" }
                    th { "Max Fuel" }
                }
            }
            tbody {
                @for solution in &data {
                    tr {
                        td {(solution.name)}
                        td {(solution.average)}
                    }
                }
            }
        }
        br;
        form action={(uri.path())"/upload"} method="post" enctype="multipart/form-data" target="_blank" {
            label { "wasm file: " }
            input type="file" name="wasm";
            br;
            input type="submit";
        }
    };
    Ok(Html(res.into_string()))
}

pub async fn upload(
    State(app): State<AppState>,
    Path(file_name): Path<String>,
    mut multipart: Multipart,
) {
    println!("got multipart");

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        let data = field.bytes().await.unwrap();
        let data_len = data.len();

        if &name == "wasm" {
            if let Err(e) = verify_wasm(&data) {
                println!("user upload error: {}", e);
                return;
            }

            let hash = hash::FileHash::new(&data);
            let path = format!("solution/{hash}.wasm");
            fs::write(path, data).unwrap();

            let submission = InsertSubmission {
                github_id: DUMMY_USER,
                file_hash: hash,
                problem_hash: app.problem_dir.mapping[&file_name],
            };

            submission.execute(&app.conn).await.unwrap();
        }

        println!("Length of `{name}` is {data_len} bytes");
    }
}
