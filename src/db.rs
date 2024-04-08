use rusqlite::ToSql;
use rust_query::{
    client::QueryBuilder,
    value::{Db, UnixEpoch, Value},
    Query,
};

use crate::{
    async_sqlite::SharedConnection,
    hash::FileHash,
    tables::{self, SolutionDummy, SubmissionDummy},
};

pub struct GithubId(pub u64);

impl ToSql for GithubId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}

#[must_use]
pub struct InsertSubmission {
    pub github_id: GithubId,
    pub file_hash: FileHash,
    pub problem_hash: FileHash,
}

impl InsertSubmission {
    pub async fn execute(self, conn: &SharedConnection) -> anyhow::Result<()> {
        conn.call(move |conn| -> rusqlite::Result<()> {
            conn.new_query(|q| {
                q.insert::<tables::Solution>(SolutionDummy {
                    file_hash: q.select(i64::from(self.file_hash)),
                    timestamp: q.select(UnixEpoch),
                })
            });
            conn.new_query(|q| {
                let problem = get_problem(q, self.problem_hash);
                let solution = get_solution(q, self.file_hash);
                let user = get_user(q, self.github_id);
                q.insert::<tables::Submission>(SubmissionDummy {
                    problem: q.select(problem),
                    solution: q.select(solution),
                    timestamp: q.select(UnixEpoch),
                    user: q.select(user),
                })
            });
            Ok(())
        })
        .await?;
        Ok(())
    }
}

pub fn get_problem<'t>(q: &mut Query<'_, 't>, has: FileHash) -> Db<'t, tables::Problem> {
    let problem = q.table(tables::Problem);
    q.filter(problem.file_hash.eq(i64::from(has)));
    problem
}

pub fn get_solution<'t>(q: &mut Query<'_, 't>, hash: FileHash) -> Db<'t, tables::Solution> {
    let solution = q.table(tables::Solution);
    q.filter(solution.file_hash.eq(i64::from(hash)));
    solution
}

pub fn get_user<'t>(q: &mut Query<'_, 't>, github_id: GithubId) -> Db<'t, tables::User> {
    let user = q.table(tables::User);
    q.filter(user.github_id.eq(github_id.0 as i64));
    user
}
