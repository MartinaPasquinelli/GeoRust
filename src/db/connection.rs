use crate::errors::DbError;
use rusqlite::Connection;

const CREATE_USERS_TABLE: &str = r#"
    CREATE TABLE IF NOT EXISTS users(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        username TEXT NOT NULL UNIQUE,
        password_hash TEXT NOT NULL,
        created_at TEXT NOT NULL DEFAULT (datetime('now'))
    );
"#;

const CREATE_POSITIONS_TABLE: &str = r#"
    CREATE TABLE IF NOT EXISTS positions(
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        username TEXT NOT NULL,
        lat REAL NOT NULL,
        lon REAL NOT NULL,
        timestamp TEXT NOT NULL DEFAULT (datetime('now'))
    );
"#;

// Inizializza la connessione al DB SQLite e applica lo schema
pub fn init_db(db_path: &str) -> Result<Connection, DbError> {
    let conn = Connection::open(db_path)?;
    init_schema(&conn)?;
    Ok(conn)
}

// Inizializza una connessione SQLite in RAM
#[allow(dead_code)]
pub fn init_in_memory() -> Result<Connection, DbError> {
    let conn = Connection::open_in_memory()?;
    init_schema(&conn)?;
    Ok(conn)
}

// Esegue le query DDL per creare tabelle e indici
fn init_schema(conn: &Connection) -> Result<(), DbError> {
    conn.execute_batch(CREATE_USERS_TABLE)?;
    conn.execute_batch(CREATE_POSITIONS_TABLE)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_in_memory_creates_tables() {
        let conn = init_in_memory().expect("Inizializzazione DB in-memory fallita");

        // Verifica che la tabella users esista
        let count_users: i64 = conn
            .query_row("SELECT count(*) FROM users", [], |r| r.get(0))
            .expect("Tabella users non trovata");
        assert_eq!(count_users, 0);

        // Verifica che la tabella positions esista
        let count_positions: i64 = conn
            .query_row("SELECT count(*) FROM positions", [], |r| r.get(0))
            .expect("Tabella positions non trovata");
        assert_eq!(count_positions, 0);
    }

    #[test]
    fn test_unique_username_constraint() {
        let conn = init_in_memory().unwrap();
        conn.execute(
            "INSERT INTO users (username, password_hash) VALUES ('test_user', 'hash1')",
            [],
        )
        .unwrap();

        let dup = conn.execute(
            "INSERT INTO users (username, password_hash) VALUES ('test_user', 'hash2')",
            [],
        );
        assert!(dup.is_err(), "Il vincolo UNIQUE avrebbe dovuto fallire");
    }
}
