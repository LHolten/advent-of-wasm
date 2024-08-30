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

use migration::{initialize_db, Instance, InstanceDummy, Problem, ProblemDummy, Schema};
use rust_query::{Database, ReadTransaction, ThreadToken, UnixEpoch, Value, WriteTransaction};

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
    pub fn write_transaction<F, R>(&self, f: F) -> R
    where
        F: FnOnce(WriteTransaction<'_, Schema>) -> R,
    {
        tokio::task::block_in_place(|| {
            let mut token = ThreadToken::try_new().unwrap();
            let transaction = self.database.write_lock(&mut token);
            f(transaction)
        })
    }

    pub fn read_transaction<F, R>(&self, f: F) -> R
    where
        F: FnOnce(ReadTransaction<'_, Schema>) -> R,
    {
        tokio::task::block_in_place(|| {
            let mut token = ThreadToken::try_new().unwrap();
            let transaction = self.database.read(&mut token);
            f(transaction)
        })
    }

    pub fn request_bench(&self) {
        *self.updated.lock().unwrap() = true;
        self.watcher.notify_all();
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let problem_dir = ProblemDir::new()?;

    let mut token = ThreadToken::try_new().unwrap();
    let database = initialize_db(&mut token);
    let mut db = database.write_lock(&mut token);

    for (problem_name, details) in &problem_dir.problems {
        // on conflict do nothing
        db.try_insert(ProblemDummy {
            timestamp: UnixEpoch,
            name: problem_name.as_str(),
        });
        let problem = db.get(Problem::unique(problem_name.as_str())).unwrap();

        let num = db.exec(|q| {
            let count = q.aggregate(|q| {
                let instance = Instance::join(q);
                q.filter(instance.problem().eq(problem));
                q.count_distinct(instance)
            });
            q.into_vec(count)[0]
        });

        let mut rng = thread_rng();
        // add instances so that there are enough for the benchmark
        for _ in (0..details.leaderboard_instances).skip(num as usize) {
            let seed = rng.next_u64() as i64;

            db.try_insert(InstanceDummy {
                problem,
                seed,
                timestamp: UnixEpoch,
            });
        }
    }
    db.commit();
    drop(token);

    let app_state = AppStateInner {
        problem_dir,
        database: database,
        updated: Mutex::new(true),
        watcher: Condvar::new(),
    };

    web_server(AppState(Arc::new(app_state))).await
}
