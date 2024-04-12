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
    pub program_hash: FileHash,
    pub problem_hash: FileHash,
}

impl InsertSubmission {
    pub async fn execute(self, conn: &SharedConnection) -> anyhow::Result<()> {
        conn.call(move |conn| -> rusqlite::Result<()> {
            conn.new_query(|q| {
                let problem = get_file(q, self.problem_hash);
                let program = get_file(q, self.program_hash);
                q.insert(SolutionDummy {
                    timestamp: q.select(UnixEpoch),
                    program: q.select(program),
                    problem: q.select(problem),
                    random_tests: q.select(0),
                })
            });
            conn.new_query(|q| {
                let solution = get_file(q, self.program_hash);
                let user = get_user(q, self.github_id);
                q.insert(SubmissionDummy {
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

pub fn get_file<'t>(q: &mut Query<'_, 't>, hash: FileHash) -> Db<'t, tables::File> {
    let file = q.table(tables::File);
    q.filter(file.file_hash.eq(i64::from(hash)));
    file
}

pub fn get_user<'t>(q: &mut Query<'_, 't>, github_id: GithubId) -> Db<'t, tables::User> {
    let user = q.table(tables::User);
    q.filter(user.github_id.eq(github_id.0 as i64));
    user
}
