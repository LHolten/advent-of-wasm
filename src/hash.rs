use std::fmt::Display;

use base64::URL_SAFE_NO_PAD;
use k12::digest::{ExtendableOutput, Update};
use rusqlite::{types::*, ToSql};
use serde::{de, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

impl ToSql for FileHash {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let val = i64::from_le_bytes(self.0);
        Ok(ToSqlOutput::Owned(Value::Integer(val)))
    }
}

impl Display for FileHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&base64::encode_config(self.0, URL_SAFE_NO_PAD))
    }
}

impl<'de> Deserialize<'de> for FileHash {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let encoded = String::deserialize(deserializer)?;
        let mut res = FileHash([0; 8]);
        base64::decode_config_slice(encoded, URL_SAFE_NO_PAD, &mut res.0)
            .map_err(de::Error::custom)?;
        Ok(res)
    }
}
