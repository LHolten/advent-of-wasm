use std::fs;

use axum::{
    extract::{Multipart, Path, Query},
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_extra::extract::CookieJar;
use maud::{html, PreEscaped};
use rust_query::{
    ops::{Assume, Col},
    Covariant, Db, UnixEpoch, Value,
};
use serde::Deserialize;

use crate::{
    chart::{Axis, Grid, Root, Series, Title, Tooltip},
    db,
    hash::{self, FileHash},
    migration::{FileDummy, Problem, Solution, SolutionDummy, SubmissionDummy, DB, TABLES},
    pages::{
        header,
        login::{fast_login, safe_login},
        Location, ProblemPage,
    },
    solution::verify_wasm,
};

#[derive(Deserialize)]
pub struct SolutionQuery {
    score: Option<String>,
}

#[derive(Clone)]
struct SolutionStats {
    name: String,
    max_fuel: u64,
    file_size: u64,
    yours: bool,
}

pub async fn get_problem(
    Path(problem_name): Path<String>,
    jar: CookieJar,
    Query(query): Query<SolutionQuery>,
) -> Result<Response, String> {
    println!("got user for {problem_name}");

    let problem = db::get_problem(&problem_name)?;

    if let Some(score) = query.score {
        let (size, fuel) = score.split_once(',').ok_or("expected two part score")?;
        let size: u64 = size.parse().map_err(|_| "could not parse size")?;
        let fuel: u64 = fuel.parse().map_err(|_| "could not parse fuel")?;

        let hashes = DB.exec(|q| {
            let sfp = solutions_for_problem(q, problem);
            q.filter(sfp.solution.program().file_size().eq(size as i64));
            q.filter(sfp.max_fuel.eq(fuel as i64));
            q.into_vec(|row| FileHash::from(row.get(sfp.solution.program().file_hash())))
        });
        if hashes.len() == 1 {
            let target = format!("{problem_name}/{}", &hashes[0]);
            return Ok(Redirect::to(&target).into_response());
        } else {
            return Err("there are multiple solutions with that score".to_owned());
        }
    }

    let github_id = fast_login(&jar).await;

    let data = DB.exec(|q| {
        let sfp = solutions_for_problem(q, problem);
        let yours = q.query(|q| {
            let subm = q.table(&TABLES.submission);
            q.filter_on(subm.solution(), sfp.solution.program());
            if let Some(github_id) = github_id {
                q.filter(subm.user().github_id().eq(github_id.0));
            } else {
                q.filter(false);
            }
            q.exists()
        });

        q.into_vec(|row| SolutionStats {
            file_size: row.get(sfp.solution.program().file_size()) as u64,
            name: FileHash::from(row.get(sfp.solution.program().file_hash())).to_string(),
            max_fuel: row.get(sfp.max_fuel) as u64,
            yours: row.get(yours),
        })
    });

    let chart_data = graph(&data);

    let js = format!(
        "
var chart = echarts.init(document.getElementById('chart'), null, {{ renderer: 'canvas' }});
chart.setOption({});
chart.on('click', 'series', function(params) {{
    window.location.href = '?score=' + params.value;
}});
window.addEventListener('resize', function() {{
  chart.resize();
}});
    ",
        serde_json::to_string(&chart_data).unwrap()
    );

    let location = Location::Problem(problem_name.clone(), ProblemPage::Home);
    let res = html! {
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
                        td { a href={(problem_name)"/"(solution.name)} { code{(solution.name)}} }
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
    let res = header(location, &jar, res);
    Ok(Html(res.into_string()).into_response())
}

struct SolutionForProblem<'a> {
    solution: Col<Solution, Db<'a, Solution>>,
    max_fuel: Assume<Col<Option<i64>, Db<'a, Option<i64>>>>,
}

fn solutions_for_problem<'a>(
    q: &mut rust_query::Query<'a>,
    problem: impl Covariant<'a, Typ = Problem> + Copy,
) -> SolutionForProblem<'a> {
    let solution = q.table(&TABLES.solution);
    q.filter(solution.problem().eq(problem));
    let fail = TABLES.failure.unique(solution).is_not_null();
    q.filter(fail.not());
    let total_instances = q.query(|q| {
        let instance = q.table(&TABLES.instance);
        q.filter(instance.problem().eq(problem.weaken()));
        q.count_distinct(instance)
    });
    let (max_fuel, count) = q.query(|q| {
        let exec = q.table(&TABLES.execution);
        q.filter_on(exec.solution(), solution);
        q.filter(exec.instance().problem().eq(problem.weaken()));
        (q.max(exec.fuel_used()), q.count_distinct(exec))
    });
    q.filter(count.eq(total_instances));
    let max_fuel = q.filter_some(max_fuel);
    SolutionForProblem { solution, max_fuel }
}

