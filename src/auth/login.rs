use super::hash::verify_password;
use crate::db::users::find_user_by_username;
use crate::errors::AuthError;
use crate::models::{User, UserCredentials};
use rusqlite::Connection;

//Autentica un utente cercando il suo username nel DB e verificando l'hash della password.
pub fn login_user(
    connection: &Connection,
    credentials: &UserCredentials,
) -> Result<User, AuthError> {
    let username = credentials.username.trim();

    // Cerca l'utente nel database
    let user = find_user_by_username(connection, username)?.ok_or(AuthError::InvalidCredentials)?;

    // Verifica se la password fornita corrisponde all'hash memorizzato
    let is_valid = verify_password(&credentials.password, &user.password_hash)?;

    if is_valid {
        Ok(user)
    } else {
        Err(AuthError::InvalidCredentials)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::register::register_user;
    use crate::db::connection::init_in_memory;

    #[test]
    fn test_login_success() {
        let conn = init_in_memory().unwrap();
        let creds = UserCredentials {
            username: "anna".into(),
            password: "segretissima".into(),
        };
        register_user(&conn, &creds).unwrap();

        let logged_user = login_user(&conn, &creds).expect("Login fallito");
        assert_eq!(logged_user.username, "anna");
    }

    #[test]
    fn test_login_wrong_password() {
        let conn = init_in_memory().unwrap();
        let creds = UserCredentials {
            username: "anna".into(),
            password: "segretissima".into(),
        };
        register_user(&conn, &creds).unwrap();

        let wrong_creds = UserCredentials {
            username: "anna".into(),
            password: "wrong_password".into(),
        };
        let res = login_user(&conn, &wrong_creds);
        match res {
            Err(AuthError::InvalidCredentials) => {}
            _ => panic!("Atteso InvalidCredentials"),
        }
    }

    #[test]
    fn test_login_nonexistent_user() {
        let conn = init_in_memory().unwrap();
        let creds = UserCredentials {
            username: "fantasma".into(),
            password: "password123".into(),
        };
        let res = login_user(&conn, &creds);
        match res {
            Err(AuthError::InvalidCredentials) => {}
            _ => panic!("Atteso InvalidCredentials"),
        }
    }
}
