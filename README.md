# GeoRust: Sistema di geolocalizzazione per una flotta di veicoli

**Gruppo 15: Pasquinelli, Danesi, Casale, Giordano**

---

## Descrizione

GeoRust è un'applicazione **client/server** sviluppata interamente in **Rust** per simulare e monitorare in tempo reale una flotta di veicoli. Gli autisti si registrano, si autenticano e inviano periodicamente la propria posizione GPS; il server memorizza lo storico degli spostamenti, ne determina automaticamente lo stato (sconnesso, fermo, in movimento), fornisce analisi su distanza e velocità e gestisce una chat testuale (broadcast o privata) tra amministratore e veicoli.
Il progetto è costruito su **Tokio** per la concorrenza asincrona e su **SQLite** (tramite `rusqlite`) per la persistenza dei dati, con particolare attenzione al contenimento del consumo di CPU, alla dimensione degli eseguibili e alla stabilità sotto carico.

---

## Funzionalità principali

- Registrazione e login con password sottoposte ad **hashing SHA-256 con salt**
- Invio automatico della posizione GPS ogni **30 secondi** (simulata da file CSV)
- Determinazione automatica dello stato del veicolo:
  - **Sconnesso** → nessun aggiornamento da oltre 60s
  - **Fermo** → posizione invariata da almeno 3 minuti
  - **In movimento** → variazione di posizione superiore a 5 metri
- Analisi storiche degli spostamenti (oggi, settimana corrente, mese corrente, tutto lo storico): distanza totale, velocità media, tempo di movimento/sosta
- Messaggistica testuale **broadcast** o **privata** tra server/amministratore e veicoli
- Logging del consumo CPU del server ogni 2 minuti
- Interfacce multiple: client CLI, client GUI, console amministratore GUI

---

## Architettura

Il sistema segue un'architettura **client/server** basata su TCP, con un server centrale che gestisce ogni connessione tramite una task asincrona indipendente (`tokio::spawn`).

**Componenti principali:**

- **Server** — ascolta su `127.0.0.1:8080`, gestisce lo stato condiviso (`Arc<RwLock<ServerState>>`) e l'accesso al database (`Arc<Mutex<Connection>>`) incapsulati in `AppState`
- **Client CLI** — thread unico che multiplexa input da tastiera, simulatore GPS e rete tramite `tokio::select!`
- **Client GUI** (`egui`/`eframe`) — runtime Tokio dedicato in background, comunicazione con la UI tramite canali `mpsc` non bloccanti (`try_recv()`)
- **Admin GUI** — stessa architettura del client GUI, con accesso diretto e in sola lettura al database per statistiche e analisi di flotta

**Protocollo di comunicazione:** messaggi JSON delimitati da newline (`LinesCodec` di `tokio-util`), tipizzati tramite gli enum `ClientMessage` e `ServerMessage` (login, registrazione, aggiornamento posizione, invio testo, disconnessione, ecc.).

**Scelte tecniche rilevanti:**

- Operazioni SQLite sempre isolate in `tokio::task::spawn_blocking`, con attesa del risultato per login/registrazione e pattern *fire-and-forget* per l'inserimento delle posizioni GPS
- Distanza tra coordinate calcolata con la **formula di Haversine**
- Password protette con **SHA-256 + salt casuale** (compromesso tra sicurezza e basso carico computazionale)
- Codice organizzato in moduli separati per favorire manutenibilità ed estensione

### Struttura del codice

