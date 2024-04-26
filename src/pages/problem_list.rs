use axum::{
    extract::State,
    response::{Html, IntoResponse, Response},
};
use axum_extra::extract::CookieJar;
use maud::html;

use crate::{
    pages::{header, Location},
    AppState,
};

pub async fn get_problem_list(
    State(app): State<AppState>,
    jar: CookieJar,
    // uri: Uri,
) -> Result<Response, String> {
    let location = Location::Home;
    let res = html! {
        table {
            thead {
                tr {
                    th { "Problem Name" }
                }
            }
            tbody {
                @for problem in app.problem_dir.mapping.keys() {
                    tr {
                        td { a href={"problem/"(problem)} { (problem) } }
                    }
                }
            }
        }
    };
    let res = header(location, &jar, res);
    Ok(Html(res.into_string()).into_response())
}
