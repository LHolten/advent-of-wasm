use std::{fs, sync::Arc};

use axum::{
    extract::{Multipart, Path, State},
    http::Uri,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use maud::html;
use problem::ProblemDir;
use rusqlite::Connection;

mod async_sqlite;
mod hash;
mod migration;
mod problem;

use async_sqlite::SharedConnection;
use migration::initialize_db;

#[derive(Clone)]
struct AppState {
    problem_dir: Arc<ProblemDir>,
    conn: SharedConnection,
}

#[tokio::main]
async fn main() {
    let mut conn = Connection::open("test.db").unwrap();
    initialize_db(&mut conn).expect("could not initialise db");

    let problem_dir = Arc::new(ProblemDir::new().unwrap());
    for problem in problem_dir.problems.values() {
        let file_hash = problem.file_name.hash().unwrap();
        assert_eq!(file_hash.to_string(), problem.file_hash);
        conn.execute(
            r"INSERT OR IGNORE INTO problem (file_hash) VALUES ($1)",
            [&problem.file_hash],
        )
        .unwrap();
    }

    let conn = SharedConnection::new(conn);

    // build our application with a single route
    let app = Router::new()
        .route("/problem/:file_name", get(get_problem))
        .route("/problem/:file_name/upload", post(upload))
        .route("/", get(overview))
        .with_state(AppState { problem_dir, conn });

    // run it with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn get_problem(
    State(app): State<AppState>,
    Path(file_name): Path<String>,
    uri: Uri,
) -> impl IntoResponse {
    println!("got user for {file_name}");
    let data = app
        .conn
        .call(move |conn| {
            let mut prepared = conn.prepare(include_query!("problem.prql")).unwrap();
            prepared
                .query_map(&[("@hash", &*file_name)], |row| {
                    row.get::<_, String>("submission.solution")
                })
                .expect("parameters were wrong")
                .collect::<rusqlite::Result<Vec<_>>>()
                .expect("could not get problems from db")
        })
        .await;
    let res = html! {
        h1 { "Hello, world!" }
        p.intro {
            "This is an example of the "
            a href="https://github.com/lambda-fairy/maud" { "Maud" }
            " template language."
        }
        // p.test {
        //     "btw, the problem name is "
        //     b {(file_name)}
        // }
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

async fn overview(State(app): State<AppState>) -> impl IntoResponse {
    let res = html! {
        ul {
            @for (problem_name, problem) in &app.problem_dir.problems {
                li {
                    a href={"/problem/" (problem.file_hash)} {(problem_name)}
                }
            }
        }
    };
    Html(res.into_string())
}

async fn upload(State(app): State<AppState>, mut multipart: Multipart) {
    println!("got multipart");
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        let data = field.bytes().await.unwrap();
        let data_len = data.len();

        if &name == "wasm" {
            let hash = hash::Hash::new(&data);
            let path = format!("solution/{hash}.wasm");
            fs::write(path, data).unwrap();

            app.conn
                .call(move |conn| {
                    conn.execute(
                        "INSERT OR IGNORE INTO solution (file_hash) VALUES ($1)",
                        [&hash],
                    )
                    .unwrap()
                })
                .await;
        }

        println!("Length of `{name}` is {data_len} bytes");
    }
}
