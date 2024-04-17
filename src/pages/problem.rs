use std::fs;

use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::{Html, Redirect},
};
use axum_extra::extract::CookieJar;
use maud::{html, PreEscaped};
use rust_query::{
    client::QueryBuilder,
    value::{UnixEpoch, Value},
};

use crate::{
    chart::{Axis, Grid, Root, Series, Title, Tooltip},
    db::{get_file, get_user, GithubId},
    hash::{self, FileHash},
    pages::{header, Location, ProblemPage},
    solution::verify_wasm,
    tables::{self, FileDummy, SolutionDummy, SubmissionDummy, UserDummy},
    AppState,
};

struct SolutionStats {
    name: String,
    max_fuel: String,
    file_size: u64,
}

pub async fn get_problem(
    State(app): State<AppState>,
    Path(problem): Path<String>,
    jar: CookieJar,
    // uri: Uri,
) -> Result<Html<String>, StatusCode> {
    println!("got user for {problem}");

    let problem_hash = *app
        .problem_dir
        .mapping
        .get(&problem)
        .ok_or(StatusCode::NOT_FOUND)?;

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
                    file_size: row.get(solution.program.file_size) as u64,
                    name: FileHash::from(row.get(solution.program.file_hash)).to_string(),
                    max_fuel: if row.get(fail) {
                        "Failed".to_owned()
                    } else if row.get(count) == row.get(total_instances) {
                        row.get(max_fuel).unwrap().to_string()
                    } else {
                        format!("benched {} / {}", row.get(count), row.get(total_instances))
                    },
                })
            })
        })
        .await;

    let chart_data = graph(&data);

    let js = format!(
        "
var chart = echarts.init(document.getElementById('chart'), null, {{ renderer: 'canvas' }});
chart.setOption({});
window.addEventListener('resize', function() {{
  chart.resize();
}});
    ",
        serde_json::to_string(&chart_data).unwrap()
    );

    let location = Location::Problem(problem.clone(), ProblemPage::Home);
    let res = html! {
        (header(location, &jar))
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
                        td { a href={(problem)"/"(solution.name)} { code{(solution.name)}} }
                        td {(solution.file_size)}
                        td {(solution.max_fuel)}
                    }
                }
            }
        }

        div id="chart" style="height: 500px" {}
        script type="text/javascript" {(PreEscaped(js))}

        form method="post" enctype="multipart/form-data" {
            fieldset {
                legend { "Submit a new program" }
                aside { "Make sure to upload a " code {".wasm"} " file" }
                input type="file" name="wasm";
                button { "Submit!" };
            }
        }
    };
    Ok(Html(res.into_string()))
}

fn graph(data: &[SolutionStats]) -> Root {
    struct Data {
        file_size: u64,
        max_fuel: u64,
    }
    let data: Vec<_> = data
        .iter()
        .filter_map(|sol| {
            Some(Data {
                file_size: sol.file_size,
                max_fuel: sol.max_fuel.parse().ok()?,
            })
        })
        .collect();
    // data is sorted by file size
    let mut pareto: Vec<_> = data
        .iter()
        .enumerate()
        .filter(|(i, sol)| {
            // check that all smaller solutions are slower
            data.iter().take(*i).all(|x| x.max_fuel > sol.max_fuel)
        })
        .map(|(_i, data)| [data.file_size, data.max_fuel])
        .collect();

    const MAX: u64 = 501;
    let min_size = pareto.iter().map(|d| d[0]).min().unwrap_or(MAX);
    let min_fuel = pareto.iter().map(|d| d[1]).min().unwrap_or(MAX);
    pareto.insert(0, [min_size, MAX]);
    pareto.push([MAX, min_fuel]);

    // let chart_data = data
    //     .iter()
    //     .filter_map(|sol| Some([sol.file_size, sol.max_fuel.parse().ok()?]))
    //     .collect();

    // let pointer = AxisPointer {
    //     show: true,
    //     r#type: "cross".to_owned(),
    //     snap: true,
    //     label: Label {
    //         precision: "0".to_string(),
    //     },
    // };
    Root {
        title: Title {
            text: "Pareto Front".to_owned(),
        },
        tooltip: Tooltip {
            // axis_pointer: pointer,
            formatter: "size,fuel = {c}".to_owned(),
        },
        grid: Grid {
            contain_label: false,
        },
        x_axis: Axis {
            r#type: "log".to_owned(),
            name: "File Size".to_owned(),
            max: 500,
            min: min_size,
            // axis_pointer: pointer.clone(),
            // data: pareto.iter().map(|d| d[0]).collect(),
        },
        y_axis: Axis {
            r#type: "log".to_owned(),
            name: "Max Fuel".to_owned(),
            max: 500,
            min: min_fuel,
            // axis_pointer: pointer,
            // data: pareto.iter().map(|d| d[1]).collect(),
        },
        series: vec![
            Series::Scatter {
                data: data.iter().map(|d| [d.file_size, d.max_fuel]).collect(),
                // tooltip: Tooltip {
                //     // axis_pointer: pointer,
                //     formatter: "size,fuel = {c}".to_owned(),
                // },
            },
            Series::Line {
                step: "end".to_owned(),
                data: pareto,
                // area_style: AreaStyle {
                //     opacity: 0.2,
                //     origin: "end".to_owned(),
                // },
            },
        ],
    }
}

pub async fn upload(
    State(app): State<AppState>,
    Path(file_name): Path<String>,
    jar: CookieJar,
    multipart: Multipart,
) -> Result<Redirect, String> {
    match inner_upload(app, &file_name, jar, multipart).await {
        Ok(file) => Ok(Redirect::to(&format!("/{file_name}/{file}"))),
        Err(err) => Err(err),
    }
}

async fn inner_upload(
    app: AppState,
    file_name: &str,
    jar: CookieJar,
    mut multipart: Multipart,
) -> Result<FileHash, String> {
    println!("got multipart");

    let access_token = jar.get("access_token").ok_or("not loged in")?;
    let response = reqwest::Client::builder()
        .user_agent("wasm-bench")
        .build()
        .unwrap()
        .get("https://api.github.com/user")
        .bearer_auth(access_token.value())
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|_| "error connecting to github")?
        .error_for_status()
        .map_err(|_| "could not get github info, try loging in again")?;
    let text = response.text().await.map_err(|_| "github response error")?;
    println!("{}", text);

    let val: serde_json::Value = serde_json::from_str(&text).unwrap();
    let github_id = val.get("id").unwrap().as_i64().unwrap();
    let github_id = GithubId(github_id);
    let github_login = val.get("login").unwrap().as_str().unwrap().to_owned();

    let field = multipart.next_field().await.unwrap().unwrap();
    assert_eq!(field.name().unwrap(), "wasm");

    let data = field.bytes().await.unwrap();
    let data_len = data.len();

    println!("Got {data_len} byte wasm file");

    verify_wasm(&data)?;

    let solution_hash = hash::FileHash::new(&data);
    let path = format!("solution/{solution_hash}.wasm");
    fs::write(path, data).unwrap();

    let problem_hash = app.problem_dir.mapping[file_name];

    app.conn
        .call(move |conn| {
            conn.new_query(|q| {
                q.insert(UserDummy {
                    github_id: q.select(github_id.0),
                    github_login: q.select(github_login.as_str()),
                    timestamp: q.select(UnixEpoch),
                })
            });
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
                let user = get_user(q, github_id);
                q.insert(SubmissionDummy {
                    solution: q.select(solution),
                    timestamp: q.select(UnixEpoch),
                    user: q.select(user),
                })
            });
        })
        .await;

    Ok(solution_hash)
}
