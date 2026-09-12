use serde::{Deserialize, Serialize};

// Re-export UserState from models for consistency
pub use crate::models::UserState;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ClientMessage {
    Login {
        username: String,
        password_hash: String,
    },
    Register {
        username: String,
        password_hash: String,
    },
    StartSimulation {
        lat: f64,
        lon: f64,
        speed: f64,
    },
    UpdatePosition {
        lat: f64,
        lon: f64,
    },
    SendDirectText {
        text: String,
    },
    Disconnect,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ServerMessage {
    UserStatus {
        username: String,
        state: UserState,
    },
    AuthResult {
        success: bool,
        msg: String,
    },
    Error {
        reason: String,
    },
    DirectText {
        #[serde(default)]
        target_user: String,
        from: String,
        text: String,
    },
    BroadcastText {
        from: String,
        text: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::init_in_memory;
    use crate::models::UserState;
    use crate::network::app_state::AppState;
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
    }

    #[test]
    fn test_client_message_serde() {
        let msg = ClientMessage::Login {
            username: "driver1".into(),
            password_hash: "hash123".into(),
        };
        let json = serde_json::to_string(&msg).expect("Serializzazione fallita");
        let parsed: ClientMessage = serde_json::from_str(&json).expect("Deserializzazione fallita");
        match parsed {
            ClientMessage::Login {
                username,
                password_hash,
            } => {
                assert_eq!(username, "driver1");
                assert_eq!(password_hash, "hash123");
            }
            _ => panic!("Variante inattesa"),
        }

        let pos_msg = ClientMessage::UpdatePosition {
            lat: 45.464,
            lon: 9.189,
        };
        let json_pos = serde_json::to_string(&pos_msg).unwrap();
        let parsed_pos: ClientMessage = serde_json::from_str(&json_pos).unwrap();
        match parsed_pos {
            ClientMessage::UpdatePosition { lat, lon } => {
                assert_eq!(lat, 45.464);
                assert_eq!(lon, 9.189);
            }
            _ => panic!("Variante inattesa"),
        }

        let direct_msg = ClientMessage::SendDirectText {
            text: "Ciao Admin!".into(),
        };
        let json_direct = serde_json::to_string(&direct_msg).unwrap();
        let parsed_direct: ClientMessage = serde_json::from_str(&json_direct).unwrap();
        match parsed_direct {
            ClientMessage::SendDirectText { text } => {
                assert_eq!(text, "Ciao Admin!");
            }
            _ => panic!("Variante inattesa"),
        }

        // Legacy alias test removed: SendText variant no longer supported

        let disc = ClientMessage::Disconnect;
        let json_disc = serde_json::to_string(&disc).unwrap();
        assert_eq!(json_disc, "\"Disconnect\"");
        let parsed_disc: ClientMessage = serde_json::from_str(&json_disc).unwrap();
        match parsed_disc {
            ClientMessage::Disconnect => {}
            _ => panic!("Variante inattesa"),
        }
    }

    #[test]
    fn test_server_message_serde() {
        let auth = ServerMessage::AuthResult {
            success: true,
            msg: "Benvenuto!".into(),
        };
        let json_auth = serde_json::to_string(&auth).unwrap();
        let parsed_auth: ServerMessage = serde_json::from_str(&json_auth).unwrap();
        match parsed_auth {
            ServerMessage::AuthResult { success, msg } => {
                assert!(success);
                assert_eq!(msg, "Benvenuto!");
            }
            _ => panic!("Variante inattesa"),
        }

        let direct = ServerMessage::DirectText {
            target_user: "driver1".into(),
            from: "ADMIN_CONSOLE".into(),
            text: "Rientra al deposito.".into(),
        };
        let json_direct = serde_json::to_string(&direct).unwrap();
        let parsed_direct: ServerMessage = serde_json::from_str(&json_direct).unwrap();
        match parsed_direct {
            ServerMessage::DirectText {
                target_user,
                from,
                text,
            } => {
                assert_eq!(target_user, "driver1");
                assert_eq!(from, "ADMIN_CONSOLE");
                assert_eq!(text, "Rientra al deposito.");
            }
            _ => panic!("Variante inattesa"),
        }

        let broadcast = ServerMessage::BroadcastText {
            from: "SERVER".into(),
            text: "Attenzione: traffico intenso!".into(),
        };
        let json_broadcast = serde_json::to_string(&broadcast).unwrap();
        let parsed_broadcast: ServerMessage = serde_json::from_str(&json_broadcast).unwrap();
        match parsed_broadcast {
            ServerMessage::BroadcastText { from, text } => {
                assert_eq!(from, "SERVER");
                assert_eq!(text, "Attenzione: traffico intenso!");
            }
            _ => panic!("Variante inattesa"),
        }

        // Verifica che target_user con #[serde(default)] funzioni anche se omesso nel JSON
        let direct_no_target = r#"{"DirectText":{"from":"SERVER","text":"Info generica"}}"#;
        let parsed_no_target: ServerMessage = serde_json::from_str(direct_no_target).unwrap();
        match parsed_no_target {
            ServerMessage::DirectText {
                target_user,
                from,
                text,
            } => {
                assert_eq!(target_user, "");
                assert_eq!(from, "SERVER");
                assert_eq!(text, "Info generica");
            }
            _ => panic!("Variante inattesa"),
        }
    }

    #[test]
    fn test_invalid_json_handling() {
        let invalid = "{ invalid json }";
        let res: Result<ClientMessage, _> = serde_json::from_str(invalid);
        assert!(res.is_err());
    }
}
