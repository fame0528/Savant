use rusqlite::Connection;

fn main() -> anyhow::Result<()> {
    let conn = Connection::open("data/savant.db")?;
    let mut stmt = conn.prepare("SELECT id, partition_id, role, content, is_telemetry FROM chat_history ORDER BY id DESC LIMIT 20")?;
    
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, i32>(4)?,
        ))
    })?;

    println!("ID | PARTITION | ROLE | IS_TELEMETRY | CONTENT (truncated)");
    println!("---------------------------------------------------------");
    for row in rows {
        let (id, part, role, content, is_tel) = row?;
        let truncated = if content.len() > 50 { format!("{}...", &content[..50]) } else { content };
        println!("{} | {} | {} | {} | {}", id, part, role, is_tel, truncated.replace("\n", " "));
    }

    Ok(())
}
