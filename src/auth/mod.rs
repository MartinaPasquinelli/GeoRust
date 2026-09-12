mod hash;
pub mod login;
pub mod register;

pub use login::login_user;
pub use register::register_user;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::errors::{AuthError};
    use crate::models::UserCredentials;
    use crate::db::connection::init_in_memory;

    #[test]
    fn test_full_auth_flow() {
        let conn = init_in_memory().unwrap();
        let creds = UserCredentials {
            username: "mario".to_string(),
            password: "super_password_123".to_string(),
        };

        // 1. Registrazione OK
        let registered_user = register_user(&conn, &creds).unwrap();
        assert_eq!(registered_user.username, "mario");

        // 2. Login OK
        let logged_in_user = login_user(&conn, &creds).unwrap();
        assert_eq!(logged_in_user.id, registered_user.id);

        // 3. Login con password errata FAIL
        let wrong_creds = UserCredentials {
            username: "mario".to_string(),
            password: "password_sbagliata".to_string(),
        };
        let err = login_user(&conn, &wrong_creds).unwrap_err();
        assert!(matches!(err, AuthError::InvalidCredentials));
    }

    #[test]
    fn test_register_duplicate_username_fails() {
        let conn = init_in_memory().unwrap();
        let creds = UserCredentials {
            username: "bob".to_string(),
            password: "password123".to_string(),
        };

        register_user(&conn, &creds).unwrap();

        // La seconda registrazione deve fallire
        let err = register_user(&conn, &creds).unwrap_err();
        assert!(matches!(err, AuthError::Db(_)));
    }
}
