use std::sync::LazyLock;

use crate::problem::ProblemDir;
use rust_query::{schema, Client, SharedClient};

#[schema]
#[version(1..4)]
enum Schema {
    #[unique(file_hash)]
    File {
        timestamp: i64,
        file_hash: i64,
        file_size: i64,
    },
    #[version(2..)]
    #[unique(name)]
    #[create_from(File)]
    Problem {
        timestamp: i64,
        name: String,
        #[unique_original]
        #[version(..3)]
        original: File,
    },
    // a problem benchmark instance
    #[unique(problem, seed)]
    Instance {
        timestamp: i64,
        #[version(..3)]
        problem: File,
        #[version(3..)]
        problem: Problem,
        seed: i64,
    },
    // a wasm solution
    // program can only be submitted to a problem once
    #[unique(program, problem)]
    Solution {
        timestamp: i64,
        program: File,
        #[version(..3)]
        problem: File,
        #[version(3..)]
        problem: Problem,
        // how many random tests did this solution pass
        random_tests: i64,
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

pub use v3::*;

pub fn initialize_db() -> (Client, Schema) {
    let problem_dir = ProblemDir::new().unwrap();

    let prepare = rust_query::Prepare::open("test.db");
    // TODO: add trait constraints to migration types
    let (mut m, s) = prepare.create_db_empty();
    let s = m.migrate(s, |_s, c| v2::up::Schema {
        problem: Box::new(|file| {
            let hash = c.get(file.file_hash()).into();
            let problem = problem_dir
                .problems
                .iter()
                .find(|x| x.1.original_file_hash == Some(hash));

            problem.map(|(problem_name, _)| v2::up::ProblemMigration {
                name: problem_name,
                timestamp: file.timestamp(),
                original: file,
            })
        }),
    });
    let s = m.migrate(s, |s, c| v3::up::Schema {
        problem: Box::new(|_problem| v3::up::ProblemMigration {}),
        instance: Box::new(|instance| v3::up::InstanceMigration {
            problem: c
                .get(s.problem.unique_original(instance.problem()))
                .unwrap(),
        }),
        solution: Box::new(|solution| v3::up::SolutionMigration {
            problem: c
                .get(s.problem.unique_original(solution.problem()))
                .unwrap(),
        }),
    });
    (m.finish(), s.unwrap())
}

static BOTH: LazyLock<(SharedClient, Schema)> = LazyLock::new(|| {
    let (client, schema) = initialize_db();
    (SharedClient::new(client), schema)
});

pub static DB: LazyLock<&SharedClient> = LazyLock::new(|| &BOTH.0);
pub static TABLES: LazyLock<&Schema> = LazyLock::new(|| &BOTH.1);

// Test that migrations are working
#[cfg(test)]
mod tests {
    use rust_query::expect;

    use super::*;

    #[test]
    fn migrations_test() {
        v1::assert_hash(expect!["fe336f7b8ab2a39e"]);
        v2::assert_hash(expect!["fe9891d018ce713f"]);
        v3::assert_hash(expect!["fcc2bc960920cc33"]);
    }
}
