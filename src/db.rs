use rusqlite::ToSql;

use crate::{async_sqlite::SharedConnection, hash::FileHash, include_query};

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
        let submission_query = include_query!("submit.prql");
        let submission_insert =
            format!("INSERT INTO submission (problem, user, solution) {submission_query}");

        conn.call(move |conn| {
            conn.execute(
                "INSERT OR IGNORE INTO solution (file_hash) VALUES ($1)",
                [&self.file_hash],
            )?;
            conn.execute(
                &submission_insert,
                &[
                    ("@github_id", &self.github_id as &dyn ToSql),
                    ("@solution_hash", &self.file_hash),
                    ("@problem_hash", &self.problem_hash),
                ],
            )
        })
        .await?;
        Ok(())
    }
}

impl SharedConnection {
    pub async fn get_user(&self, github_id: u64) -> anyhow::Result<u64> {
        self.call(move |conn| {
            conn.query_row(
                "SELECT id FROM user WHERE github_id == $1",
                [github_id],
                |x| x.get("id"),
            )
        })
        .await
        .map_err(From::from)
    }

    // pub async fn submit(&self, github_id: u64, )
}
