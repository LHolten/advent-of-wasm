use std::collections::HashMap;

use crate::problem::ProblemDir;
use rust_query::{
    migration::{schema, Alter, Config, Create},
    Database, Dummy, IntoColumn, LocalClient, Table,
};

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
    // TODO: add trait constraints to migration types
    let m = m.migrate(v2::update::Schema {
        problem: Box::new(|rows| {
            let file = v1::File::join(rows);
            let hash = file.file_hash();

            // figure out which files are problems
            let mut cond = false.into_column();
            for problem_hash in hash_to_name.keys() {
                cond = cond.or(hash.eq(problem_hash));
            }
            rows.filter(cond);

            Create::new(v2::update::ProblemMigration {
                name: hash.map_dummy(|hash| hash_to_name.remove(&hash).unwrap()),
                timestamp: file.timestamp(),
                original: file,
            })
        }),
    });
    let m = m.migrate(v3::update::Schema {
        problem: Box::new(|_problem| Alter::new(v3::update::ProblemMigration {})),
        instance: Box::new(|instance| {
            Alter::new(v3::update::InstanceMigration {
                problem: v2::Problem::unique_original(instance.problem()).map_dummy(Option::unwrap),
            })
        }),
        solution: Box::new(|solution| {
            Alter::new(v3::update::SolutionMigration {
                problem: v2::Problem::unique_original(solution.problem()).map_dummy(Option::unwrap),
            })
        }),
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
        expect!["fe9891d018ce713f"].assert_eq(&hash_schema::<v2::Schema>());
        expect!["fcc2bc960920cc33"].assert_eq(&hash_schema::<v3::Schema>());
    }
}
