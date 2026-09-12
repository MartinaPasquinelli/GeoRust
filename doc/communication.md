# Architettura e Protocolli di Comunicazione

## GeoRust: Sistema di geolocalizzazione per una flotta di veicoli

**Gruppo 15: Pasquinelli, Danesi, Casale, Giordano**

---
## Descrizione

Questo documento descrive in dettaglio l'architettura di comunicazione dell'intero ecosistema GeoRust. Illustra come tutti i moduli applicativi, i binari (Server, Client CLI, Client GUI, Admin GUI), il database SQLite, il simulatore GPS e i logger di sistema interagiscono tra loro in modo concorrente, asincrono e thread-safe, minimizzando il consumo di CPU e garantendo la massima reattività.

---

## Panoramica Architetturale Generale

Il sistema è strutturato come un'architettura **Client/Server distribuita**, orchestrata tramite il runtime asincrono **Tokio** (multi-thread) e basata su protocollo di trasporto **TCP**.

I componenti fondamentali dell'ecosistema comunicano secondo precisi canali di interconnessione:
- **Server Centrale (`georust_server`)**: gestisce il ciclo di ascolto su porta TCP (`127.0.0.1:8080`), spawna un task asincrono dedicato (`handle_client`) per ogni connessione, mantiene lo stato in memoria protetto da `RwLock` e delega le operazioni bloccanti verso SQLite e console a thread dedicati (`spawn_blocking`).
- **Client Console CLI (`client`)**: un singolo thread asincrono cooperativo che impiega `tokio::select!` per multiplexare contemporaneamente l'input da tastiera (tramite canale MPSC), le coordinate generate dal simulatore GPS e la ricezione di messaggi TCP dal server.
- **Client ad Interfaccia Grafica (`client_gui`)**: architettura ibrida in cui il thread principale della finestra grafica (`egui`/`eframe` a 60 FPS) comunica con un runtime Tokio dedicato in background tramite code non bloccanti (`unbounded_channel` con `try_recv`).
- **Console Amministratore Flotta (`admin_gui`)**: riceve i messaggi in tempo reale dal server tramite connessione TCP autenticata come `ADMIN_CONSOLE` e interroga in parallelo il database locale `georust.db` per analizzare percorsi, velocità e soste dei veicoli.

I punti cardine di questa infrastruttura sono:
1. **Trasporto TCP con Framing a Linee (`LinesCodec`)**: stream orientato a byte convertito trasparentemente in singoli messaggi atomici.
2. **Messaggistica JSON Tipizzata (`ClientMessage` / `ServerMessage`)**: parsing e serializzazione sicuri tramite `serde`.
3. **Disaccoppiamento tramite Canali Tokio (`mpsc`)**: assenza di lock di rete contesi tra task concorrenti.
4. **Isolamento dell'I/O Bloccante (`tokio::task::spawn_blocking`)**: chiamate al DB SQLite e lettura della tastiera (`stdin`) delegate al thread-pool dedicato di Tokio per non arrestare l'event-loop asincrono.

---

## Il Protocollo Applicativo su TCP (`src/network/protocol.rs`)

### Il Problema del Framing su TCP e la Soluzione `LinesCodec`
TCP è un protocollo di trasporto a stream continuo di byte: non preserva nativamente i confini dei messaggi applicativi. Due pacchetti inviati in rapida successione possono arrivare fusi in un'unica lettura (*packet coalescence*) oppure un singolo pacchetto può arrivare frammentato (*packet fragmentation*).

Per risolvere questo problema a costo computazionale nullo, GeoRust adotta il wrapper:
```rust
tokio_util::codec::Framed<TcpStream, LinesCodec>
```
`LinesCodec` individua il carattere newline `\n` come delimitatore di fine messaggio:
- In ricezione (`StreamExt::next`): accumula i byte in un buffer interno ed emette un elemento solo quando incontra un `\n` valido, restituendo la stringa decodificata priva del delimitatore.
- In trasmissione (`SinkExt::send`): appende automaticamente `\n` in coda al payload JSON prima di trasmetterlo sul socket.

