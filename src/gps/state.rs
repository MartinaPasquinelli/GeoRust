use crate::gps::analytics::calculate_distance_km;
use crate::models::UserState;
use chrono::{DateTime, Utc};

const MOVEMENT_THRESHOLD_METERS: f64 = 5.0;
const STOP_TIMEOUT_SECONDS: i64 = 180; // 3 minuti di immobilità per passare a Fermo

#[derive(Debug, Clone)]
pub struct UserTracker {
    pub username: String,
    pub current_state: UserState,
    pub last_lat: Option<f64>,
    pub last_lon: Option<f64>,
    pub last_update_time: Option<DateTime<Utc>>,
    pub last_moved_time: Option<DateTime<Utc>>,
}

impl UserTracker {
    pub fn new(username: String) -> Self {
        Self {
            username,
            current_state: UserState::Sconnesso,
            last_lat: None,
            last_lon: None,
            last_update_time: None,
            last_moved_time: None,
        }
    }

    // Segna l'utente come connesso
    pub fn set_connected(&mut self) {
        if self.current_state == UserState::Sconnesso {
            self.current_state = UserState::Fermo;
        }
    }

    // Segna l'utente come sconnesso
    pub fn set_disconnected(&mut self) {
        self.current_state = UserState::Sconnesso;
    }

    // Aggiorna la posizione e calcola la transizione di stato
    pub fn update_position(&mut self, lat: f64, lon: f64, timestamp: DateTime<Utc>) -> UserState {
        // Se era sconnesso, ora è connesso
        if self.current_state == UserState::Sconnesso {
            self.current_state = UserState::Fermo;
        }

        match (self.last_lat, self.last_lon) {
            (Some(prev_lat), Some(prev_lon)) => {
                let dist_meters = calculate_distance_km(prev_lat, prev_lon, lat, lon) * 1000.0;

                if dist_meters > MOVEMENT_THRESHOLD_METERS {
                    // C'è stato uno spostamento -> transizione immediata a InMovimento
                    self.current_state = UserState::InMovimento;
                    self.last_moved_time = Some(timestamp);
                } else {
                    // Coordinate invariate: controlla se sono passati almeno 3 minuti (180s)
                    if let Some(last_move) = self.last_moved_time {
                        let elapsed_secs = (timestamp - last_move).num_seconds();
                        if elapsed_secs >= STOP_TIMEOUT_SECONDS {
                            self.current_state = UserState::Fermo;
                        }
                    } else {
                        self.current_state = UserState::Fermo;
                    }
                }
            }
            _ => {
                // Prima coordinata registrata
                self.current_state = UserState::Fermo;
                self.last_moved_time = Some(timestamp);
            }
        }

        self.last_lat = Some(lat);
        self.last_lon = Some(lon);
        self.last_update_time = Some(timestamp);
        self.current_state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_state_transitions() {
        let mut tracker = UserTracker::new("auto1".into());
        assert_eq!(tracker.current_state, UserState::Sconnesso);

        let t0 = Utc::now();
        // 1. Prima posizione ricevuta -> Fermo
        let state = tracker.update_position(45.0, 9.0, t0);
        assert_eq!(state, UserState::Fermo);

        // 2. Posizione invariata dopo 30s -> rimane Fermo
        let t1 = t0 + Duration::seconds(30);
        let state = tracker.update_position(45.0, 9.0, t1);
        assert_eq!(state, UserState::Fermo);

        // 3. Spostamento coordinate -> InMovimento
        let t2 = t1 + Duration::seconds(30);
        let state = tracker.update_position(45.01, 9.01, t2);
        assert_eq!(state, UserState::InMovimento);

        // 4. Posizione invariata dopo 1 minuto (60s < 180s) -> rimane InMovimento
        let t3 = t2 + Duration::seconds(60);
        let state = tracker.update_position(45.01, 9.01, t3);
        assert_eq!(state, UserState::InMovimento);

        // 5. Posizione invariata dopo 3 minuti (180s) -> transizione a Fermo
        let t4 = t2 + Duration::seconds(180);
        let state = tracker.update_position(45.01, 9.01, t4);
        assert_eq!(state, UserState::Fermo);

        // 6. Disconnessione -> Sconnesso
        tracker.set_disconnected();
        assert_eq!(tracker.current_state, UserState::Sconnesso);
    }

    #[test]
    fn test_movement_threshold() {
        let mut tracker = UserTracker::new("auto2".into());
        let t0 = Utc::now();
        tracker.update_position(45.0, 9.0, t0);

        // Spostamento infinitesimale (< 5 metri)
        let t1 = t0 + Duration::seconds(30);
        let state = tracker.update_position(45.00001, 9.00001, t1);
        assert_eq!(state, UserState::Fermo);
    }

    #[test]
    fn test_simulation_route_transitions() {
        let mut tracker = UserTracker::new("veicolo_test".into());
        let mut current_time = Utc::now();

        // Le coordinate dal file data/route.csv
        let coords = vec![
            (45.464203, 9.189982), // 0s
            (45.464520, 9.190100), // 30s: Movimento
            (45.465000, 9.191000), // 60s: Movimento
            (45.466000, 9.192000), // 90s: Movimento
            (45.467000, 9.193000), // 120s: Arrivo alla sosta
            (45.467000, 9.193000), // 150s: Sosta (+30s)
            (45.467000, 9.193000), // 180s: Sosta (+60s)
            (45.467000, 9.193000), // 210s: Sosta (+90s)
            (45.467000, 9.193000), // 240s: Sosta (+120s)
            (45.467000, 9.193000), // 270s: Sosta (+150s)
            (45.467000, 9.193000), // 300s: Sosta (+180s = 3 minuti) -> Diventa FERMO!
            (45.467000, 9.193000), // 330s: Ancora FERMO
            (45.468000, 9.194000), // 360s: Ripartenza -> Diventa IN MOVIMENTO!
            (45.469000, 9.195000), // 390s: Movimento
            (45.470000, 9.196000), // 420s: Movimento
        ];

        let mut states = Vec::new();
        for (lat, lon) in coords {
            let state = tracker.update_position(lat, lon, current_time);
            states.push(state);
            current_time = current_time + Duration::seconds(30);
        }

        assert_eq!(states[0], UserState::Fermo);        // Inizio
        assert_eq!(states[1], UserState::InMovimento);  // Inizio spostamento
        assert_eq!(states[5], UserState::InMovimento);  // Durante la sosta (< 3 min)
        assert_eq!(states[9], UserState::InMovimento);  // A 2.5 min di sosta
        assert_eq!(states[10], UserState::Fermo);       // A 3 min di sosta -> Passa a FERMO!
        assert_eq!(states[11], UserState::Fermo);       // Rimane FERMO
        assert_eq!(states[12], UserState::InMovimento); // Ripartenza -> Torna IN MOVIMENTO!
    }
}