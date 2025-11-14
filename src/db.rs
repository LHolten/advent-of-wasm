use rust_query::{TableRow, Transaction};

use crate::migration::{Problem, Schema};

#[derive(Clone, Copy)]
pub struct GithubId(pub i64);

pub fn get_problem(
    db: &Transaction<Schema>,
    problem_name: &str,
) -> Result<TableRow<Problem>, &'static str> {
    db.query_one(Problem::unique(problem_name))
        .ok_or("could not find problem")
}
