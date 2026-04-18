use rust_query::{FromExpr, Select, TableRow};

use crate::hash::FileHash;
use crate::migration::{Execution, Failure, Instance, Solution};
use crate::AppState;

#[derive(Select)]
struct NeedsBench {
    program_hash: FileHash,
    problem_name: String,
    seed: i64,
    instance: TableRow<Instance>,
    solution: TableRow<Solution>,
}

pub fn bencher_main(app: AppState) -> anyhow::Result<()> {
    loop {
        // wait for database state to change
        *app.watcher
            .wait_while(app.updated.lock().unwrap(), |updated| !*updated)
            .unwrap() = false;

        println!("querying the database for queue");

        app.0.database.transaction_mut_ok(|db| {
            let needs_bench = db.query(|q| {
                let instance = &q.join(Instance);
                let solution = &q.join(Solution);
                q.filter(instance.problem.eq(&solution.problem));

                let is_executed = Execution.solution(solution).instance(instance).is_some();
                // not executed yet
                q.filter(is_executed.not());

                let fail = Failure.solution(solution).is_some();
                // has not failed
                q.filter(fail.not());

                q.into_vec(NeedsBenchSelect {
                    program_hash: FileHash::from_expr(&solution.program.file_hash),
                    problem_name: &solution.problem.name,
                    seed: &instance.seed,
                    instance,
                    solution,
                })
            });

            for item in &needs_bench {
                let solution_obj = crate::solution::Solution {
                    hash: item.program_hash,
                };
                let problem = &app.problem_dir.problems[&item.problem_name];

                let res = solution_obj.run(problem, item.seed);

                match res {
                    Ok(fuel) => {
                        db.insert(Execution {
                            answer: None,
                            fuel_used: fuel as i64,
                            instance: item.instance,
                            solution: item.solution,
                            timestamp: jiff::Timestamp::now(),
                        })
                        .unwrap();
                    }
                    Err(err) => {
                        // there might already be a failure.
                        db.find_or_insert(Failure {
                            seed: item.seed,
                            solution: item.solution,
                            timestamp: jiff::Timestamp::now(),
                            message: err,
                        });
                    }
                }
            }

            println!("updated: {}", needs_bench.len());
        });
    }
}
