use std::collections::HashMap;

use crate::problem::ProblemDir;
use rust_query::{
    migration::{schema, Config, Migrated},
    Database, LocalClient,
};

#[schema]
#[version(1..=2)]
enum Schema {
    #[unique(file_hash)]
    File {
        timestamp: i64,
        file_hash: i64,
        file_size: i64,
    },
    #[version(2..)]
    #[unique(name)]
    #[from(File)]
    Problem { timestamp: i64, name: String },
    // a problem benchmark instance
    #[unique(problem, seed)]
    Instance {
        timestamp: i64,
        seed: i64,
        #[follow]
        problem: Problem,
    },
    // a wasm solution
    // program can only be submitted to a problem once
    #[unique(program, problem)]
    Solution {
        timestamp: i64,
        // how many random tests did this solution pass
        random_tests: i64,
        program: File,
        #[follow]
        problem: Problem,
    },
    // a random test "or benchmark test" failed
    #[unique(solution)]
    Failure {
        timestamp: i64,
        solution: Solution,
        seed: i64,
        message: String,
    },
    // a user of the server
    #[unique(github_id)]
    User {
        timestamp: i64,
        github_id: i64,
        github_login: String,
    },
    // who uploaded the solution
    #[unique(solution, user)]
    Submission {
        timestamp: i64,
        solution: File,
        user: User,
    },
    // a solution applied to a problem instance results in an execution
    #[unique(instance, solution)]
    Execution {
        timestamp: i64,
        fuel_used: i64,
        // answer can be null if the solution crashed
        answer: Option<i64>,
        instance: Instance,
        solution: Solution,
    },
}

pub use v2::*;

pub fn initialize_db(client: &mut LocalClient) -> Database<Schema> {
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

    let m = client.migrator(Config::open("test.db")).unwrap();
    let m = m.migrate(|txn| {
        for (idx, old) in txn.unmigrated::<v2::Problem, v1::File!(file_hash, timestamp)>() {
            if let Some(name) = hash_to_name.remove(&old.file_hash) {
                idx.try_migrate(v2::Problem {
                    timestamp: old.timestamp,
                    name,
                })
                .expect("name should be unique");
            }
        }
        v2::update::Schema {
            problem: Migrated::map_fk_err(|| panic!("name missing for hash")),
        }
    });
    m.finish().unwrap()
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
