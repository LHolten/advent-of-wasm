use std::{sync::Arc, thread};

use axum::{
    routing::{get, post},
    Router,
};
use axum_extra::extract::CookieJar;
use maud::{html, Markup, DOCTYPE};

use crate::{bencher::bencher_main, problem::ProblemDir, AppState};

use self::{
    problem::{get_problem, upload},
    submission::submission,
};

mod login;
mod problem;
mod problem_list;
mod submission;

pub async fn web_server(problem_dir: Arc<ProblemDir>) -> anyhow::Result<()> {
    let app_state = AppState { problem_dir };

    // build our application with a single route
    let app = Router::new()
        .route("/problem", get(problem_list::get_problem_list))
        .route("/problem/:problem", get(get_problem))
        .route("/problem/:problem", post(upload))
        .route("/problem/:problem/:solution_hash", get(submission))
        .route("/login", get(login::login))
        .route("/redirect", get(login::redirect))
        .with_state(app_state.clone());

    // start the bencher
    thread::spawn(|| bencher_main(app_state).unwrap());
    // run out app with hyper on localhost:3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

enum Location {
    Problem(String, ProblemPage),
    Home,
}

enum ProblemPage {
    Home,
    Solution(String),
}

fn nav(location: &Location, login: Markup) -> Markup {
    html! {
        nav {
            @if let Location::Problem(problem, page) = location {
                a href={"/problem"} { "problem" };
                @if let ProblemPage::Solution(_) = page {
                    a href={"/problem/"(problem)} { (problem) };
                }
            }
            (login)
        }
    }
}

fn title(location: &Location) -> Markup {
    html! {
        @match location {
            Location::Problem(problem, page) => {
                @match page {
                    ProblemPage::Home => {
                        h1 { "Problem " mark{(problem)} }
                    }
                    ProblemPage::Solution(solution) => {
                        h1 { "Solution " mark{(solution)} }
                    }
                }
            }
            Location::Home => {
                h1 { "Problem List" }
            }
        }
    }
}

fn header(location: Location, jar: &CookieJar, rest: Markup) -> Markup {
    let logged_in = jar.get("access_token").is_some() && jar.get("github_id").is_some();
    let login = match logged_in {
        true => html! {
            "logged in"
        },
        false => html! {
            a href="/login" { "login!" };
        },
    };

    html! {
        (DOCTYPE)
        html {
            head {
                link rel="stylesheet" href="https://cdn.simplecss.org/simple.css";
                // link rel="stylesheet" href="https://unpkg.com/chota";
                // style { (include_str!("style.css")) }

                script src="https://cdn.jsdelivr.net/npm/echarts@5.4.2/dist/echarts.js" {}
                script src="https://cdn.jsdelivr.net/npm/echarts-gl@2.0.9/dist/echarts-gl.js" {}
            }
            header {
                (nav(&location, login))
                (title(&location))
            }
            (rest)
        }
    }
}
