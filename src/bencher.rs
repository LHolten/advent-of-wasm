use rust_query::{FromDummy, Table, TableRow, UnixEpoch};

use crate::migration::{Execution, ExecutionDummy, Failure, FailureDummy, Instance, Solution};
use crate::AppState;

#[derive(FromDummy)]
struct NeedsBench<'a> {
    program_hash: i64,
    problem_name: String,
    seed: i64,
    instance: TableRow<'a, Instance>,
    solution: TableRow<'a, Solution>,
}

pub fn bencher_main(app: AppState) -> anyhow::Result<()> {
    loop {
        // wait for database state to change
        *app.watcher
            .wait_while(app.updated.lock().unwrap(), |updated| !*updated)
            .unwrap() = false;

        println!("querying the database for queue");

        app.write_transaction(|mut db| {
            let needs_bench = db.query(|q| {
                let instance = Instance::join(q);
                let solution = Solution::join(q);
                q.filter(instance.problem().eq(solution.problem()));

                let is_executed = Execution::unique(&instance, &solution).is_some();
                // not executed yet
                q.filter(is_executed.not());

                let fail = Failure::unique(&solution).is_some();
                // has not failed
                q.filter(fail.not());

                q.into_vec(NeedsBenchDummy {
                    program_hash: solution.program().file_hash(),
                    problem_name: solution.problem().name(),
                    seed: instance.seed(),
                    instance,
                    solution,
                })
            });

            for item in &needs_bench {
                let solution_obj = crate::solution::Solution {
                    hash: item.program_hash.into(),
                };
                let problem = &app.problem_dir.problems[&item.problem_name];

                let res = solution_obj.run(problem, item.seed);

                match res {
                    Ok(fuel) => {
                        db.try_insert(ExecutionDummy {
                            answer: None::<i64>,
                            fuel_used: fuel as i64,
                            instance: item.instance,
                            solution: item.solution,
                            timestamp: UnixEpoch,
                        })
                        .unwrap();
                    }
                    Err(err) => {
                        // there might already be a failure, so we can fail to insert.
                        db.try_insert(FailureDummy {
                            seed: item.seed,
                            solution: item.solution,
                            timestamp: UnixEpoch,
                            message: err.as_str(),
                        });
                    }
                }
            }

            println!("updated: {}", needs_bench.len());

            db.commit();
        });
    }
}
