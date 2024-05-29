use rust_query::{client::Client, schema};

#[schema]
#[version(0..2)]
enum Schema {
    #[version(1..)]
    #[unique(file_hash)]
    File {
        timestamp: i64,
        file_hash: i64,
        file_size: i64,
    },
    // a problem benchmark instance
    #[version(1..)]
    #[unique(problem, seed)]
    Instance {
        timestamp: i64,
        problem: File,
        seed: i64,
    },
    // a wasm solution
    // program can only be submitted to a problem once
    #[version(1..)]
    #[unique(program, problem)]
    Solution {
        timestamp: i64,
        program: File,
        problem: File,
        // how many random tests did this solution pass
        random_tests: i64,
    },
    // a random test "or benchmark test" failed
    #[version(1..)]
    #[unique(solution)]
    Failure {
        timestamp: i64,
        solution: Solution,
        seed: i64,
        message: String,
    },
    // a user of the server
    #[version(1..)]
    #[unique(github_id)]
    User {
        timestamp: i64,
        github_id: i64,
        github_login: String,
    },
    // who uploaded the solution
    #[version(1..)]
    #[unique(solution, user)]
    Submission {
        timestamp: i64,
        solution: File,
        user: User,
    },
    // a solution applied to a problem instance results in an execution
    #[version(1..)]
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

pub fn initialize_db() -> (Client, Schema) {
    let prepare = rust_query::migrate::Prepare::open("test.db");
    prepare.migrator().migrate(|_schema| v1::M {}).finish()
}

pub use v1::*;

// Test that migrations are working
#[cfg(test)]
mod tests {
    use rust_query::{expect, migrate::Schema};

    use super::*;

    #[test]
    fn migrations_test() {
        v0::Schema::assert_hash(expect!["3a122e2d6ba33b97"]);
        v1::Schema::assert_hash(expect!["fe336f7b8ab2a39e"]);
    }
}
