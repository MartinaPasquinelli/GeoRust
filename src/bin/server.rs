use georust::db::init_db;
use georust::logging::cpu_logger::start_cpu_logger;
use georust::network::app_state::AppState;
use georust::network::handler::handle_client;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Avvio GeoRust Server...");

    // Avvio task in background per logging CPU
    start_cpu_logger().await;

    // Inizializza il DB e crea l'AppState che gestisce internamente Arc<Mutex<Connection>>
    //    e Arc<RwLock<ServerState>> — non è più necessario gestirli separatamente.
    let conn = init_db("georust.db")?;
    let state = AppState::new(conn);
    println!("Database SQLite connesso con successo!");

    // Avvia il listener TCP
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    println!("TCP Listener avviato. In attesa di connessioni su {}...", addr);
    println!("Comandi disponibili: 'broadcast <msg>', 'msg <user> <msg>', 'stats', 'help'");

    // Avvia la console interattiva del server (thread separato, non bloccante)
    georust::network::server_console::start_server_console(state.clone());

    // Loop principale: accetta connessioni e le gestisce in task separati
    loop {
        let (stream, socket_addr) = listener.accept().await?;
        println!("Nuova connessione in ingresso da: {}", socket_addr);

        // AppState è Clone (solo incremento ref count degli Arc interni)
        let state_clone = state.clone();

        tokio::spawn(async move {
            handle_client(stream, state_clone).await;
        });
    }
}