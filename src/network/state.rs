use std::collections::HashMap;
use tokio::sync::mpsc;
use std::time::Instant;
use crate::models::UserState;
use crate::gps::state::UserTracker;

pub type ClientSender = mpsc::Sender<String>;

/// Stato condiviso del server: mappa username → informazioni del client connesso.
#[derive(Default)]
pub struct ServerState {
    pub online_users: HashMap<String, ClientInfo>,
    pub user_trackers: HashMap<String, UserTracker>,
}

#[derive(Clone)]
pub struct ClientInfo {
    pub sender: ClientSender,
    pub last_update: Option<Instant>,
    pub current_state: UserState, // current user state as determined by server
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_state_user_lifecycle() {
        let mut state = ServerState::default();
        assert!(state.online_users.is_empty());

        let (tx, _rx) = mpsc::channel(10);
        let client_info = ClientInfo { sender: tx, last_update: None, stopped: false, current_state: UserState::Fermo };
        state.online_users.insert("alice".into(), client_info);
        assert_eq!(state.online_users.len(), 1);
        assert!(state.online_users.contains_key("alice"));

        let removed = state.online_users.remove("alice");
        assert!(removed.is_some());
        assert!(state.online_users.is_empty());
    }
}
