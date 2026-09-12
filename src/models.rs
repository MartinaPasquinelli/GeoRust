use serde::{Deserialize, Serialize};

/// Riga della tabella 'users'
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub created_at: String,
}

/// Data Transfer Object (DTO) per registrazione/login
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCredentials {
    pub username: String,
    pub password: String,
}

/// Riga della tabella 'positions'
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Position {
    pub id: i64,
    pub username: String,
    pub lat: f64,
    pub lon: f64,
    pub timestamp: String,
}

/// Stati possibili del veicolo/utente
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UserState {
    Sconnesso,
    Fermo,
    InMovimento,
}

impl std::fmt::Display for UserState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserState::Sconnesso => write!(f, "Sconnesso"),
            UserState::Fermo => write!(f, "Fermo"),
            UserState::InMovimento => write!(f, "In Movimento"),
        }
    }
}

/// Intervalli temporali programmabili per l'analisi del movimento
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeRange {
    Today,
    ThisWeek,
    ThisMonth,
    AllTime,
}

/// Risultato dell'analisi del movimento
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MovementAnalytics {
    pub total_distance_km: f64,
    pub average_speed_kmh: f64,
    pub moving_duration_secs: i64,
    pub stopped_duration_secs: i64,
}