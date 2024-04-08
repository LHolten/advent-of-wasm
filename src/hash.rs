use std::{fmt::Display, str::FromStr};

use base64::URL_SAFE_NO_PAD;
use k12::digest::{ExtendableOutput, Update};
use rusqlite::{types::*, ToSql};
use serde::{de, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileHash([u8; 8]);

impl FileHash {
    pub fn new(data: impl AsRef<[u8]>) -> Self {
        let mut hasher = k12::KangarooTwelve::new();
        hasher.update(data.as_ref());
        let hash = hasher.finalize_boxed(8);
        let hash = *Box::<[u8; 8]>::try_from(hash).unwrap();
        Self(hash)
    }
}

impl From<FileHash> for i64 {
    fn from(value: FileHash) -> Self {
        i64::from_le_bytes(value.0)
    }
}

impl ToSql for FileHash {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Integer((*self).into())))
    }
}

impl From<i64> for FileHash {
    fn from(value: i64) -> Self {
        FileHash(value.to_le_bytes())
    }
}

impl FromSql for FileHash {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        i64::column_result(value).map(From::from)
    }
}

impl Display for FileHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&base64::encode_config(self.0, URL_SAFE_NO_PAD))
    }
}

impl FromStr for FileHash {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut res = FileHash([0; 8]);
        base64::decode_config_slice(s, URL_SAFE_NO_PAD, &mut res.0)?;
        Ok(res)
    }
}

impl<'de> Deserialize<'de> for FileHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let encoded = String::deserialize(deserializer)?;
        encoded.parse().map_err(de::Error::custom)
    }
}
