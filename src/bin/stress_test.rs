use futures::{SinkExt, StreamExt};
use georust::network::protocol::ClientMessage;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::sleep;
use tokio_util::codec::{Framed, LinesCodec};

#[tokio::main]
async fn main() {
    println!("Avvio dello STRESS TEST...");

    let num_bots = 100;
    println!("Creazione di {} bot in corso...", num_bots);

    for i in 0..num_bots {
        tokio::spawn(async move {
            let addr = "127.0.0.1:8080";

            // Tentativo di connessione
            if let Ok(stream) = TcpStream::connect(addr).await {
                let mut framed = Framed::new(stream, LinesCodec::new());

                // Fase di Registrazione del Bot
                let username = format!("bot_{}", i);
                let password = "password123".to_string();
                let reg_msg = ClientMessage::Register {
                    username: username.clone(),
                    password_hash: password.clone(),
                };

                if let Ok(json) = serde_json::to_string(&reg_msg) {
                    let _ = framed.send(json).await;
                }

                // Leggiamo e scartiamo la risposta di AuthResult per sbloccare lo stream
                let _ = framed.next().await;

                // Loop Invio continuo di coordinate
                // Invece di inviare una posizione ogni 30 secondi (come da traccia),
                // i bot inviano una posizione ogni 500 millisecondi per stressare al massimo il database
                // e i lock del server. (100 bot * 2 update/s = 200 aggiornamenti al secondo totali sul server!)
                let mut lat = 45.0 + (i as f64 * 0.001);
                let mut lon = 9.0 + (i as f64 * 0.001);

                loop {
                    sleep(Duration::from_millis(500)).await;

                    // Muoviamo leggermente il bot per registrare un cambiamento di stato
                    lat += 0.0001;
                    lon += 0.0001;

                    let update_msg = ClientMessage::UpdatePosition { lat, lon };

                    if let Ok(json) = serde_json::to_string(&update_msg) {
                        // Se l'invio fallisce (es. server crashato), il task bot si ferma silenziosamente
                        if framed.send(json).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        // Inseriamo una micro-pausa tra la creazione di un bot e l'altro per non far respingere
        // le connessioni dal sistema operativo (Handshake TCP flood)
        sleep(Duration::from_millis(15)).await;
    }

    println!("Tutti i {} bot sono stati lanciati con successo!",num_bots);
    println!("I bot stanno attualmente bombardando il server con 200 coordinate GPS al secondo!");
    println!("Lascia girare questo script e il server per 4-5 minuti, poi controlla cpu_log.txt.");
    println!("(Premi Ctrl+C per fermare lo stress test)");

    // Teniamo vivo il programma principale all'infinito per permettere ai task in background di girare
    loop {
        sleep(Duration::from_secs(60)).await;
    }
}
