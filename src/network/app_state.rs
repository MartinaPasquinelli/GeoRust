use crate::network::state::ServerState;
use rusqlite::Connection;
use std::sync::{Arc, Mutex, RwLock};

// Stato condiviso dell'applicazione server.
#[derive(Clone)]
pub struct AppState {
    state: Arc<RwLock<ServerState>>,
    db: Arc<Mutex<Connection>>,
}

impl AppState {
    // Crea un nuovo AppState
    pub fn new(conn: Connection) -> Self {
        Self {
            state: Arc::new(RwLock::new(ServerState::default())),
            db: Arc::new(Mutex::new(conn)),
        }
    }

    // Restituisce un clone del riferimento allo stato dei client connessi.
    pub fn shared_state(&self) -> Arc<RwLock<ServerState>> {
        self.state.clone()
    }

    // Esegue una closure con accesso in lettura allo ServerState.
    pub fn with_state<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&ServerState) -> R,
    {
        let guard = self.state.read().expect("ServerState RwLock avvelenato");
        f(&guard)
    }

    // Esegue una closure con accesso in scrittura allo ServerState.
    pub fn with_state_mut<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut ServerState) -> R,
    {
        let mut guard = self.state.write().expect("ServerState RwLock avvelenato");
        f(&mut guard)
    }

    // Restituisce un clone del Arc<Mutex<Connection>> per passarlo a spawn_blocking.
    pub fn db_arc(&self) -> Arc<Mutex<Connection>> {
        self.db.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::init_in_memory;
    use crate::models::UserState;
    use crate::network::state::ClientInfo;
    use tokio::sync::mpsc;

    #[test]
    fn test_app_state_read_write() {
        let conn = init_in_memory().unwrap();
        let app_state = AppState::new(conn);

        // Inizialmente vuoto
        let count = app_state.with_state(|st| st.online_users.len());
        assert_eq!(count, 0);

        // Inserimento utente
        let (tx, _rx) = mpsc::channel(10);
        let client_info = ClientInfo {
            sender: tx,
            last_update: None,
            current_state: UserState::Fermo,
        };
        app_state.with_state_mut(|st| {
            st.online_users.insert("bob".into(), client_info);
        });

        // Verifica lettura
        let has_bob = app_state.with_state(|st| st.online_users.contains_key("bob"));
        assert!(has_bob);

        // Riferimento al DB
        let db_arc = app_state.db_arc();
        let db_lock = db_arc.lock().unwrap();
        let count_db_users: i64 = db_lock
            .query_row("SELECT count(*) FROM users", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count_db_users, 0);
    }

    #[tokio::test]
    async fn test_concurrent_app_state_access() {
        let conn = init_in_memory().unwrap();
        let app_state = AppState::new(conn);

        let mut handles = Vec::new();
        for i in 0..10 {
            let state_clone = app_state.clone();
            handles.push(tokio::spawn(async move {
                let username = format!("user_{}", i);
                let (tx, _rx) = mpsc::channel(10);
                state_clone.with_state_mut(|st| {
                    let client_info = ClientInfo {
                        sender: tx,
                        last_update: None,
                        current_state: UserState::Fermo,
                    };
                    st.online_users.insert(username.clone(), client_info);
                });
                let exists = state_clone.with_state(|st| st.online_users.contains_key(&username));
                assert!(exists);
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        let total = app_state.with_state(|st| st.online_users.len());
        assert_eq!(total, 10);
    }
}
