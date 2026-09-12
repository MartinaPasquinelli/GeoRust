use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Errore SQLite: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Utente '{0}' già esistente")]
    UserAlreadyExists(String),

    #[error("Utente non trovato")]
    NotFound,
}

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Errore database: {0}")]
    Db(#[from] DbError),

    #[error("Credenziali non valide")]
    InvalidCredentials,

    #[error("Errore hashing password: {0}")]
    PasswordHashError(String),

    #[error("Dati di input non validi: {0}")]
    InvalidInput(String),
}
