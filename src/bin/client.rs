use futures::{SinkExt, StreamExt};
use georust::gps::simulator::run_simulation;
use georust::network::protocol::{ClientMessage, ServerMessage};
use std::io::{self, Write};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio_util::codec::{Framed, LinesCodec};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Avvio GeoRust Client...");

    // Scelta tra Login e Registrazione
    println!("\nSeleziona un'opzione:");
    println!("Digia 1 per Login");
    println!("Digia 2 per Registrazione");
    print!("Scelta: ");
    io::stdout().flush()?;

    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;

    let is_login = match choice.trim() {
        "1" => true,
        "2" => false,
        _ => {
            eprintln!("Scelta non valida. Annullamento.");
            return Ok(());
        }
    };

    // Input credenziali
    print!("Username: ");
    io::stdout().flush()?;
    let mut username = String::new();
    io::stdin().read_line(&mut username)?;

    print!("Password: ");
    io::stdout().flush()?;

    let password = rpassword::read_password()?;

    let clean_username = username.trim().to_string();
    let clean_password = password.trim().to_string();

    // Connessione TCP
    let addr = "127.0.0.1:8080";
    println!("Tentativo di connessione al server su {}...", addr);

    let stream = match TcpStream::connect(addr).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "Errore di connessione: {}. Assicurati che il Server sia avviato!",
                e
            );
            return Ok(());
        }
    };

    println!("Connesso al server!");
    let mut framed = Framed::new(stream, LinesCodec::new());

    // Invio messaggio di Login o Registrazione
    let auth_msg = if is_login {
        ClientMessage::Login {
            username: clean_username,
            password_hash: clean_password,
        }
    } else {
        ClientMessage::Register {
            username: clean_username,
            password_hash: clean_password,
        }
    };

    let auth_json = serde_json::to_string(&auth_msg)?;
    framed.send(auth_json).await?;
    println!("Richiesta inviata al server...");

    // Verifica risposta dal Server
    if let Some(Ok(line)) = framed.next().await {
        match serde_json::from_str::<ServerMessage>(&line) {
            Ok(ServerMessage::AuthResult { success, msg }) => {
                if success {
                    println!("Successo: {}", msg);
                } else {
                    eprintln!("Operazione fallita: {}", msg);
                    return Ok(()); // Interrompe l'esecuzione se fallito
                }
            }
            Ok(other) => {
                eprintln!("Risposta inattesa dal server: {:?}", other);
                return Ok(());
            }
            Err(e) => {
                eprintln!("Errore nella lettura della risposta: {}", e);
                return Ok(());
            }
        }
    }

    // Avvio Simulatore GPS dopo il successo
    let (tx, mut rx) = mpsc::channel(10);
    let file_path = "data/route.csv";

    tokio::spawn(async move {
        if let Err(e) = run_simulation(file_path, tx, 30000).await {
            eprintln!("Errore nel simulatore: {}", e);
        }
    });

    // Avvio del task di console delegato a un modulo esterno e bloccante (thread-safe)
    let (tx_stdin, mut rx_stdin) = mpsc::channel::<String>(10);
    georust::network::client_console::start_client_console(tx_stdin);

    println!("\nPuoi scrivere messaggi da inviare al server premendo Invio!\n");

    // Loop di trasmissione coordinate e messaggi
    loop {
        tokio::select! {
            Some(text) = rx_stdin.recv() => {
                let msg = ClientMessage::SendDirectText { text };
                let json_string = serde_json::to_string(&msg)?;
                framed.send(json_string).await?;
            }
            Some((lat, lon)) = rx.recv() => {
                let msg = ClientMessage::UpdatePosition { lat, lon };
                let json_string = serde_json::to_string(&msg)?;
                framed.send(json_string).await?;
                println!("Inviata posizione al server: lat={}, lon={}", lat, lon);
            }
            result = framed.next() => {
                match result {
                    Some(Ok(line)) => {
                        if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&line) {
                            match server_msg {
                                ServerMessage::DirectText { from, text, .. } => {
                                    println!("\n[MESSAGGIO DIRETTO da {}]: {}\n", from, text);
                                }
                                ServerMessage::BroadcastText { from, text } => {
                                    println!("\n[BROADCAST da {}]: {}\n", from, text);
                                }
                                _ => {
                                    println!("Ricevuto dal Server: {:?}", server_msg);
                                }
                            }
                        }
                    }
                    Some(Err(e)) => {
                        eprintln!("Errore di rete: {}", e);
                        break;
                    }
                    None => {
                        println!("Il Server ha chiuso la connessione.");
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}
