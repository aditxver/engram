/// Minimal test: create db, init schema, insert two chunks for one doc, query back.
fn main() {
    use rusqlite::ffi::sqlite3_auto_extension;

    println!("1. registering sqlite-vec...");
    unsafe {
        sqlite3_auto_extension(Some(std::mem::transmute::<
            *const (),
            unsafe extern "C" fn(
                *mut rusqlite::ffi::sqlite3,
                *mut *const std::ffi::c_char,
                *const rusqlite::ffi::sqlite3_api_routines,
            ) -> i32,
        >(sqlite_vec::sqlite3_vec_init as *const ())));
    }

    println!("2. opening db...");
    let conn = rusqlite::Connection::open("/tmp/engram-test.db").unwrap();

    println!("3. init schema...");
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS documents (
             id INTEGER PRIMARY KEY, path TEXT, hash TEXT, snippet TEXT, indexed_at INTEGER
         );
         CREATE VIRTUAL TABLE IF NOT EXISTS chunks USING vec0(
             document_id INTEGER,
             embedding FLOAT[768]
         );",
    )
    .unwrap();

    println!("4. inserting document...");
    conn.execute(
        "INSERT INTO documents (path, hash, snippet, indexed_at) VALUES (?1,?2,?3,?4)",
        rusqlite::params!["test.md", "abc123", "test snippet", 0i64],
    )
    .unwrap();
    let doc_id: i64 = conn
        .query_row("SELECT id FROM documents WHERE path='test.md'", [], |r| {
            r.get(0)
        })
        .unwrap();
    println!("   doc_id = {doc_id}");

    // Insert first chunk
    println!("5. inserting chunk 1...");
    let v1: Vec<f32> = (0..768).map(|i| (i as f32) / 768.0).collect();
    let b1: Vec<u8> = v1.iter().flat_map(|f| f.to_le_bytes()).collect();
    conn.execute(
        "INSERT INTO chunks (document_id, embedding) VALUES (?1, ?2)",
        rusqlite::params![doc_id, b1],
    )
    .unwrap();
    println!("   chunk 1 ok");

    // Insert second chunk (same doc)
    println!("6. inserting chunk 2...");
    let v2: Vec<f32> = (0..768).map(|i| ((768 - i) as f32) / 768.0).collect();
    let b2: Vec<u8> = v2.iter().flat_map(|f| f.to_le_bytes()).collect();
    conn.execute(
        "INSERT INTO chunks (document_id, embedding) VALUES (?1, ?2)",
        rusqlite::params![doc_id, b2],
    )
    .unwrap();
    println!("   chunk 2 ok");

    // Query
    println!("7. searching...");
    let query: Vec<u8> = v1.iter().flat_map(|f| f.to_le_bytes()).collect();
    let mut stmt = conn.prepare(
        "SELECT document_id, distance FROM chunks WHERE embedding MATCH ?1 AND k=5 ORDER BY distance"
    ).unwrap();
    let rows: Vec<(i64, f32)> = stmt
        .query_map(rusqlite::params![query], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(|r| r.unwrap())
        .collect();
    println!("   results: {rows:?}");

    println!("ALL DONE");
    std::fs::remove_file("/tmp/engram-test.db").ok();
}
