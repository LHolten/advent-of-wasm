use rust_query::Just;

use crate::migration::{Problem, DB, TABLES};

#[derive(Clone, Copy)]
pub struct GithubId(pub i64);

pub fn get_problem(problem_name: &str) -> Result<Just<'static, Problem>, &'static str> {
    DB.get(TABLES.problem.unique(&problem_name))
        .ok_or("could not find problem")
}
