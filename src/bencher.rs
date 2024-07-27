use rust_query::{Just, UnixEpoch, Value};

use crate::migration::{ExecutionDummy, FailureDummy, Instance, Solution, DB, TABLES};
use crate::AppState;

struct QueuedTask<'a> {
    instance: Just<'a, Instance>,
    solution: Just<'a, Solution>,
    problem_name: String,
}

pub fn bencher_main(app: AppState) -> anyhow::Result<()> {
    loop {
        // wait for database state to change
        DB.wait();
        println!("querying the database for queue");

        let queue = DB.exec(|q| {
            let instance = q.table(&TABLES.instance);
            let solution = q.table(&TABLES.solution);
            q.filter(instance.problem().eq(solution.problem()));

            let is_executed = q.query(|q| {
                let exec = q.table(&TABLES.execution);
                q.filter_on(exec.instance(), instance);
                q.filter_on(exec.solution(), solution);
                q.exists()
            });
            // not executed yet
            q.filter(is_executed.not());

            let fail = q.query(|q| {
                let failure = q.table(&TABLES.failure);
                q.filter_on(failure.solution(), solution);
                q.exists()
            });
            // has not failed
            q.filter(fail.not());

            q.into_vec(|row| QueuedTask {
                instance: row.get(instance),
                solution: row.get(solution),
                problem_name: row.get(instance.problem().name()),
            })
        });

        println!("{} new tasks queued", queue.len());

        for task in queue {
            let solution = crate::solution::Solution {
                hash: DB.get(task.solution.program().file_hash()).into(),
            };
            let problem = &app.problem_dir.problems[&task.problem_name];

            let instance_seed = DB.get(task.instance.seed());
            let res = solution.run(problem, instance_seed);

            match res {
                Ok(fuel) => {
                    DB.try_insert(ExecutionDummy {
                        answer: None::<i64>,
                        fuel_used: fuel as i64,
                        instance: task.instance,
                        solution: task.solution,
                        timestamp: UnixEpoch,
                    })
                    .unwrap();
                }
                Err(err) => {
                    DB.try_insert(FailureDummy {
                        seed: instance_seed,
                        solution: task.solution,
                        timestamp: UnixEpoch,
                        message: err.as_str(),
                    })
                    .unwrap();
                }
            }
        }
    }
}
