use std::fmt::Display;

use base64::URL_SAFE_NO_PAD;
use k12::digest::{ExtendableOutput, Update};
use rusqlite::{types::*, ToSql};

pub struct Hash([u8; 8]);

impl Hash {
    pub fn new(data: impl AsRef<[u8]>) -> Self {
        let mut hasher = k12::KangarooTwelve::new();
        hasher.update(data.as_ref());
        let hash = hasher.finalize_boxed(8);
        let hash = *Box::<[u8; 8]>::try_from(hash).unwrap();
        Self(hash)
    }
}

impl ToSql for Hash {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let val = i64::from_le_bytes(self.0);
        Ok(ToSqlOutput::Owned(Value::Integer(val)))
    }
}

impl Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&base64::encode_config(self.0, URL_SAFE_NO_PAD))
    }
}