### Tipi di Messaggi: `ClientMessage`
Messaggi inviati dai vari client (CLI, GUI, Admin) verso il Server:

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientMessage {
    // Richiesta di autenticazione con credenziali
    Login { username: String, password_hash: String },
    // Richiesta di registrazione nuovo account
    Register { username: String, password_hash: String },
    // Comando per impostare una simulazione parametrizzata
    StartSimulation { lat: f64, lon: f64, speed: f64 },
    // Invio periodico delle coordinate geografiche rilevate dal GPS
    UpdatePosition { lat: f64, lon: f64 },
    // Messaggio di testo destinato alla chat pubblica (broadcast)
    SendText { text: String },
    // Messaggio di testo privato indirizzato a un utente specifico
    SendPrivateText { to: String, text: String },
    // Notifica esplicita di chiusura della sessione
    Disconnect,
}
```

### Tipi di Messaggi: `ServerMessage`
Messaggi inviati dal Server verso uno o più client:

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerMessage {
    // Esito di una richiesta di login o registrazione
    AuthResult { success: bool, msg: String },

    // Notifica generica (esito operazione, messaggi del server)
    GenericResponse { success: bool, msg: String },

    // Messaggio chat in arrivo (mittente e corpo)
    Text { from: String, text: String },

    // Notifica broadcast che un utente si è disconnesso
    UserDisconnected { username: String },

    // Errore applicativo o di protocollo
    Error(String),
}
```

### Esempi di Frame di Rete Scambiati (Wire Format)
Ciascun frame è una stringa JSON valida terminata da `\n`:

| Direzione | Descrizione | Payload di Rete (Wire JSON) |
| :--- | :--- | :--- |
| **C → S** | Login | `{"Login":{"username":"driver1","password_hash":"segreto123"}}` |
| **S → C** | Esito Login OK | `{"AuthResult":{"success":true,"msg":"Login effettuato con successo!"}}` |
| **C → S** | Invio Posizione | `{"UpdatePosition":{"lat":45.0703,"lon":7.6869}}` |
| **C → S** | Chat Globale | `{"SendText":{"text":"Traffico intenso sulla tangenziale"}}` |
| **S → C** | Inoltro Broadcast | `{"Text":{"from":"driver1","text":"Traffico intenso sulla tangenziale"}}` |
| **S → C** | Notifica Privata | `{"Text":{"from":"SERVER (Privato)","text":"Rientra alla base per rifornimento"}}` |
| **C → S** | Disconnessione | `"Disconnect"` |

---

## Architettura del Server (`server.rs` e `src/network/*`)

Il server (`src/bin/server.rs`) opera su architettura asincrona multi-thread supportata dal runtime Tokio (`#[tokio::main]`). Il ciclo principale si pone in ascolto sulla porta configurata (`127.0.0.1:8080`) tramite `TcpListener::bind`. A ogni connessione in arrivo catturata da `listener.accept().await`, il server istanzia un task indipendente via `tokio::spawn(handle_client(stream, state.clone()))`, garantendo che ciascun client sia servito in parallelo senza bloccare l'accettazione di ulteriori connessioni.

### Lo Stato Condiviso: `AppState` e `ServerState` (`app_state.rs`, `state.rs`)
La sincronizzazione in memoria del server è centralizzata nella struttura `AppState`:

```rust
#[derive(Clone)]
pub struct AppState {
    state: Arc<RwLock<ServerState>>,
    db: Arc<Mutex<Connection>>,
}

#[derive(Default)]
pub struct ServerState {
    pub online_users: HashMap<String, ClientSender>,
}

pub type ClientSender = mpsc::Sender<String>;
```

