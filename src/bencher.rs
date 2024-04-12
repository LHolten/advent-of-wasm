use rust_query::client::QueryBuilder;
use rust_query::value::{UnixEpoch, Value};
use wasmtime::{Config, Engine};

use crate::tables::{ExecutionDummy, FailureDummy, Instance};
use crate::{
    hash::FileHash,
    solution::Solution,
    tables::{self},
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
            q.filter((&instance.problem).eq(&solution.problem));

            let is_executed = q.query(|q| {
                let exec = q.table(tables::Execution);
                q.filter_on(&exec.instance, &instance);
                q.filter_on(&exec.solution, &solution);
                q.group().exists()
            });
            // not executed yet
            q.filter(is_executed.not());

            let fail = q.query(|q| {
                let failure = q.table(tables::Failure);
                q.filter_on(&failure.solution, &solution);
                q.group().exists()
            });
            // has not failed
            q.filter(fail.not());

            q.into_vec(u32::MAX, |row| QueuedTask {
                solution_hash: row.get(solution.file_hash).into(),
                problem_hash: row.get(instance.problem.file_hash).into(),
                instance_seed: row.get(instance.seed),
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

            if run_result.answer == Some(instance.answer) {
                conn.new_query(|q| {
                    let instance = q.table(Instance);
                    q.filter(instance.problem.file_hash.eq(i64::from(task.problem_hash)));
                    q.filter(instance.seed.eq(task.instance_seed));
                    let solution = q.table(tables::Solution);
                    q.filter(solution.file_hash.eq(i64::from(task.solution_hash)));

                    q.insert(ExecutionDummy {
                        answer: q.select(&run_result.answer),
                        fuel_used: q.select(run_result.fuel_used as i64),
                        instance: q.select(instance),
                        solution: q.select(solution),
                        timestamp: q.select(UnixEpoch),
                    });
                });
            } else {
                conn.new_query(|q| {
                    let solution = q.table(tables::Solution);
                    q.filter(solution.file_hash.eq(i64::from(task.solution_hash)));

                    q.insert(FailureDummy {
                        seed: q.select(task.instance_seed),
                        solution: q.select(solution),
                        timestamp: q.select(UnixEpoch),
                    })
                })
            }
        }
    }
}
