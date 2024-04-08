use std::{fs, sync::Arc, thread};

use axum::{
    extract::{Multipart, Path, State},
    http::Uri,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use bencher::bencher_main;
use db::GithubId;
use maud::html;
use problem::ProblemDir;
use rand::{thread_rng, RngCore};
use rusqlite::Connection;

mod async_sqlite;
mod bencher;
mod db;
mod hash;
mod migration;
mod problem;
mod solution;

pub mod tables {
    include!(concat!(env!("OUT_DIR"), "/tables.rs"));
}

use async_sqlite::SharedConnection;
use migration::initialize_db;
use rust_query::{
    client::QueryBuilder,
    value::{UnixEpoch, Value},
};
use tables::UserDummy;

use crate::{
    db::InsertSubmission,
    hash::FileHash,
    solution::verify_wasm,
    tables::{InstanceDummy, ProblemDummy},
};

#[derive(Clone)]
pub struct AppState {
    problem_dir: Arc<ProblemDir>,
    conn: SharedConnection,
}

const DUMMY_USER: GithubId = GithubId(1337);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut conn = Connection::open("test.db")?;
    initialize_db(&mut conn).expect("could not initialise db");

    let problem_dir = Arc::new(ProblemDir::new()?);
    for (file_hash, problem) in &problem_dir.problems {
        let real_file_hash = problem.file_name.hash()?;
        assert_eq!(file_hash.to_string(), real_file_hash.to_string());

        conn.new_query(|q| {
            // on conflict do nothing
            q.insert::<tables::Problem>(ProblemDummy {
                file_hash: q.select(i64::from(*file_hash)),
                timestamp: q.select(UnixEpoch),
            })
        });

        let num = conn.new_query(|q| {
            let instance = q.table(tables::Instance);
            q.filter(instance.problem.file_hash.eq(i64::from(*file_hash)));
            let q = q.group();
            let count = q.count_distinct(instance);
            q.into_vec(1, |row| row.get(count))[0]
        });

        let mut rng = thread_rng();
        // add instances so that there are enough for the benchmark
        for _ in (0..problem.leaderboard_instances).skip(num as usize) {
            conn.new_query(|q| {
                let problem = db::get_problem(q, *file_hash);
                q.insert::<tables::Instance>(InstanceDummy {
                    problem: q.select(problem),
                    seed: q.select(rng.next_u64() as i64),
                    timestamp: q.select(UnixEpoch),
                })
            });
        }
    }

    conn.new_query(|q| {
        q.insert::<tables::User>(UserDummy {
            github_id: q.select(DUMMY_USER.0 as i64),
            timestamp: q.select(UnixEpoch),
        })
    });

    let conn = SharedConnection::new(conn);
    let app_state = AppState { problem_dir, conn };

    // build our application with a single route
    let app = Router::new()
        .route("/problem/:file_name", get(get_problem))
        .route("/problem/:file_name/upload", post(upload))
        .with_state(app_state.clone());

    // start the bencher
    thread::spawn(|| bencher_main(app_state).unwrap());
    // run out app with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

async fn get_problem(
    State(app): State<AppState>,
    Path(file_name): Path<String>,
    uri: Uri,
) -> impl IntoResponse {
    println!("got user for {file_name}");

    let file_hash = app.problem_dir.mapping[&file_name];

    let data = app
        .conn
        .call(move |conn| {
            // list solutions for this problem
            conn.new_query(|q| {
                let solution = q.table(tables::Solution);
                let is_submitted = q.query(|q| {
                    let submission = q.table(tables::Submission);
                    q.filter(submission.problem.file_hash.eq(i64::from(file_hash)));
                    let q = q.group();
                    q.exists()
                });
                q.filter(is_submitted);
                q.into_vec(u32::MAX, |row| {
                    FileHash::from(row.get(q.select(solution.file_hash))).to_string()
                })
            })
        })
        .await;
    let res = html! {
        h1 { "Hello, world!" }
        p.intro {
            "This is an example of the "
            a href="https://github.com/lambda-fairy/maud" { "Maud" }
            " template language."
        }
        p.test {
            "btw, the problem name is "
            b {(file_name)}
        }
        ul {
            @for solution in &data {
                li {
                    {(solution)}
                }
            }
        }
        form action={(uri.path())"/upload"} method="post" enctype="multipart/form-data" {
            label { "wasm file" }
            br;
            input type="file" name="wasm";
            br;
            input type="submit";
        }
    };
    Html(res.into_string())
}

async fn upload(
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