#### Pattern Closure per la Prevenzione dei Deadlock
Per impedire il pericolo comune nei sistemi asincroni di trattenere una lock oltre un punto di sospensione (`.await`), `AppState` espone metodi basati su closure:
```rust
pub fn with_state<F, R>(&self, f: F) -> R
where
    F: FnOnce(&ServerState) -> R,
{
    let guard = self.state.read().expect("ServerState RwLock avvelenato");
    f(&guard)
} // La lock viene rilasciata istantaneamente al termine della closure, MAI tenuta su un .await!
```
Esempio di utilizzo durante un broadcast in `handler.rs`:
```rust
// Clona i canali tenendo la lock solo per pochi microsecondi
let senders: Vec<_> = state.with_state(|st| st.online_users.values().cloned().collect());
// Invia asincronamente sui canali DOPO aver rilasciato la lock
for sender in senders {
    let _ = sender.send(json.clone()).await;
}
```

### Il Modello del Singolo Client: `handle_client` (`handler.rs`)
Per ogni connessione TCP accettata, viene creata una task dedicata `handle_client`, organizzata come segue:

1. **Split del socket**: `framed.split()` divide la connessione in un sink di scrittura (`tx`) e uno stream di lettura (`rx`).
2. **Creazione del Canale MPSC Dedicato**: `let (mpsc_tx, mut mpsc_rx) = mpsc::channel::<String>(32);`.
3. **Writer Task Asincrono Indipendente**:
   ```rust
   tokio::spawn(async move {
       while let Some(msg) = mpsc_rx.recv().await {
           if tx.send(msg).await.is_err() {
               break; // Socket chiuso: arresto immediato del writer
           }
       }
   });
   ```
   Questo design assicura che qualsiasi altro modulo (la console del server, un altro client via broadcast, o un messaggio privato) possa inviare una riga a questo client semplicemente invocando `mpsc_tx.send(json).await`, senza mai toccare direttamente il socket di rete.
4. **Reader Loop**: analizza i messaggi in ingresso riga per riga, deserializzando i pacchetti `ClientMessage`.
5. **Disconnessione e Pulizia**: all'uscita dal ciclo (chiusura socket o `ClientMessage::Disconnect`), il nome utente viene rimosso in modo atomico da `online_users`:
   ```rust
   if let Some(uname) = current_username {
       state.with_state_mut(|st| {
           st.online_users.remove(&uname);
       });
   }
   ```

### Integrazione Asincrona con il Database SQLite (`rusqlite`)
La libreria `rusqlite` opera in modalità bloccante (accesso diretto al filesystem). Se invocata direttamente nel contesto di un thread asincrono Tokio, saturerebbe i worker-thread impedendo la gestione delle altre connessioni di rete.

GeoRust adotta due strategie mirate:

#### A) Chiamata Attesa con `spawn_blocking` (Login & Register)
Poiché l'esito dell'operazione è indispensabile per decidere la risposta (`AuthResult`), il server delega la chiamata al thread-pool dedicato ai task bloccanti e ne attende il risultato:
```rust
let auth_result = task::spawn_blocking(move || {
    let conn = db_arc.lock().expect("DB Mutex avvelenato");
    login_user(&conn, &credentials)
}).await;
```

#### B) Pattern **Fire-and-Forget** ad Alte Prestazioni (`UpdatePosition`)
Durante la ricezione delle coordinate GPS (fino a centinaia al secondo), attendere il completamento della scrittura su disco bloccherebbe il loop del client. GeoRust disaccoppia completamente la persistenza:
```rust
// Il reader task rilascia immediatamente l'esecuzione e torna ad ascoltare il socket!
let _handle = task::spawn_blocking(move || {
    let conn = db_arc.lock().expect("DB Mutex avvelenato");
    let _ = insert_position(&conn, &uname_clone, lat, lon);
});
```
Il Mutex interno `Arc<Mutex<Connection>>` garantisce che le scritture su SQLite rimangano rigorosamente serializzate e protette, azzerando la latenza di risposta di rete.

### La Console Amministrativa Interattiva (`server_console.rs`)
Il server avvia un task di amministrazione da terminale:
```rust
pub fn start_server_console(state: AppState);
```
- Eseguita in `task::spawn_blocking` per non interferire con il runtime asincrono.
- Legge bloccante da `std::io::stdin()`.
- Utilizza `sender.blocking_send(json)` per recapitare messaggi broadcast o privati nei canali `mpsc` dei rispettivi client connessi.
- Implementa il comando `stats` per interrogare e aggregare in tempo reale lo stato dei client in memoria e i record storici del database.

