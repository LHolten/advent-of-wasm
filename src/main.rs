use std::sync::Arc;

use pages::web_server;
use problem::ProblemDir;
use rand::{thread_rng, RngCore};

mod bencher;
mod chart;
mod db;
mod hash;
mod migration;
mod pages;
mod problem;
mod solution;

use migration::{InstanceDummy, ProblemDummy, DB, TABLES};
use rust_query::{UnixEpoch, Value};

#[derive(Clone)]
pub struct AppState {
    problem_dir: Arc<ProblemDir>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let problem_dir = Arc::new(ProblemDir::new()?);
    let problem_dir_clone = problem_dir.clone();
    for (problem_name, details) in &problem_dir.problems {
        // on conflict do nothing
        DB.try_insert(ProblemDummy {
            timestamp: UnixEpoch,
            name: problem_name,
        });
        let problem = DB.get(TABLES.problem.unique(problem_name)).unwrap();

        let num = DB.exec(|q| {
            let count = q.query(|q| {
                let instance = q.table(&TABLES.instance);
                q.filter(instance.problem().eq(problem));
                q.count_distinct(instance)
            });
            q.into_vec(|row| row.get(count))[0]
        });

        let mut rng = thread_rng();
        // add instances so that there are enough for the benchmark
        for _ in (0..details.leaderboard_instances).skip(num as usize) {
            let seed = rng.next_u64() as i64;

            DB.try_insert(InstanceDummy {
                problem,
                seed,
                timestamp: UnixEpoch,
            });
        }
    }

    web_server(problem_dir_clone).await
}
