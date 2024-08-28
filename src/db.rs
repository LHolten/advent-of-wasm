use rust_query::{Free, ReadTransaction};

use crate::migration::{Problem, Schema};

#[derive(Clone, Copy)]
pub struct GithubId(pub i64);

pub fn get_problem<'a>(
    db: &ReadTransaction<'a, Schema>,
    problem_name: &str,
) -> Result<Free<'a, Problem>, &'static str> {
    db.get(Problem::unique(problem_name))
        .ok_or("could not find problem")
}