---

## Architettura del Client Console CLI (`src/bin/client.rs`)

Il client CLI combina tre flussi asincroni concorrenti in un singolo thread cooperativo mediante la macro `tokio::select!`:

1. **Flusso Input Utente (`client_console.rs`)**:
   Un task bloccante dedicato legge le righe da tastiera e le riversa su un canale `tokio::sync::mpsc::channel<String>(10)`. All'arrivo di una stringa, il loop principale la trasforma in `ClientMessage::SendText` e la invia al socket.
2. **Flusso Simulatore GPS (`gps/simulator.rs`)**:
   Un task asincrono legge il percorso dal file CSV (`data/route.csv`), usa il timer precisissimo `tokio::time::interval(30000)` e spinge una coppia `(lat, lon)` ogni 30 secondi su un canale `mpsc::channel(10)`. All'arrivo della coordinata, il client invia `ClientMessage::UpdatePosition`.
3. **Flusso Ricezione Rete**:
   In attesa asincrona di pacchetti `ServerMessage` in arrivo dal server tramite `framed.next()`. Quando giunge un messaggio (es. chat broadcast o privata), viene stampato a terminale su `stdout`.

Grazie a `tokio::select!`, il processo rimane completamente dormiente a consumo CPU prossimo allo 0%, risvegliandosi unicamente nel momento in cui una delle tre sorgenti produce un evento.

---

## Architettura del Client ad Interfaccia Grafica (`src/bin/client_gui.rs`)

### Il Problema: GUI Immediata (`egui`) vs Networking Asincrono (`Tokio`)
La libreria `egui` adotta il paradigma Immediate-Mode GUI: il metodo `eframe::App::update` viene invocato a ogni frame di rendering (~60 volte al secondo) sul thread principale della finestra grafica dell'OS. Non può essere asincrono né può eseguire operazioni bloccanti, pena il congelamento istantaneo dell'interfaccia.

### La Soluzione: Runtime Tokio Dedicato e Canali Bridge
`ClientApp` inizializza all'avvio un **Runtime Tokio multi-thread dedicato in background**:
```rust
let runtime = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .worker_threads(2)
    .thread_name("georust-client-rt")
    .build()
    .expect("Impossibile avviare il runtime Tokio in background");
```

La comunicazione tra il thread della UI e i task asincroni di rete/GPS avviene mediante canali **`tokio::sync::mpsc::unbounded_channel`**:
- **Dalla Rete alla GUI (`rx_from_server`)**: il task asincrono di lettura TCP legge i `ServerMessage` e li spinge nel canale unbounded. A ogni frame, la GUI li preleva per aggiornare il pannello della chat.
- **Dal Simulatore GPS alla GUI (`rx_gps`)**: il task GPS legge `route.csv`, invia la posizione al server via TCP e la passa anche alla GUI tramite canale per visualizzare le coordinate correnti e lo stato.
- **Dalla GUI alla Rete (`tx_to_server`)**: quando l'utente digita un messaggio nella finestra grafica e preme invio, la GUI invia il testo sul canale, prelevato dal task asincrono di scrittura che lo trasmette sul socket TCP.

#### Perché `try_recv()` invece di `recv().await`?
All'interno del loop `update()`, la GUI invoca:
```rust
while let Ok(line) = rx.try_recv() {
    // Processa i messaggi accumulati nel buffer
}
```
Se non ci sono messaggi in coda, `try_recv()` restituisce immediatamente `Err(TryRecvError::Empty)` senza bloccare il frame di rendering.

### Controllo Sosta Concorrente con `Arc<AtomicBool>`
Per consentire all'utente di simulare un veicolo fermo tramite il pulsante **"Simula Fermo"**, viene utilizzato un puntatore atomico lock-free condiviso tra il thread GUI e il task asincrono GPS:
```rust
is_stopped: Arc<AtomicBool>
```
- Il thread GUI commuta il valore con `is_stopped.store(val, Ordering::Relaxed)` al click del pulsante.
- Il task GPS legge il valore ad ogni ciclo di 30 secondi con `is_stopped.load(Ordering::Relaxed)`. Se attivo, re-invia l'ultima coordinata nota invece di avanzare nel percorso, simulando una sosta prolungata.

