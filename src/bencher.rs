use rust_query::{UnixEpoch, Value};

use crate::migration::{ExecutionDummy, FailureDummy, DB, TABLES};
use crate::AppState;

pub fn bencher_main(app: AppState) -> anyhow::Result<()> {
    loop {
        // wait for database state to change
        DB.wait();
        println!("querying the database for queue");

        let updated = DB.exec(|q| {
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

            q.into_vec(|row| {
                let solution_obj = crate::solution::Solution {
                    hash: row.get(solution.program().file_hash()).into(),
                };
                let problem_name = row.get(solution.problem().name());
                let problem = &app.problem_dir.problems[&problem_name];

                let instance_seed = row.get(instance.seed());
                let res = solution_obj.run(problem, instance_seed);

                match res {
                    Ok(fuel) => {
                        DB.try_insert(ExecutionDummy {
                            answer: None::<i64>,
                            fuel_used: fuel as i64,
                            instance: row.get(instance),
                            solution: row.get(solution),
                            timestamp: UnixEpoch,
                        })
                        .unwrap();
                    }
                    Err(err) => {
                        DB.try_insert(FailureDummy {
                            seed: instance_seed,
                            solution: row.get(solution),
                            timestamp: UnixEpoch,
                            message: err.as_str(),
                        })
                        .unwrap();
                    }
                }
            })
        });

        println!("updated: {}", updated.len());
    }
}
