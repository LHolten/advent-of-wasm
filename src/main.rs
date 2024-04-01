#![feature(unboxed_closures)]
#![feature(closure_lifetime_binder)]
// #![feature(impl_trait_in_fn_trait_return)]
// use std::{fs, sync::Arc, thread};

// use axum::{
//     extract::{Multipart, Path, State},
//     http::Uri,
//     response::{Html, IntoResponse},
//     routing::{get, post},
//     Router,
// };
// use bencher::bencher_main;
// use db::GithubId;
// use maud::html;
// use problem::ProblemDir;
// use rand::{thread_rng, RngCore};

// mod async_sqlite;
// mod bencher;
// mod db;
// mod hash;
// mod migration;
// mod problem;
// mod solution;

mod orm;
mod queries;
// // #[allow(warnings, unused)]
// // mod prisma;

// use async_sqlite::SharedConnection;
// use migration::initialize_db;

// use crate::{db::InsertSubmission, solution::verify_wasm};

// #[derive(Clone)]
// pub struct AppState {
//     problem_dir: Arc<ProblemDir>,
//     conn: SharedConnection,
// }

// const DUMMY_USER: GithubId = GithubId(1337);

pub fn main() {}

// #[tokio::main]
// async fn main() -> anyhow::Result<()> {
//     let mut conn = Connection::open("test.db")?;
//     initialize_db(&mut conn).expect("could not initialise db");

//     let problem_dir = Arc::new(ProblemDir::new()?);
//     for (file_hash, problem) in &problem_dir.problems {
//         let real_file_hash = problem.file_name.hash()?;
//         assert_eq!(file_hash.to_string(), real_file_hash.to_string());

//         conn.execute(
//             r"INSERT OR IGNORE INTO problem (file_hash) VALUES ($1)",
//             [&file_hash],
//         )?;

//         let num: u32 = conn.query_row(
//             include_query!("bench_list.prql"),
//             &[("@problem_hash", &file_hash)],
//             |row| row.get("count"),
//         )?;
//         let mut rng = thread_rng();
//         for _ in (0..problem.leaderboard_instances).skip(num as usize) {
//             let sql = format!(
//                 "INSERT INTO instance (problem, seed) {}",
//                 include_query!("instance.prql")
//             );
//             conn.execute(
//                 &sql,
//                 &[
//                     ("@problem_hash", &file_hash as &dyn ToSql),
//                     ("@seed", &(rng.next_u64() as i64)),
//                 ],
//             )?;
//         }
//     }
//     conn.execute(
//         "INSERT OR IGNORE INTO user (github_id) VALUES ($1)",
//         [&DUMMY_USER],
//     )?;

//     let conn = SharedConnection::new(conn);
//     let app_state = AppState { problem_dir, conn };

//     // build our application with a single route
//     let app = Router::new()
//         .route("/problem/:file_name", get(get_problem))
//         .route("/problem/:file_name/upload", post(upload))
//         .with_state(app_state.clone());

//     // start the bencher
//     thread::spawn(|| bencher_main(app_state).unwrap());
//     // run out app with hyper on localhost:3000
//     axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
//         .serve(app.into_make_service())
//         .await?;
//     Ok(())
// }

// async fn get_problem(
//     State(app): State<AppState>,
//     Path(file_name): Path<String>,
//     uri: Uri,
// ) -> impl IntoResponse {
//     println!("got user for {file_name}");
//     let data = app
//         .conn
//         .call(move |conn| {
//             let mut prepared = conn.prepare(include_query!("problem.prql")).unwrap();
//             prepared
//                 .query_map(&[("@hash", &*file_name)], |row| {
//                     row.get::<_, String>("submission.solution")
//                 })
//                 .expect("parameters were wrong")
//                 .collect::<rusqlite::Result<Vec<_>>>()
//                 .expect("could not get problems from db")
//         })
//         .await;
//     let res = html! {
//         h1 { "Hello, world!" }
//         p.intro {
//             "This is an example of the "
//             a href="https://github.com/lambda-fairy/maud" { "Maud" }
//             " template language."
//         }
//         // p.test {
//         //     "btw, the problem name is "
//         //     b {(file_name)}
//         // }
//         ul {
//             @for solution in &data {
//                 li {
//                     {(solution)}
//                 }
//             }
//         }
//         form action={(uri.path())"/upload"} method="post" enctype="multipart/form-data" {
//             label { "wasm file" }
//             br;
//             input type="file" name="wasm";
//             br;
//             input type="submit";
//         }
//     };
//     Html(res.into_string())
// }

// async fn upload(
//     State(app): State<AppState>,
//     Path(file_name): Path<String>,
//     mut multipart: Multipart,
// ) {
//     println!("got multipart");

//     while let Some(field) = multipart.next_field().await.unwrap() {
//         let name = field.name().unwrap().to_string();
//         let data = field.bytes().await.unwrap();
//         let data_len = data.len();

//         if &name == "wasm" {
//             if let Err(e) = verify_wasm(&data) {
//                 println!("user upload error: {}", e);
//                 return;
//             }

//             let hash = hash::FileHash::new(&data);
//             let path = format!("solution/{hash}.wasm");
//             fs::write(path, data).unwrap();

//             let submission = InsertSubmission {
//                 github_id: DUMMY_USER,
//                 file_hash: hash,
//                 problem_hash: app.problem_dir.mapping[&file_name],
//             };

//             submission.execute(&app.conn).await.unwrap();
//         }

//         println!("Length of `{name}` is {data_len} bytes");
//     }
// }
