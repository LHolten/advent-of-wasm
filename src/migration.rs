use std::collections::HashMap;

use crate::problem::ProblemDir;
use rust_query::{
    migration::{schema, Config, Migrated, TransactionMigrate},
    Database, Lazy,
};

#[schema(Schema)]
#[version(1..=4)]
pub mod vN {
    use jiff::Timestamp;
    use rust_query::TableRow;

    pub struct File {
        #[version[..4]]
        pub timestamp: i64,
        #[version[4..]]
        pub timestamp: Timestamp,
        #[unique]
        pub file_hash: i64,
        pub file_size: i64,
    }
    #[version(2..)]
    #[from(File)]
    pub struct Problem {
        #[version[..4]]
        pub timestamp: i64,
        #[version[4..]]
        pub timestamp: Timestamp,
        #[unique]
        pub name: String,
    }
    // a problem benchmark instance
    #[unique(problem, seed)]
    pub struct Instance {
        #[version[..4]]
        pub timestamp: i64,
        #[version[4..]]
        pub timestamp: Timestamp,
        pub seed: i64,
        pub problem: TableRow<Problem>,
    }
    // a wasm solution
    // program can only be submitted to a problem once
    #[unique(problem, program)]
    pub struct Solution {
        #[version[..4]]
        pub timestamp: i64,
        #[version[4..]]
        pub timestamp: Timestamp,
        // how many random tests did this solution pass
        pub random_tests: i64,
        pub program: TableRow<File>,
        pub problem: TableRow<Problem>,
    }
    // a random test "or benchmark test" failed
    pub struct Failure {
        #[version[..4]]
        pub timestamp: i64,
        #[version[4..]]
        pub timestamp: Timestamp,
        #[unique]
        pub solution: TableRow<Solution>,
        pub seed: i64,
        pub message: String,
    }
    // a user of the server
    pub struct User {
        #[version[..4]]
        pub timestamp: i64,
        #[version[4..]]
        pub timestamp: Timestamp,
        #[unique]
        pub github_id: i64,
        pub github_login: String,
    }
    // who uploaded the solution
    #[unique(solution, user)]
    pub struct Submission {
        #[version[..4]]
        pub timestamp: i64,
        #[version[4..]]
        pub timestamp: Timestamp,
        pub solution: TableRow<File>,
        pub user: TableRow<User>,
    }
    // a solution applied to a problem instance results in an execution
    #[unique(solution, instance)]
    pub struct Execution {
        #[version[..4]]
        pub timestamp: i64,
        #[version[4..]]
        pub timestamp: Timestamp,
        pub fuel_used: i64,
        // answer can be null if the solution crashed
        pub answer: Option<i64>,
        pub instance: TableRow<Instance>,
        pub solution: TableRow<Solution>,
    }
}

pub use v4::*;

pub fn initialize_db() -> Database<Schema> {
    let m = Database::migrator(Config::open("test.db")).unwrap();
    let m = m
        .migrate(|txn| v1::migrate::Schema {
            problem: file_to_problem(txn),
        })
        .migrate(|_txn| v2::migrate::Schema {})
        .migrate(|txn| v3::migrate::Schema {
            file: txn.migrate_ok(|row: Lazy<v3::File>| v3::migrate::File {
                timestamp: jiff::Timestamp::from_second(row.timestamp).unwrap(),
            }),
            problem: txn.migrate_ok(|row: Lazy<v3::Problem>| v3::migrate::Problem {
                timestamp: jiff::Timestamp::from_second(row.timestamp).unwrap(),
            }),
            instance: txn.migrate_ok(|row: Lazy<v3::Instance>| v3::migrate::Instance {
                timestamp: jiff::Timestamp::from_second(row.timestamp).unwrap(),
            }),
            solution: txn.migrate_ok(|row: Lazy<v3::Solution>| v3::migrate::Solution {
                timestamp: jiff::Timestamp::from_second(row.timestamp).unwrap(),
            }),
            failure: txn.migrate_ok(|row: Lazy<v3::Failure>| v3::migrate::Failure {
                timestamp: jiff::Timestamp::from_second(row.timestamp).unwrap(),
            }),
            user: txn.migrate_ok(|row: Lazy<v3::User>| v3::migrate::User {
                timestamp: jiff::Timestamp::from_second(row.timestamp).unwrap(),
            }),
            submission: txn.migrate_ok(|row: Lazy<v3::Submission>| v3::migrate::Submission {
                timestamp: jiff::Timestamp::from_second(row.timestamp).unwrap(),
            }),
            execution: txn.migrate_ok(|row: Lazy<v3::Execution>| v3::migrate::Execution {
                timestamp: jiff::Timestamp::from_second(row.timestamp).unwrap(),
            }),
        });
    m.finish().unwrap()
}

fn file_to_problem<'t>(
    txn: &mut TransactionMigrate<v1::Schema>,
) -> Migrated<'t, v1::Schema, v2::Problem> {
    let problem_dir = ProblemDir::new().unwrap();
    let mut hash_to_name: HashMap<i64, String> = problem_dir
        .problems
        .into_iter()
        .filter_map(|(name, problem)| {
            problem
                .original_file_hash
                .map(|hash| (i64::from(hash), name))
        })
        .collect();

    txn.migrate_optional(|old: Lazy<v1::File>| {
        let name = hash_to_name.remove(&old.file_hash)?;
        Some(v1::migrate::Problem {
            timestamp: old.timestamp,
            name,
        })
    })
    .expect("name should be unique");

    Migrated::map_fk_err(|| panic!("name missing for hash"))
}

// Test that migrations are working
#[cfg(test)]
mod tests {
    use expect_test::expect;
    use rust_query::migration::hash_schema;

    use super::*;

    #[test]
    fn migrations_test() {
        expect!["79beac10ee351d58"].assert_eq(&hash_schema::<v1::Schema>());
        expect!["4c48677ee05dc15a"].assert_eq(&hash_schema::<v2::Schema>());
        expect!["4c48677ee05dc15a"].assert_eq(&hash_schema::<v3::Schema>());
        expect!["86db366b1e4c58d7"].assert_eq(&hash_schema::<v4::Schema>());
    }
}
