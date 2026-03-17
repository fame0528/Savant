use fjall::OptimisticTxDatabase;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let db_path = args.get(1).map(|s| s.as_str()).unwrap_or("./data/savant");

    println!("Inspecting database at: {}", db_path);

    let db = OptimisticTxDatabase::open(PathBuf::from(db_path))?;

    // List all keyspaces and their contents
    for ks_name in db.keyspaces() {
        println!("\n=== Keyspace: {} ===", ks_name);
        let ks = db.keyspace(&ks_name, fjall::KeyspaceCreateOptions::default)?;

        let mut count = 0;
        for item in ks.inner().iter().take(20) {
            let key = item
                .key()
                .map(|k| String::from_utf8_lossy(&k).to_string())
                .unwrap_or_else(|_| "<invalid>".to_string());
            let value = item
                .value()
                .map(|v| {
                    let s = String::from_utf8_lossy(&v);
                    let truncated: String = s.chars().take(80).collect();
                    truncated.replace('\n', " ")
                })
                .unwrap_or_else(|_| "<invalid>".to_string());

            println!("  {} | {}", key, value);
            count += 1;
        }

        let total = ks.inner().iter().count();
        if total > 20 {
            println!("  ... ({} total entries, showing first 20)", total);
        } else {
            println!("  ({} entries)", count);
        }
    }

    Ok(())
}
