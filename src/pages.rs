use std::{sync::Arc, thread};

use axum::{
    routing::{get, post},
    Router,
};
use maud::{html, Markup};
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

enum Location {
    Problem(String, ProblemPage),
}

enum ProblemPage {
    Home,
    Solution(String),
}

fn header(location: Location) -> Markup {
    let Location::Problem(problem, page) = location;
    html! {
        head {
            link rel="stylesheet" href="https://cdn.simplecss.org/simple.css";
            // link rel="stylesheet" href="https://unpkg.com/chota";
            // style { (include_str!("style.css")) }

            script src="https://cdn.jsdelivr.net/npm/echarts@5.4.2/dist/echarts.js" {}
            script src="https://cdn.jsdelivr.net/npm/echarts-gl@2.0.9/dist/echarts-gl.js" {}
        }
        header {
            @match page {
                ProblemPage::Home => {
                    h1 { "Problem " mark{(problem)} }
                }
                ProblemPage::Solution(solution) => {
                    nav {
                        a href={"/problem/"(problem)} { (problem) };
                    }
                    h1 { "Solution " mark{(solution)} }
                }
            }
        }
    }
}
