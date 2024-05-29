use std::sync::Arc;

use pages::web_server;
use problem::ProblemDir;
use rand::{thread_rng, RngCore};

mod async_sqlite;
mod bencher;
mod chart;
mod db;
mod hash;
mod migration;
mod pages;
mod problem;
mod solution;

use async_sqlite::DB;
use migration::{FileDummy, InstanceDummy};
use rust_query::value::{UnixEpoch, Value};

#[derive(Clone)]
pub struct AppState {
    problem_dir: Arc<ProblemDir>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let problem_dir = Arc::new(ProblemDir::new()?);
    let problem_dir_clone = problem_dir.clone();
    DB.call(move |conn| {
        for (file_hash, problem) in &problem_dir.problems {
            // let real_file_hash = problem.file_name.hash()?;
            // assert_eq!(file_hash.to_string(), real_file_hash.to_string());

            conn.new_query(|q| {
                // on conflict do nothing
                q.insert(FileDummy {
                    timestamp: UnixEpoch,
                    file_hash: i64::from(*file_hash),
                    file_size: problem.file_name.file_len().unwrap() as i64,
                })
            });

            let num = conn.new_query(|q| {
                let count = q.query(|q| {
                    let instance = q.table(&DB.instance);
                    q.filter(instance.problem.file_hash.eq(i64::from(*file_hash)));
                    q.count_distinct(instance)
                });
                q.into_vec(1, |row| row.get(count))[0]
            });

            let mut rng = thread_rng();
            // add instances so that there are enough for the benchmark
            for _ in (0..problem.leaderboard_instances).skip(num as usize) {
                let seed = rng.next_u64() as i64;

                conn.new_query(|q| {
                    let problem = db::get_file(q, *file_hash);
                    q.insert(InstanceDummy {
                        problem,
                        seed,
                        timestamp: UnixEpoch,
                    })
                });
            }
        }
    })
    .await;

    web_server(problem_dir_clone).await
}
