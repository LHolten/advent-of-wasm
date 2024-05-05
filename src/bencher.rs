use rust_query::client::QueryBuilder;
use rust_query::value::{UnixEpoch, Value};

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
                q.exists()
            });
            // not executed yet
            q.filter(is_executed.not());

            let fail = q.query(|q| {
                let failure = q.table(tables::Failure);
                q.filter_on(&failure.solution, &solution);
                q.exists()
            });
            // has not failed
            q.filter(fail.not());

            q.into_vec(u32::MAX, |row| QueuedTask {
                solution_hash: row.get(solution.program.file_hash).into(),
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

            let res = solution.run(problem, task.instance_seed);

            let conn = app.conn.lock();

            match res {
                Ok(fuel) => {
                    conn.new_query(|q| {
                        let instance = q.table(Instance);
                        q.filter(instance.problem.file_hash.eq(i64::from(task.problem_hash)));
                        q.filter(instance.seed.eq(task.instance_seed));

                        let solution = q.table(tables::Solution);
                        q.filter(solution.program.file_hash.eq(i64::from(task.solution_hash)));
                        q.filter(solution.problem.file_hash.eq(i64::from(task.problem_hash)));

                        q.insert(ExecutionDummy {
                            answer: None::<i64>,
                            fuel_used: fuel as i64,
                            instance,
                            solution,
                            timestamp: UnixEpoch,
                        });
                    });
                }
                Err(err) => conn.new_query(|q| {
                    let solution = q.table(tables::Solution);
                    q.filter(solution.program.file_hash.eq(i64::from(task.solution_hash)));
                    q.filter(solution.problem.file_hash.eq(i64::from(task.problem_hash)));

                    q.insert(FailureDummy {
                        seed: task.instance_seed,
                        solution,
                        timestamp: UnixEpoch,
                        message: err.as_str(),
                    })
                }),
            }
        }
    }
}
