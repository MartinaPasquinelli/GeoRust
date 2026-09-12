use super::hash::hash_password;
use crate::db::users::insert_user;
use crate::errors::AuthError;
use crate::models::{User, UserCredentials};
use rusqlite::Connection;

// Registra un nuovo utente nel sistema: valida l'input, calcola l'hash della password e lo salva nel DB.
pub fn register_user(
    connection: &Connection,
    credentials: &UserCredentials,
) -> Result<User, AuthError> {
    let username = credentials.username.trim();

    // Validazione di base degli input
    if username.is_empty() {
        return Err(AuthError::InvalidInput(
            "L'username non può essere vuoto".into(),
        ));
    }
    if credentials.password.len() < 4 {
        return Err(AuthError::InvalidInput(
            "La password deve contenere almeno 4 caratteri".into(),
        ));
    }

    //Generazione dell'hash sicuro
    let password_hash = hash_password(&credentials.password)?;

    // Salvataggio nel database SQLite, gestisce automaticamente l'errore di username duplicato
    let user = insert_user(connection, username, &password_hash)?;

    Ok(user)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::init_in_memory;

    #[test]
    fn test_register_user_success() {
        let conn = init_in_memory().unwrap();
        let creds = UserCredentials {
            username: "mario".into(),
            password: "password123".into(),
        };

        let user = register_user(&conn, &creds).expect("Registrazione fallita");
        assert_eq!(user.username, "mario");
        assert!(user.password_hash.contains(':'));
    }

    #[test]
    fn test_register_empty_username_fails() {
        let conn = init_in_memory().unwrap();
        let creds = UserCredentials {
            username: "   ".into(),
            password: "password123".into(),
        };

        let res = register_user(&conn, &creds);
        match res {
            Err(AuthError::InvalidInput(_)) => {}
            _ => panic!("Atteso InvalidInput per username vuoto"),
        }
    }

    #[test]
    fn test_register_short_password_fails() {
        let conn = init_in_memory().unwrap();
        let creds = UserCredentials {
            username: "mario".into(),
            password: "12".into(),
        };

        let res = register_user(&conn, &creds);
        match res {
            Err(AuthError::InvalidInput(_)) => {}
            _ => panic!("Atteso InvalidInput per password corta"),
        }
    }

    #[test]
    fn test_register_duplicate_username_fails() {
        let conn = init_in_memory().unwrap();
        let creds = UserCredentials {
            username: "luigi".into(),
            password: "password123".into(),
        };

        register_user(&conn, &creds).unwrap();
        let res = register_user(&conn, &creds);
        assert!(res.is_err());
    }
}
