use std::collections::HashMap;

use crate::problem::ProblemDir;
use rust_query::{
    migration::{schema, Config, Migrated, TransactionMigrate},
    Database,
};

#[schema(Schema)]
#[version(1..=2)]
pub mod vN {
    pub struct File {
        pub timestamp: i64,
        #[unique]
        pub file_hash: i64,
        pub file_size: i64,
    }
    #[version(2..)]
    #[from(File)]
    pub struct Problem {
        pub timestamp: i64,
        #[unique]
        pub name: String,
    }
    // a problem benchmark instance
    #[unique(problem, seed)]
    pub struct Instance {
        pub timestamp: i64,
        pub seed: i64,
        pub problem: Problem,
    }
    // a wasm solution
    // program can only be submitted to a problem once
    #[unique(program, problem)]
    pub struct Solution {
        pub timestamp: i64,
        // how many random tests did this solution pass
        pub random_tests: i64,
        pub program: File,
        pub problem: Problem,
    }
    // a random test "or benchmark test" failed
    pub struct Failure {
        pub timestamp: i64,
        #[unique]
        pub solution: Solution,
        pub seed: i64,
        pub message: String,
    }
    // a user of the server
    pub struct User {
        pub timestamp: i64,
        #[unique]
        pub github_id: i64,
        pub github_login: String,
    }
    // who uploaded the solution
    #[unique(solution, user)]
    pub struct Submission {
        pub timestamp: i64,
        pub solution: File,
        pub user: User,
    }
    // a solution applied to a problem instance results in an execution
    #[unique(instance, solution)]
    pub struct Execution {
        pub timestamp: i64,
        pub fuel_used: i64,
        // answer can be null if the solution crashed
        pub answer: Option<i64>,
        pub instance: Instance,
        pub solution: Solution,
    }
}

pub use v2::*;

pub fn initialize_db() -> Database<Schema> {
    let m = Database::migrator(Config::open("test.db")).unwrap();
    let m = m.migrate(|txn| v1::migrate::Schema {
        problem: file_to_problem(txn),
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

    txn.migrate_optional(|old: v1::File!(file_hash, timestamp)| {
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
        expect!["fe336f7b8ab2a39e"].assert_eq(&hash_schema::<v1::Schema>());
        expect!["fcc2bc960920cc33"].assert_eq(&hash_schema::<v2::Schema>());
    }
}
