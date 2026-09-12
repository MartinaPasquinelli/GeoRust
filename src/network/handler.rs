use crate::auth::login::login_user;
use crate::auth::register::register_user;
use crate::db::positions::insert_position;
use crate::models::UserCredentials;
use crate::models::UserState;
use chrono::Utc;
use crate::gps::state::UserTracker;
use crate::network::state::ClientInfo;
use crate::network::app_state::AppState;
use crate::network::protocol::{ClientMessage, ServerMessage};
use futures::{SinkExt, StreamExt};
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::task;
use tokio_util::codec::{Framed, LinesCodec};

pub async fn handle_client(stream: TcpStream, state: AppState) {
    let framed = Framed::new(stream, LinesCodec::new());
    let (mut tx, mut rx) = framed.split();
    let (mpsc_tx, mut mpsc_rx) = mpsc::channel::<String>(32);

    tokio::spawn(async move {
        while let Some(msg) = mpsc_rx.recv().await {
            if tx.send(msg).await.is_err() {
                break;
            }
        }
    });

    let mut current_username: Option<String> = None;
    let mut timeout_check = tokio::time::interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            _ = timeout_check.tick() => {
                if let Some(ref uname) = current_username {
                    let maybe_status = state.with_state_mut(|st| {
                        if let Some(ci) = st.online_users.get_mut(uname) {
                            if let Some(last_update) = ci.last_update {
                                if last_update.elapsed() >= Duration::from_secs(180) && ci.current_state != UserState::Fermo {
                                    println!("Nessun UpdatePosition ricevuto da {} per 180 secondi. Stato -> Fermo.", uname);
                                    ci.current_state = UserState::InMovimento;
                                    if let Some(tracker) = st.user_trackers.get_mut(uname) { tracker.current_state = UserState::Fermo; }
                                    Some(ServerMessage::UserStatus { username: uname.clone(), state: UserState::Fermo })
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    });

                    if let Some(status_msg) = maybe_status {
                        if let Ok(json) = serde_json::to_string(&status_msg) {
                            let senders: Vec<_> = state.with_state(|st| st.online_users.values().map(|ci| ci.sender.clone()).collect());
                            for sender in senders {
                                let _ = sender.send(json.clone()).await;
                            }
                        }
                    }
                }
            }

            maybe_line = rx.next() => {
                let Some(Ok(line)) = maybe_line else {
                    break;
                };

                if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&line) {
                    match client_msg {
                        ClientMessage::Login { username, password_hash } => {
                            if username.starts_with("ADMIN") {
                                let _credentials = UserCredentials { username: username.clone(), password: password_hash };
                                current_username = Some(username.clone());

                                state.with_state_mut(|st| {
                                    st.online_users.insert(username.clone(), ClientInfo {
                                        sender: mpsc_tx.clone(),
                                        last_update: Some(Instant::now()),
                                        current_state: UserState::InMovimento,
                                    });
                                });

                                let init_status = ServerMessage::UserStatus { username: username.clone(), state: UserState::Fermo };

                                if let Ok(json) = serde_json::to_string(&init_status) {
                                    let senders: Vec<_> = state.with_state(|st| st.online_users.values().map(|ci| ci.sender.clone()).collect());
                                    for sender in senders {
                                        let _ = sender.send(json.clone()).await;
                                    }
                                }

                                let response = ServerMessage::AuthResult { success: true, msg: "Login Amministratore effettuato con successo!".into() };

                                if let Ok(json_resp) = serde_json::to_string(&response) {
                                    let _ = mpsc_tx.send(json_resp).await;
                                }

                                continue;
                            }

                            let db_arc = state.db_arc();
                            let credentials = UserCredentials { username: username.clone(), password: password_hash };

                            let auth_result = task::spawn_blocking(move || {
                                let conn = db_arc.lock().expect("DB Mutex avvelenato");
                                login_user(&conn, &credentials)
                            }).await;

                            let mut success = false;

                            let response = match auth_result {
                                Ok(Ok(_user)) => {
                                    success = true;
                                    ServerMessage::AuthResult { success: true, msg: "Login effettuato con successo!".into() }
                                }
                                Ok(Err(_err)) => ServerMessage::AuthResult { success: false, msg: "Credenziali non valide.".into() },
                                Err(_) => ServerMessage::AuthResult { success: false, msg: "Errore interno del server.".into() },
                            };

                            if success {
                                current_username = Some(username.clone());

                                state.with_state_mut(|st| {
                                    st.online_users.insert(username.clone(), ClientInfo {
                                        sender: mpsc_tx.clone(),
                                        last_update: Some(Instant::now()),
                                        current_state: UserState::InMovimento,
                                    });
                                });
                            }

                            if let Ok(json_resp) = serde_json::to_string(&response) {
                                let _ = mpsc_tx.send(json_resp).await;
                            }
                        }

                        ClientMessage::Register { username, password_hash } => {
                            let db_arc = state.db_arc();
                            let credentials = UserCredentials { username: username.clone(), password: password_hash };

                            let reg_result = task::spawn_blocking(move || {
                                let conn = db_arc.lock().expect("DB Mutex avvelenato");
                                register_user(&conn, &credentials)
                            }).await;

                            let mut success = false;

                            let response = match reg_result {
                                Ok(Ok(_user)) => {
                                    success = true;
                                    ServerMessage::AuthResult { success: true, msg: "Registrazione completata con successo!".into() }
                                }
                                Ok(Err(err)) => ServerMessage::AuthResult { success: false, msg: format!("Errore di registrazione: {:?}", err) },
                                Err(_) => ServerMessage::AuthResult { success: false, msg: "Errore interno del server.".into() },
                            };

                            if success {
                                current_username = Some(username.clone());

                                state.with_state_mut(|st| {
                                    st.online_users.insert(username.clone(), ClientInfo {
                                        sender: mpsc_tx.clone(),
                                        last_update: Some(Instant::now()),
                                        current_state: UserState::Fermo,
                                    });
                                });
                            }

                            if let Ok(json_resp) = serde_json::to_string(&response) {
                                let _ = mpsc_tx.send(json_resp).await;
                            }
                        }

                        ClientMessage::UpdatePosition { lat, lon } => {
                            println!("Posizione ricevuta: lat={}, lon={}", lat, lon);

                            if let Some(ref uname) = current_username {
                                let db_arc = state.db_arc();
                                let uname_clone = uname.clone();

                                let _handle = task::spawn_blocking(move || {
                                    let conn = db_arc.lock().expect("DB Mutex avvelenato");

                                    if let Err(e) = insert_position(&conn, &uname_clone, lat, lon) {
                                        eprintln!("Errore nel salvare la posizione: {:?}", e);
                                    }
                                });

                                let maybe_status = state.with_state_mut(|st| {
                                    let tracker = st.user_trackers.entry(uname.clone()).or_insert_with(|| UserTracker::new(uname.clone()));
                                    let prev = tracker.current_state;
                                    let new_state = tracker.update_position(lat, lon, Utc::now());

                                    if let Some(ci) = st.online_users.get_mut(uname) {
                                        ci.last_update = Some(Instant::now());
                                        ci.current_state = new_state;
                                    }

                                    if new_state != prev {
                                        Some(ServerMessage::UserStatus { username: uname.clone(), state: new_state })
                                    } else {
                                        None
                                    }
                                });

                                if let Some(status_msg) = maybe_status {
                                    if let Ok(json) = serde_json::to_string(&status_msg) {
                                        let senders: Vec<_> = state.with_state(|st| st.online_users.values().map(|ci| ci.sender.clone()).collect());
                                        for sender in senders {
                                            let _ = sender.send(json.clone()).await;
                                        }
                                    }
                                }
                            }
                        }

                        ClientMessage::StartSimulation { .. } => {
                            println!("Ricevuto comando di avvio simulazione.");
                        }

                        ClientMessage::SendDirectText { text } => {
                            let uname_str = current_username.clone().unwrap_or_else(|| "Sconosciuto".to_string());
                            println!("[MESSAGGIO DIRETTO DA {}]: {}", uname_str, text);

                            let server_msg = ServerMessage::DirectText {
                                target_user: "ADMIN_CONSOLE".to_string(),
                                from: uname_str,
                                text,
                            };

                            if let Ok(json) = serde_json::to_string(&server_msg) {
                                let admin_sender = state.with_state(|st| st.online_users.get("ADMIN_CONSOLE").map(|ci| ci.sender.clone()));

                                if let Some(sender) = admin_sender {
                                    let _ = sender.send(json).await;
                                }
                            }
                        }

                        ClientMessage::Disconnect => {
                            println!("Il client ha richiesto la disconnessione.");
                            break;
                        }
                    }
                } else if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&line) {
                    match server_msg {
                        ServerMessage::BroadcastText { from, text } => {
                            let uname_str = current_username.clone().unwrap_or(from);
                            println!("[BROADCAST DA {}]: {}", uname_str, text);

                            let forward_msg = ServerMessage::BroadcastText { from: uname_str, text };

                            if let Ok(json) = serde_json::to_string(&forward_msg) {
                                let senders: Vec<_> = state.with_state(|st| st.online_users.values().map(|ci| ci.sender.clone()).collect());

                                for sender in senders {
                                    let _ = sender.send(json.clone()).await;
                                }
                            }
                        }

                        ServerMessage::DirectText { target_user, from, text } => {
                            let uname_str = current_username.clone().unwrap_or(from);
                            println!("[PRIVATO DA {} a {}]: {}", uname_str, target_user, text);

                            let forward_msg = ServerMessage::DirectText {
                                target_user: target_user.clone(),
                                from: uname_str,
                                text,
                            };

                            if let Ok(json) = serde_json::to_string(&forward_msg) {
                                let sender_opt = state.with_state(|st| st.online_users.get(&target_user).map(|ci| ci.sender.clone()));

                                if let Some(sender) = sender_opt {
                                    let _ = sender.send(json).await;
                                } else {
                                    let err_msg = ServerMessage::Error {
                                        reason: format!("Utente '{}' non trovato o non connesso.", target_user),
                                    };

                                    if let Ok(err_json) = serde_json::to_string(&err_msg) {
                                        let _ = mpsc_tx.send(err_json).await;
                                    }
                                }
                            }
                        }

                        _ => {
                            eprintln!("Messaggio server non gestito per l'inoltro: {:?}", server_msg);
                        }
                    }
                } else {
                    eprintln!("Errore di deserializzazione dal socket: {}", line);
                }
            }
        }
    }

    if let Some(uname) = current_username {
        let senders: Vec<_> = state.with_state(|st| st.online_users.values().map(|ci| ci.sender.clone()).collect());

        state.with_state_mut(|st| {
            st.online_users.remove(&uname);
            st.user_trackers.remove(&uname);
        });

        let disc_msg = ServerMessage::UserStatus {
            username: uname.clone(),
            state: UserState::Sconnesso,
        };

        if let Ok(json) = serde_json::to_string(&disc_msg) {
            for sender in senders {
                let _ = sender.send(json.clone()).await;
            }
        }

        println!("Utente {} disconnesso e rimosso dagli utenti online.", uname);
    }
}