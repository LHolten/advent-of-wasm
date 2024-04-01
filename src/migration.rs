use crate::prisma::PrismaClient;

pub fn initialize_db(conn: &mut PrismaClient) -> anyhow::Result<()> {
    // list_migrations().to_latest(conn)?;

    conn.pragma_update(None, "journal_mode", &"WAL")?;
    conn.pragma_update(None, "synchronous", &"NORMAL")?;
    conn.pragma_update(None, "foreign_keys", &"ON")?;

    Ok(())
}
