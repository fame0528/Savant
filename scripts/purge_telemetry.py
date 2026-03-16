import sqlite3
import re
import os

def purge():
    db_path = "savant.db"
    if not os.path.exists(db_path):
        print(f"Database {db_path} not found.")
        return

    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()

    # The "Global Sinkhole" regex
    sinkhole = re.compile(r"<thought>|Thought:|Subconscious Reflection:|HEARTBEAT_OK|NOMINAL|PULSE|TICK|\[PROACTIVE", re.IGNORECASE)

    # Check columns
    cursor.execute("PRAGMA table_info(chat_history)")
    columns = [row[1] for row in cursor.fetchall()]
    has_telemetry_col = "is_telemetry" in columns

    if has_telemetry_col:
        cursor.execute("SELECT id, content FROM chat_history WHERE is_telemetry = 0")
    else:
        cursor.execute("SELECT id, content FROM chat_history")
    
    rows = cursor.fetchall()

    to_purge = []
    for row_id, content in rows:
        if sinkhole.search(content):
            to_purge.append(row_id)

    print(f"Found {len(to_purge)} messages leaking technical noise into the dialogue lane.")

    if to_purge:
        if has_telemetry_col:
            cursor.executemany("UPDATE chat_history SET is_telemetry = 1 WHERE id = ?", [(rid,) for rid in to_purge])
            print(f"✅ Successfully marked {len(to_purge)} messages as telemetry.")
        else:
            cursor.executemany("DELETE FROM chat_history WHERE id = ?", [(rid,) for rid in to_purge])
            print(f"✅ Successfully deleted {len(to_purge)} leaked messages.")
        conn.commit()
    else:
        print("✨ Database is already clean.")

    conn.close()

if __name__ == "__main__":
    purge()
