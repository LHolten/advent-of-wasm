use rust_query::client::QueryBuilder;
use rust_query::value::{UnixEpoch, Value};
use wasmtime::{Config, Engine};

use crate::tables::{Execution, ExecutionDummy, Instance};
use crate::{
    hash::FileHash,
    solution::Solution,
    tables::{self, Submission},
    AppState,
};

struct QueuedTask {
    solution_hash: FileHash,
    problem_hash: FileHash,
    instance_seed: i64,
}

pub fn bencher_main(app: AppState) -> anyhow::Result<()> {
    let problem_engine = Engine::default();
    let solution_engine = Engine::new(Config::new().consume_fuel(true))?;
    loop {
        // wait for database state to change
        app.conn.wait();
        println!("querying the database for queue");
        let conn = app.conn.lock();

        let queue = conn.new_query(|q| {
            let instance = q.table(tables::Instance);
            let solution = q.table(tables::Solution);
            let is_executed = q.query(|q| {
                let exec = q.table(tables::Execution);
                let mut q = q.group();
                q.project_eq(&exec.instance, &instance);
                q.project_eq(&exec.solution, &solution);
                q.exists()
            });
            // not executed yet
            q.filter(is_executed.not());

            let is_submitted = q.query(|q| {
                let submission = q.table(Submission);
                let mut q = q.group();
                q.project_eq(&submission.problem, &instance.problem);
                q.project_eq(&submission.solution, &solution);
                q.exists()
            });
            // is submitted
            q.filter(is_submitted);

            q.into_vec(u32::MAX, |row| QueuedTask {
                solution_hash: row.get(q.select(solution.file_hash)).into(),
                problem_hash: row.get(q.select(instance.problem.file_hash)).into(),
                instance_seed: row.get(q.select(instance.seed)),
            })
        });

        drop(conn);

        println!("{} new tasks queued", queue.len());

        for task in queue {
            let solution = Solution {
                hash: task.solution_hash,
            };
            let problem = &app.problem_dir.problems[&task.problem_hash];
            let instance = problem.generate(&problem_engine, task.instance_seed)?;

            let run_result = solution.run(&solution_engine, &instance.input, problem.fuel_limit);

            let conn = app.conn.lock();

            conn.new_query(|q| {
                let instance = q.table(Instance);
                q.filter(instance.problem.file_hash.eq(i64::from(task.problem_hash)));
                q.filter(instance.seed.eq(task.instance_seed));
                let solution = q.table(tables::Solution);
                q.filter(solution.file_hash.eq(i64::from(task.solution_hash)));

                q.insert::<Execution>(ExecutionDummy {
                    answer: q.select(&run_result.answer),
                    fuel_used: q.select(run_result.fuel_used as i64),
                    instance: q.select(instance),
                    solution: q.select(solution),
                    timestamp: q.select(UnixEpoch),
                });
            });
        }
    }
}
