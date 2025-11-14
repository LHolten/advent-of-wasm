use std::{
    ops::Deref,
    sync::{Arc, Condvar, Mutex},
};

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

use migration::{initialize_db, Instance, Problem, Schema};
use rust_query::{aggregate, Database, Transaction, UnixEpoch};

#[derive(Clone)]
pub struct AppState(Arc<AppStateInner>);

impl Deref for AppState {
    type Target = AppStateInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct AppStateInner {
    problem_dir: ProblemDir,
    database: Database<Schema>,
    watcher: Condvar,
    updated: Mutex<bool>,
}

impl AppState {
    /// Don't forget to commit!
    pub fn write_transaction<F, R: Send>(&self, f: F) -> R
    where
        F: Send + FnOnce(&'static mut Transaction<Schema>) -> R,
    {
        tokio::task::block_in_place(|| self.database.transaction_mut_ok(f))
    }

    pub fn read_transaction<F, R: Send>(&self, f: F) -> R
    where
        F: Send + FnOnce(&'static Transaction<Schema>) -> R,
    {
        tokio::task::block_in_place(|| self.database.transaction(f))
    }

    pub fn request_bench(&self) {
        *self.updated.lock().unwrap() = true;
        self.watcher.notify_all();
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let problem_dir = ProblemDir::new()?;

    let database = initialize_db();

    database.transaction_mut_ok(|db| {
        for (problem_name, details) in &problem_dir.problems {
            let problem = db.find_or_insert(Problem {
                timestamp: UnixEpoch,
                name: problem_name.as_str(),
            });

            let num = db.query_one(aggregate(|q| {
                let instance = q.join(Instance);
                q.filter(instance.problem.eq(&problem));
                q.count_distinct(instance)
            }));

            let mut rng = thread_rng();
            // add instances so that there are enough for the benchmark
            for _ in (0..details.leaderboard_instances).skip(num as usize) {
                let seed = rng.next_u64() as i64;

                db.find_or_insert(Instance {
                    problem: &problem,
                    seed,
                    timestamp: UnixEpoch,
                });
            }
        }
    });

    let app_state = AppStateInner {
        problem_dir,
        database,
        updated: Mutex::new(true),
        watcher: Condvar::new(),
    };

    web_server(AppState(Arc::new(app_state))).await
}
