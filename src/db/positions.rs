use crate::errors::DbError;
use crate::models::Position;
use rusqlite::{params, Connection};

// Inserisce una nuova posizione nel database.
pub fn insert_position(
    conn: &Connection,
    username: &str,
    lat: f64,
    lon: f64,
) -> Result<Position, DbError> {
    let query = "INSERT INTO positions (username, lat, lon) VALUES (?, ?, ?) \
                 RETURNING id, username, lat, lon, timestamp";

    conn.query_row(query, params![username, lat, lon], |row| {
        Ok(Position {
            id: row.get(0)?,
            username: row.get(1)?,
            lat: row.get(2)?,
            lon: row.get(3)?,
            timestamp: row.get(4)?,
        })
    })
    .map_err(DbError::Sqlite)
}

// Recupera tutte le posizioni di un determinato utente, ordinate per timestamp crescente.
pub fn get_positions_by_user(conn: &Connection, username: &str) -> Result<Vec<Position>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, username, lat, lon, timestamp FROM positions \
         WHERE username = ? ORDER BY timestamp ASC, id ASC",
    )?;

    let pos_iter = stmt.query_map(params![username], |row| {
        Ok(Position {
            id: row.get(0)?,
            username: row.get(1)?,
            lat: row.get(2)?,
            lon: row.get(3)?,
            timestamp: row.get(4)?,
        })
    })?;

    let mut positions = Vec::new();
    for pos in pos_iter {
        positions.push(pos?);
    }
    Ok(positions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::init_in_memory;

    #[test]
    fn test_insert_and_get_positions() {
        let conn = init_in_memory().unwrap();
        let pos1 = insert_position(&conn, "veicolo1", 45.0, 9.0).expect("Inserimento 1 fallito");
        assert_eq!(pos1.username, "veicolo1");
        assert!((pos1.lat - 45.0).abs() < 1e-6);
        assert!((pos1.lon - 9.0).abs() < 1e-6);

        let pos2 = insert_position(&conn, "veicolo1", 45.01, 9.01).expect("Inserimento 2 fallito");
        assert_eq!(pos2.username, "veicolo1");

        let positions = get_positions_by_user(&conn, "veicolo1").expect("Query fallita");
        assert_eq!(positions.len(), 2);
        assert_eq!(positions[0].id, pos1.id);
        assert_eq!(positions[1].id, pos2.id);
    }

    #[test]
    fn test_get_positions_empty_for_unknown_user() {
        let conn = init_in_memory().unwrap();
        let positions = get_positions_by_user(&conn, "non_esisto").unwrap();
        assert!(positions.is_empty());
    }
}
