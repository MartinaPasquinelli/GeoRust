use crate::errors::DbError;
use crate::models::User;
use rusqlite::{params, Connection, OptionalExtension};

// Inserisce un nuovo utente nel database. Usa RETURNING per ottenere tutti i dati in una singola query, evitando il doppio round-trip.
// Ritorna DbError::UserAlreadyExists se l'username è già registrato.
pub fn insert_user(
    conn: &Connection,
    username: &str,
    password_hash: &str,
) -> Result<User, DbError> {
    let query = "INSERT INTO users (username, password_hash) VALUES (?, ?) \
                 RETURNING id, username, password_hash, created_at";

    conn.query_row(query, params![username, password_hash], |row| {
        Ok(User {
            id: row.get(0)?,
            username: row.get(1)?,
            password_hash: row.get(2)?,
            created_at: row.get(3)?,
        })
    })
    .map_err(|err| match err {
        // Intercettiamo l'errore di vincolo UNIQUE di SQLite per username duplicati
        rusqlite::Error::SqliteFailure(e, _)
            if e.code == rusqlite::ErrorCode::ConstraintViolation =>
        {
            DbError::UserAlreadyExists(username.to_string())
        }
        // Per tutti gli altri errori SQLite
        other => DbError::Sqlite(other),
    })
}

///Cerca un utente tramite il suo username. Usa prepare_cached per riutilizzare lo statement già compilato su chiamate ripetute.
pub fn find_user_by_username(conn: &Connection, username: &str) -> Result<Option<User>, DbError> {
    let query = "SELECT id, username, password_hash, created_at FROM users WHERE username = ?";

    let mut stmt = conn.prepare_cached(query)?;
    let user_opt = stmt
        .query_row(params![username], |row| {
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                password_hash: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .optional()?; // .optional() trasforma QueryReturnedNoRows in Ok(None)

    Ok(user_opt)
}

// Ritorna la lista di tutti gli utenti registrati nel database.
pub fn get_all_users(conn: &Connection) -> Result<Vec<User>, DbError> {
    let query = "SELECT id, username, password_hash, created_at FROM users ORDER BY username ASC";
    let mut stmt = conn.prepare(query)?;
    let user_iter = stmt.query_map([], |row| {
        Ok(User {
            id: row.get(0)?,
            username: row.get(1)?,
            password_hash: row.get(2)?,
            created_at: row.get(3)?,
        })
    })?;

    let mut users = Vec::new();
    for u in user_iter {
        users.push(u?);
    }
    Ok(users)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::init_in_memory;

    #[test]
    fn test_insert_and_find_user() {
        let conn = init_in_memory().unwrap();
        let inserted = insert_user(&conn, "marco", "hash_marco").expect("Inserimento fallito");
        assert_eq!(inserted.username, "marco");
        assert_eq!(inserted.password_hash, "hash_marco");

        let found = find_user_by_username(&conn, "marco").expect("Query fallita");
        assert!(found.is_some());
        let user = found.unwrap();
        assert_eq!(user.id, inserted.id);
        assert_eq!(user.username, "marco");
    }

    #[test]
    fn test_find_nonexistent_user_returns_none() {
        let conn = init_in_memory().unwrap();
        let found = find_user_by_username(&conn, "inesistente").unwrap();
        assert!(found.is_none());
    }

    #[test]
    fn test_insert_duplicate_user_fails() {
        let conn = init_in_memory().unwrap();
        insert_user(&conn, "giulia", "hash1").unwrap();
        let res = insert_user(&conn, "giulia", "hash2");

        match res {
            Err(DbError::UserAlreadyExists(u)) => assert_eq!(u, "giulia"),
            _ => panic!("Atteso DbError::UserAlreadyExists"),
        }
    }

    #[test]
    fn test_get_all_users_ordered() {
        let conn = init_in_memory().unwrap();
        insert_user(&conn, "zaccaria", "h1").unwrap();
        insert_user(&conn, "alberto", "h2").unwrap();
        insert_user(&conn, "marta", "h3").unwrap();

        let all = get_all_users(&conn).unwrap();
        assert_eq!(all.len(), 3);
        // Ordinati per username ASC
        assert_eq!(all[0].username, "alberto");
        assert_eq!(all[1].username, "marta");
        assert_eq!(all[2].username, "zaccaria");
    }
}