fn pareto(data: &[SolutionStats]) -> Vec<[u64; 2]> {
    // data is sorted by file size
    let mut tmp: Vec<_> = data
        .iter()
        .enumerate()
        .filter(|(i, sol)| {
            // check that all smaller solutions are slower
            data.iter().take(*i).all(|x| x.max_fuel > sol.max_fuel)
        })
        .map(|(_i, data)| [data.file_size, data.max_fuel])
        .collect();
    const MAX: u64 = 513;
    let min_size = tmp.iter().map(|d| d[0]).min().unwrap_or(MAX);
    let min_fuel = tmp.iter().map(|d| d[1]).min().unwrap_or(MAX);
    tmp.insert(0, [min_size, MAX]);
    tmp.push([MAX, min_fuel]);
    tmp
}

fn graph(data: &[SolutionStats]) -> Root {
    let your_data: Vec<_> = data.iter().filter(|d| d.yours).cloned().collect();
    let your_pareto = pareto(&your_data);
    let pareto = pareto(data);

    Root {
        title: Title {
            text: "Pareto Front".to_owned(),
        },
        tooltip: Tooltip {
            formatter: "size,fuel = {c}".to_owned(),
        },
        grid: Grid {
            contain_label: false,
        },
        x_axis: Axis {
            r#type: "log".to_owned(),
            name: "File Size".to_owned(),
            max: 512,
            log_base: 2,
        },
        y_axis: Axis {
            r#type: "log".to_owned(),
            name: "Max Fuel".to_owned(),
            max: 512,
            log_base: 2,
        },
        series: vec![
            Series::Scatter {
                data: your_data
                    .iter()
                    .map(|d| [d.file_size, d.max_fuel])
                    .collect(),
            },
            Series::Scatter {
                data: data
                    .iter()
                    .filter(|d| !d.yours)
                    .map(|d| [d.file_size, d.max_fuel])
                    .collect(),
            },
            Series::Line {
                step: "end".to_owned(),
                data: your_pareto,
            },
            Series::Line {
                step: "end".to_owned(),
                data: pareto,
            },
        ],
    }
}

pub async fn upload(
    Path(problem_name): Path<String>,
    mut jar: CookieJar,
    mut multipart: Multipart,
) -> Result<Redirect, String> {
    let problem = db::get_problem(&problem_name)?;

    println!("got multipart");

    let github_id = safe_login(&mut jar).await?;

    let Some(field) = multipart.next_field().await.map_err(|e| e.to_string())? else {
        return Err("expected multipart field".to_owned());
    };
    if field.name() != Some("wasm") {
        return Err("multipart field name is not `wasm`".to_owned());
    }

    let data = field.bytes().await.map_err(|e| e.to_string())?;
    let data_len = data.len();

    println!("Got {data_len} byte wasm file");

    verify_wasm(&data)?;

    let solution_hash = hash::FileHash::new(&data);
    let path = format!("solution/{solution_hash}.wasm");
    fs::write(path, data).unwrap();

    DB.try_insert(FileDummy {
        file_hash: i64::from(solution_hash),
        file_size: data_len as i64,
        timestamp: UnixEpoch,
    });
    let program = DB
        .get(TABLES.file.unique(i64::from(solution_hash)))
        .unwrap();

    DB.try_insert(SolutionDummy {
        program,
        problem,
        random_tests: 0,
        timestamp: UnixEpoch,
    });

    let user = DB.get(TABLES.user.unique(github_id.0)).unwrap();
    DB.try_insert(SubmissionDummy {
        solution: program,
        user,
        timestamp: UnixEpoch,
    });

    Ok(Redirect::to(&format!(
        "/problem/{problem_name}/{solution_hash}"
    )))
}

pub async fn get_template(Path(problem_name): Path<String>) -> Result<String, String> {
    // Check that this is a problem name.
    // Otherwise we need to sanitize it.
    db::get_problem(&problem_name)?;

    fs::read_to_string(format!("template/{problem_name}.wat")).map_err(|_| {
        format!(
            ";; No template available yet for problem {problem_name}.\n;; Delete this text and refresh to try again."
        )
    })
}
