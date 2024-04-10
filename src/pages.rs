use std::{sync::Arc, thread};

use axum::{
    routing::{get, post},
    Router,
};
use rusqlite::Connection;

use crate::{async_sqlite::SharedConnection, bencher::bencher_main, problem::ProblemDir, AppState};

use self::{
    problem::{get_problem, upload},
    submission::submission,
};

mod problem;
mod submission;

pub async fn web_server(problem_dir: Arc<ProblemDir>, conn: Connection) -> anyhow::Result<()> {
    let conn = SharedConnection::new(conn);
    let app_state = AppState { problem_dir, conn };

    // build our application with a single route
    let app = Router::new()
        .route("/problem/:problem", get(get_problem))
        .route("/problem/:problem", post(upload))
        .route("/problem/:problem/:solution_hash", get(submission))
        .with_state(app_state.clone());

    // start the bencher
    thread::spawn(|| bencher_main(app_state).unwrap());
    // run out app with hyper on localhost:3000
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await?;
    Ok(())
}
