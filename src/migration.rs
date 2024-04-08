use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

fn list_migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(include_str!("migration/initial.sql"))])
}

pub fn initialize_db(conn: &mut Connection) -> anyhow::Result<()> {
    list_migrations().to_latest(conn)?;

    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;

    Ok(())
}

// Test that migrations are working
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_test() -> rusqlite_migration::Result<()> {
        list_migrations().validate()
    }
}
