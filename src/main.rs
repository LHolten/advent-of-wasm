use std::sync::Arc;

use db::GithubId;
use pages::web_server;
use problem::ProblemDir;
use rand::{thread_rng, RngCore};
use rusqlite::Connection;

mod async_sqlite;
mod bencher;
mod db;
mod hash;
mod migration;
mod pages;
mod problem;
mod solution;

pub mod tables {
    include!(concat!(env!("OUT_DIR"), "/tables.rs"));
}

use async_sqlite::SharedConnection;
use migration::initialize_db;
use rust_query::{
    client::QueryBuilder,
    value::{UnixEpoch, Value},
};
use tables::UserDummy;

use crate::tables::{InstanceDummy, ProblemDummy};

#[derive(Clone)]
pub struct AppState {
    problem_dir: Arc<ProblemDir>,
    conn: SharedConnection,
}

const DUMMY_USER: GithubId = GithubId(1337);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut conn = Connection::open("test.db")?;
    initialize_db(&mut conn).expect("could not initialise db");

    let problem_dir = Arc::new(ProblemDir::new()?);
    for (file_hash, problem) in &problem_dir.problems {
        let real_file_hash = problem.file_name.hash()?;
        assert_eq!(file_hash.to_string(), real_file_hash.to_string());

        conn.new_query(|q| {
            // on conflict do nothing
            q.insert(ProblemDummy {
                file_hash: q.select(i64::from(*file_hash)),
                timestamp: q.select(UnixEpoch),
            })
        });

        let num = conn.new_query(|q| {
            let count = q.query(|q| {
                let instance = q.table(tables::Instance);
                q.filter(instance.problem.file_hash.eq(i64::from(*file_hash)));
                q.group().count_distinct(instance)
            });
            q.into_vec(1, |row| row.get(count))[0]
        });

        let mut rng = thread_rng();
        // add instances so that there are enough for the benchmark
        for _ in (0..problem.leaderboard_instances).skip(num as usize) {
            let seed = rng.next_u64() as i64;

            conn.new_query(|q| {
                let problem = db::get_problem(q, *file_hash);
                q.insert(InstanceDummy {
                    problem: q.select(problem),
                    seed: q.select(seed),
                    timestamp: q.select(UnixEpoch),
                })
            });
        }
    }

    conn.new_query(|q| {
        q.insert(UserDummy {
            github_id: q.select(DUMMY_USER.0 as i64),
            timestamp: q.select(UnixEpoch),
        })
    });

    web_server(problem_dir, conn).await
}
