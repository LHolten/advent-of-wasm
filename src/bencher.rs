use rusqlite::ToSql;
use wasmtime::{Config, Engine};

use crate::{hash::FileHash, include_query, solution::Solution, AppState};

const BENCH_QUEUE: &str = include_query!("bench_queue.prql");
const EXECUTE: &str = include_query!("execute.prql");
struct QueuedTask {
    solution_hash: FileHash,
    problem_hash: FileHash,
    instance_seed: u64,
}

pub fn bencher_main(app: AppState) -> anyhow::Result<()> {
    let problem_engine = Engine::default();
    let solution_engine = Engine::new(&Config::new().consume_fuel(true))?;
    loop {
        app.conn.wait();
        println!("querying the database for queue");
        let conn = app.conn.lock();
        let queue = conn
            .prepare(BENCH_QUEUE)?
            .query_map([], |row| {
                Ok(QueuedTask {
                    solution_hash: row.get("solution_hash")?,
                    problem_hash: row.get("problem_hash")?,
                    instance_seed: row.get("instance_seed")?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        drop(conn);

        println!("{} new tasks queued", queue.len());

        for task in queue {
            let solution = Solution {
                hash: task.solution_hash,
            };
            let problem = &app.problem_dir.problems[&task.problem_hash];
            let instance = problem.generate(&problem_engine, task.instance_seed)?;
            let answer =
                solution.run_submission(&solution_engine, &instance.input, problem.fuel_limit)?;

            let sql =
                format!("INSERT INTO execution (fuel_used, answer, instance, solution) {EXECUTE}");
            let conn = app.conn.lock();
            conn.prepare(&sql)?.execute(&[
                ("@fuel", &0 as &dyn ToSql),
                ("@answer", &answer),
                ("instance", &task.instance_seed),
                ("solution", &task.solution_hash),
            ])?;
        }
    }
}
