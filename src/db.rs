use rusqlite::ToSql;
use rust_query::{
    value::{Db, Value},
    Query,
};

use crate::{
    hash::FileHash,
    tables::{self},
};

#[derive(Clone, Copy)]
pub struct GithubId(pub i64);

impl ToSql for GithubId {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        self.0.to_sql()
    }
}

pub fn get_file<'t>(q: &mut Query<'_, 't>, hash: FileHash) -> Db<'t, tables::File> {
    let file = q.table(tables::File);
    q.filter(file.file_hash.eq(i64::from(hash)));
    file
}

pub fn get_user<'t>(q: &mut Query<'_, 't>, github_id: GithubId) -> Db<'t, tables::User> {
    let user = q.table(tables::User);
    q.filter(user.github_id.eq(github_id.0));
    user
}