---

## Architettura della Console Amministratore Flotta (`src/bin/admin_gui.rs`)

La console amministrativa (`AdminApp`) presenta un'architettura ibrida a doppio canale:

1. **Canale di Rete TCP (Server)**:
   - Utilizza lo stesso pattern a runtime Tokio dedicato e canali `mpsc unbounded` del client GUI.
   - Si autentica con credenziali speciali: `username: "ADMIN_CONSOLE"`, bypassando i controlli password standard del server.
   - Riceve in tempo reale tutti i messaggi broadcast della flotta e può indirizzare messaggi privati o broadcast a qualsiasi veicolo.
2. **Accesso Diretto Locale a SQLite (`georust.db`)**:
   - Possiede una connessione diretta a SQLite (`rusqlite::Connection`).
   - Esegue query dirette sulle tabelle `users` e `positions` per ricostruire l'intera flotta veicoli, visualizzandone coordinate attuali e stato.
   - Esegue le analisi di movimento (`gps::analytics` e `UserTracker`):
     * Formula di **Haversine** per il calcolo chilometrico su sfera terrestre.
     * **Soglia di movimento a 5 metri** per filtrare il rumore del sensore GPS.
     * Timeout di sosta a **180 secondi** per dichiarare un veicolo da `InMovimento` a `Fermo`.
3. **Monitoraggio Prestazionale CPU**:
   - Legge periodicamente il file di log `cpu_log.txt` prodotto dal demone `logging::cpu_logger`, visualizzando le percentuali di carico della CPU del server.

---

## Il Modulo di Monitoraggio CPU (`src/logging/cpu_logger.rs`)

Il server monitora autonomamente il proprio impatto sulle risorse hardware:
- Avviato come task asincrono indipendente all'inizio del server (`tokio::spawn`).
- All'avvio effettua un **doppio campionamento distanziato di 500 ms** tramite la crate `sysinfo`. Questo passaggio è fondamentale in quanto i moderni sistemi operativi richiedono un delta temporale per calcolare una percentuale CPU diversa da 0% o 100%.
- Ogni 120 secondi si risveglia tramite `tokio::time::sleep(Duration::from_secs(120))` ed effettua un append asincrono sul file `cpu_log.txt` tramite `tokio::fs::OpenOptions` e `write_all`.

---

## Tassonomia dei Canali e Primitivi di Sincronizzazione

| Primitiva | Modulo / File | Dimensione Buffer / Natura | Motivazione Tecnica |
| :--- | :--- | :--- | :--- |
| **`tokio::sync::mpsc` (Bounded)** | `handler.rs` | Buffer: 32 stringhe | Fornisce contropressione (*backpressure*): se un client rallenta la ricezione, evita saturazione della RAM del server. |
| **`tokio::sync::mpsc` (Bounded)** | `simulator.rs` | Buffer: 10 coordinate | Disaccoppia la lettura CSV dal trasmettitore di rete con garanzia di buffering limitato. |
| **`tokio::sync::mpsc` (Bounded)** | `client_console.rs` | Buffer: 10 stringhe | Bufferizza le righe digitate da tastiera prima dell'invio sul socket. |
| **`tokio::sync::mpsc` (Unbounded)** | `client_gui.rs`, `admin_gui.rs` | Illimitato | Evita il blocco sul thread di rendering della GUI durante le chiamate di invio/ricezione messaggi. |
| **`std::sync::RwLock`** | `app_state.rs` (`ServerState`) | Lock lettori multipli / singolo scrittore | Massimizza la concorrenza in lettura della mappa utenti; mai trattenuto attraverso `.await`. |
| **`std::sync::Mutex`** | `app_state.rs` (`rusqlite::Connection`) | Esclusione mutua sincrona | SQLite richiede serializzazione degli accessi; lock sempre racchiuso dentro `spawn_blocking`. |
| **`std::sync::atomic::AtomicBool`** | `client_gui.rs` | Lock-free atomico | Scambio dello stato di sosta tra thread GUI e task GPS senza overhead di lock o context-switch. |
| **`tokio::select!`** | `bin/client.rs` | Multiplexer di eventi asincroni | Consente al client CLI di reagire istantaneamente a Stdin, GPS o Rete senza polling attivo. |
| **`tokio_util::codec::Framed`** | `handler.rs`, `client.rs`, GUI | Framing a righe (`LinesCodec`) | Elimina la complessità della segmentazione TCP garantendo l'integrità dei pacchetti JSON. |

