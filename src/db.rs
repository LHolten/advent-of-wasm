use rust_query::{
    value::{Db, Value},
    Query,
};

use crate::{async_sqlite::DB, hash::FileHash, migration};

#[derive(Clone, Copy)]
pub struct GithubId(pub i64);

pub fn get_file<'t>(q: &mut Query<'t>, hash: FileHash) -> Db<'t, migration::File> {
    let file = q.table(&DB.file);
    q.filter(file.file_hash.eq(i64::from(hash)));
    file
}

pub fn get_user<'t>(q: &mut Query<'t>, github_id: GithubId) -> Db<'t, migration::User> {
    let user = q.table(&DB.user);
    q.filter(user.github_id.eq(github_id.0));
    user
}
