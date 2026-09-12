use crate::db::positions::get_positions_by_user;
use crate::db::users::get_all_users;
use crate::network::app_state::AppState;
use crate::network::protocol::ServerMessage;
use std::io;
use tokio::task;

// Avvia la console.
// Comandi disponibili:
// broadcast <msg>          → invia un testo a tutti i client connessi
// msg <utente> <messaggio> → invia un messaggio privato a un client specifico
// stats                    → mostra le statistiche del server (utenti, posizioni)
// help                     → mostra la lista dei comandi disponibili
pub fn start_server_console(state: AppState) {
    task::spawn_blocking(move || {
        println!("Console server pronta. Digita 'help' per vedere i comandi disponibili.");
        let stdin = io::stdin();
        let mut line = String::new();

        loop {
            line.clear();
            match stdin.read_line(&mut line) {
                Ok(0) => break, // EOF
                Err(e) => {
                    eprintln!("Errore lettura stdin: {}", e);
                    break;
                }
                Ok(_) => {}
            }

            let input = line.trim();

            if input.starts_with("broadcast ") {
                // BROADCAST
                let msg_text = input[10..].to_string();
                let server_msg = ServerMessage::BroadcastText {
                    from: "SERVER".to_string(),
                    text: msg_text,
                };
                if let Ok(json) = serde_json::to_string(&server_msg) {
                    let senders: Vec<_> =
                        state.with_state(|st| st.online_users.values().map(|ci| ci.sender.clone()).collect());
                    let count = senders.len();
                    for sender in senders {
                        let _ = sender.blocking_send(json.clone());
                    }
                    println!("✉ Messaggio broadcast inviato a {} utenti.", count);
                }
            } else if input.starts_with("msg ") {
                // MESSAGGIO PRIVATO
                let parts: Vec<&str> = input.splitn(3, ' ').collect();
                if parts.len() == 3 {
                    let target_user = parts[1];
                    let msg_text = parts[2].to_string();
                    let server_msg = ServerMessage::DirectText {
                        target_user: target_user.to_string(),
                        from: "SERVER".to_string(),
                        text: msg_text,
                    };
                    if let Ok(json) = serde_json::to_string(&server_msg) {
                        let sender_opt =
                            state.with_state(|st| st.online_users.get(target_user).map(|ci| ci.sender.clone()));
                        if let Some(sender) = sender_opt {
                            let _ = sender.blocking_send(json);
                            println!("✉ Messaggio privato inviato a {}.", target_user);
                        } else {
                            println!("⚠ Utente '{}' non trovato o non online.", target_user);
                        }
                    }
                } else {
                    println!("Formato errato. Usa: msg <utente> <messaggio>");
                }
            } else if input == "stats" {
                // STATISTICHE SERVER
                print_stats(&state);
            } else if input == "help" {
                // HELP
                println!("\nComandi disponibili:");
                println!("  broadcast <msg>          → invia un testo a tutti i client connessi");
                println!("  msg <utente> <messaggio> → invia un messaggio privato");
                println!("  stats                    → mostra le statistiche del server");
                println!("  help                     → mostra questo aiuto\n");
            } else if !input.is_empty() {
                println!(
                    "Comando non riconosciuto. Usa 'help' per vedere i comandi disponibili."
                );
            }
        }
    });
}

// Stampa le statistiche del server:
// - utenti attualmente online
// - totale utenti registrati nel DB
// - totale posizioni registrate nel DB (per ogni utente)
fn print_stats(state: &AppState) {
    // Utenti online (dallo stato in memoria, nessun lock DB)
    let (online_count, online_list): (usize, Vec<String>) = state.with_state(|st| {
        let names: Vec<String> = st.online_users.keys().cloned().collect();
        (names.len(), names)
    });

    println!("STATISTICHE SERVER GEORUST");
    println!("Utenti online: {}", online_count);

    if online_list.is_empty() {
        println!("(nessun client connesso)");
    } else {
        for name in &online_list {
            println!("→ {:<43}", name);
        }
    }

    // Statistiche dal DB (accesso bloccante — siamo già in spawn_blocking)
    let db_arc = state.db_arc();
    let conn = match db_arc.lock() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Impossibile acquisire il lock DB per le stats: {}", e);
            println!("⚠ Impossibile leggere le statistiche dal DB.");
            return;
        }
    };

    match get_all_users(&conn) {
        Ok(users) => {
            println!("Utenti registrati: {}", users.len());

            let mut total_positions: usize = 0;
            for user in &users {
                let pos_count = get_positions_by_user(&conn, &user.username)
                    .map(|v| v.len())
                    .unwrap_or(0);
                total_positions += pos_count;
                println!("{:<20} {:>5} posizioni registrate", user.username, pos_count);
            }
            println!("Posizioni totali nel DB: {}", total_positions);
        }
        Err(e) => {
            eprintln!("Errore lettura utenti dal DB: {}", e);
            println!("⚠ Errore nella lettura degli utenti dal DB.");
        }
    }
}