---

## Flusso Sequenziale di una Sessione Tipica

Il ciclo vitale di una sessione client/server si articola nei seguenti passaggi:

1. **Connessione TCP e Setup Task**:
   Il client (CLI o GUI) stabilisce la connessione TCP con il server sulla porta `8080`. Il listener del server accetta la connessione e avvia un task `handle_client`. La connessione viene incapsulata in `Framed<TcpStream, LinesCodec>` e divisa in stream di lettura e sink di scrittura. Viene creato un canale bounded `mpsc::channel(32)` e avviato un task dedicato in background (`writer task`) che preleva le stringhe dalla coda e le scrive sul socket.

2. **Autenticazione (`Login`)**:
   Il client invia il frame serializzato `ClientMessage::Login { username, password_hash }`. Il reader task del server intercetta il messaggio e invoca `task::spawn_blocking` per interrogare la funzione bloccante `login_user` su SQLite. Alla ricezione del risultato positivo, il server registra il mittente e il suo `mpsc_tx` nella mappa in memoria `ServerState::online_users` protetta da `RwLock`. Il server inoltra quindi `ServerMessage::AuthResult { success: true, ... }` al canale del writer task, che lo trasmette sulla socket verso il client.

3. **Aggiornamento Telemetria GPS (`UpdatePosition`)**:
   Il simulatore GPS del client genera nuove coordinate a intervalli regolari, inviando `ClientMessage::UpdatePosition { lat, lon }`. Il reader task del server riceve il frame e delega l'operazione di inserimento nel database a un task bloccante (`task::spawn_blocking`) con approccio *fire-and-forget*, rilasciando immediatamente il reader per continuare ad ascoltare nuovi messaggi di rete senza accumulare latenza.

4. **Scambio Messaggi Chat (`SendText`)**:
   Il client invia una stringa di testo destinata alla chat comune (`ClientMessage::SendText`). Il reader task del server accede in sola lettura ad `AppState` tramite la closure sicura `with_state`, estrae e clona i sender di tutti gli utenti online, rilascia istantaneamente il lock ed esegue l'invio asincrono (`mpsc_tx.send(...)`). Ciascun writer task riceve la riga e la invia al rispettivo client via TCP.

5. **Disconnessione (`Disconnect` o chiusura socket)**:
   All'arrivo del messaggio `ClientMessage::Disconnect` o in caso di caduta della connessione TCP (EOF sul socket), il reader task rimuove in modo atomico il nome utente dalla tabella `online_users` in `ServerState`. Il canale `mpsc_rx` del writer si chiude automaticamente all'uscita dal reader, provocando l'arresto pulito e simultaneo di tutti i task associati a quel client.

---

## Conclusioni e Punti di Forza dell'Architettura

1. **Assenza di Race Condition e Deadlock**: l'incapsulamento dei lock in `AppState` tramite closure immediate impedisce che un blocco sincronizzato rimanga aperto durante un'operazione di I/O asincrono.
2. **Massima Efficienza Energetica e CPU**: l'assoluta assenza di cicli di polling (`busy-waiting`), rimpiazzati da timer `interval`, sospensioni `sleep` asincrone e multiplexing `select!`, garantisce un consumo CPU a riposo prossimo allo **0.0%**.
3. **Resilienza ai Crash di Rete**: la separazione tra socket di lettura e task di scrittura evita blocchi a catena nel server: la rottura di una pipe TCP interrompe istantaneamente i relativi task locali e rimuove l'utente dalle strutture condivise senza impattare gli altri client connessi.