```src/
├── auth/
│   ├── hash.rs              → hashing e verifica delle password (SHA-256 + salt)
│   ├── login.rs             → autenticazione degli utenti registrati
│   └── register.rs          → registrazione di nuovi utenti
│
├── db/
│   ├── connection.rs        → inizializzazione e schema del database SQLite
│   ├── users.rs             → query su utenti (inserimento, ricerca, elenco)
│   └── positions.rs         → query su posizioni GPS (inserimento, storico)
│
├── gps/
│   ├── analytics.rs         → calcolo distanze, filtri temporali, analisi movimento
│   ├── simulator.rs         → simulazione GPS da file CSV
│   └── state.rs             → macchina a stati del veicolo
│
├── network/
│   ├── app_state.rs         → stato condiviso del server (AppState)
│   ├── handler.rs           → gestione della connessione di ogni singolo client
│   ├── protocol.rs          → definizione dei messaggi ClientMessage/ServerMessage
│   ├── server_console.rs    → console interattiva del server (broadcast, msg, stats)
│   ├── client_console.rs    → gestione input da tastiera lato client CLI
│   └── state.rs             → mappa degli utenti online (ServerState)
│
├── logging/
│   └── cpu_logger.rs        → campionamento e log periodico del consumo CPU
│
├── bin/
│   ├── server.rs            → eseguibile del server TCP
│   ├── client.rs            → eseguibile del client CLI
│   ├── client_gui.rs        → eseguibile del client con interfaccia grafica
│   ├── admin_gui.rs         → eseguibile della console amministratore
│   └── stress_test.rs       → script di test di carico
│
├── models.rs                → strutture dati condivise 
└── errors.rs                → tipi di errore applicativi 
```

---

## Requisiti

- Sistema operativo **Windows** o **Linux** (testato su entrambi, oltre a macOS)
- Toolchain **Rust** installata (`cargo`)
- Il server deve essere avviato **prima** di qualsiasi client

---

## Avvio rapido

**1. Avviare il server** (componente centrale, va lanciato per primo):

```bash
cargo run --bin server
```

Il server crea automaticamente il database `georust.db` se non esiste e si mette in ascolto su `127.0.0.1:8080`.

**2. Avviare uno o più client**, in terminali separati, scegliendo la modalità preferita:

| Comando | Descrizione |
|---|---|
| `cargo run --bin client` | Client veicolo da terminale (CLI), con simulatore GPS e chat testuale |
| `cargo run --bin client_gui` | Interfaccia grafica per l'autista |
| `cargo run --bin admin_gui` | Dashboard grafica per l'amministratore della flotta |

Dalla console del server è inoltre possibile digitare comandi interattivi: `broadcast <msg>`, `msg <utente> <msg>`, `stats`, `help`.

---

## File generati dal sistema

| File | Contenuto |
|---|---|
| `georust.db` | Database SQLite con utenti e storico posizioni |
| `cpu_log.txt` | Log del consumo CPU del server, aggiornato ogni 2 minuti |
| `data/route.csv` | Sequenza di coordinate del percorso GPS simulato |

---

## Prestazioni

Dati raccolti in build **Release** (`cargo build --release`):

- **Dimensione eseguibili:** server ~2.7 MB, client CLI ~2.3 MB, client GUI ~6.3 MB, admin GUI ~6.4 MB
- **Stress test:** 100 client concorrenti, 200 aggiornamenti GPS/secondo → **~31-33% CPU**, oltre 48.000 posizioni registrate senza crash o deadlock
- **Memoria:** ~6-12 MB a riposo, ~26 MB sotto stress con 100 bot attivi
- **Latenza:** < 1 ms per ciclo completo di ricezione/broadcast su rete locale
- **Multipiattaforma:** testato su Windows, Linux e macOS

---

## Documentazione

Il progetto include documentazione dettagliata suddivisa per argomento:

- **Documento di Analisi** — requisiti funzionali e non funzionali, casi d'uso
- **Manuale del Progettista** — dettaglio implementativo e scelte architetturali per ciascun modulo
- **Manuale Utente** — guida all'installazione e all'utilizzo delle interfacce CLI/GUI
- **Architettura e Protocolli di Comunicazione** — approfondimento su protocollo di rete, concorrenza e sincronizzazione
- **Report delle Prestazioni** — metriche complete su CPU, memoria, latenza e stress test
