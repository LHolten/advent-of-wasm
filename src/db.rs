// use diesel::{insert_into, RunQueryDsl};
// use diesel::query_dsl::methods::FilterDsl;

use crate::prisma::PrismaClient;

use crate::hash::FileHash;

pub struct GithubId(pub u64);

impl GithubId {
    pub fn as_i64(self) -> i64 {
        i64::from_le_bytes(self.0.to_le_bytes())
    }
}

#[must_use]
pub struct InsertSubmission {
    pub github_id: GithubId,
    pub file_hash: FileHash,
    pub problem_hash: FileHash,
}

impl InsertSubmission {
    pub async fn execute(self, conn: &PrismaClient) -> anyhow::Result<()> {
        // use crate::schema::Submission::dsl::*;
        // use crate::schema::User::dsl::*;

        // insert_into(Submission).values(&[
        //     userId::eq(self.github_id),
        //     solutionId::eq(self.file_hash),
        //     problemId::eq(self.problem_hash),
        // ]);

        // User.filter(githubId::eq(self.github_id)).load(conn);
        // User.

        use crate::prisma::{problem, solution, user};
        conn.solution()
            .create(self.file_hash.as_i64(), vec![])
            .exec()
            .await?;

        conn.submission()
            .create(
                user::github_id::equals(self.github_id.as_i64()),
                solution::file_hash::equals(self.file_hash.as_i64()),
                problem::file_hash::equals(self.problem_hash.as_i64()),
                vec![],
            )
            .exec()
            .await?;

        Ok(())
    }
}
