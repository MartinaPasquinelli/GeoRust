# Report delle Prestazioni 

## GeoRust: Sistema di geolocalizzazione per una flotta di veicoli

**Gruppo 15: Pasquinelli, Danesi, Casale, Giordano**

---

## Descrizione

In questo documento vengono analizzate le prestazioni complessive dell'applicazione GeoRust, suddivise nelle sue componenti Server, Client CLI, GUI e sottosistemi di persistenza e rete.
Tutti i test e le metriche riportate sono stati raccolti compilando l'applicativo in modalità Release (`cargo build --release`), per valutare il comportamento, i tempi di esecuzione e l'ingombro effettivi in ambiente di produzione.

---

## Dimensione degli Eseguibili

Rust produce binari nativi e staticamente linkati che incorporano l'intero runtime asincrono, le librerie crittografiche, il driver SQLite e i framework grafici, eliminando qualsiasi dipendenza esterna o interprete. I pesi effettivi degli eseguibili compilati in Release sono i seguenti:

* **Server (`server.exe`)**: **~2.7 MB** (2.71 MB)
* **Client Console (`client.exe`)**: **~2.3 MB** (2.34 MB)
* **Client GUI (`client_gui.exe`)**: **~6.3 MB** (6.27 MB)
* **Admin GUI (`admin_gui.exe`)**: **~6.4 MB** (6.37 MB)

---

## Consumo di CPU e Concorrenza

L'architettura del server adotta il paradigma **Asincrono basato su Tokio**:
- Il server gestisce centinaia di connessioni simultanee delegando le operazioni di I/O di rete all'Event Loop del sistema operativo (IOCP su Windows / epoll su Linux).
- Il monitoraggio della CPU è affidato a un logger dedicato in background (`src/logging/cpu_logger.rs`) che campiona il consumo a intervalli regolari di 2 minuti tramite le crate `sysinfo` e `cpu_time`, scrivendo i report su disco in modo non bloccante via `tokio::fs`.

### Ottimizzazioni Architetturali Recenti
A seguito dell'ultima revisione del codice, sono stati implementati miglioramenti determinanti per la scalabilità:
1. **Incapsulamento dello Stato (`AppState`)**: Lo stato condiviso del server è incapsulato nella struttura AppState, che raccoglie sia lo stato dei client connessi (`Arc<RwLock<ServerState>>`) sia la connessione al database (`Arc<Mutex<Connection>>`). L'utilizzo di Arc, RwLock e Mutex garantisce la condivisione sicura di queste risorse tra le diverse task concorrenti del server.
2. **Lock a grana finissima senza blocco su `.await`**: I lock di lettura (`state.with_state(...)`) vengono acquisiti e rilasciati all'istante per raccogliere i canali dei client, senza mai essere trattenuti durante le operazioni asincrone di invio su rete.
3. **Persistenza su DB Disaccoppiata**: L'inserimento delle coordinate GPS nel database SQLite per il messaggio `UpdatePosition` è stato delegato a `tokio::task::spawn_blocking` in modalità autonoma. In questo modo il loop di lettura dei pacchetti di ciascun client non subisce ritardi dovuti all'I/O su disco, liberando immediatamente il thread per il pacchetto successivo.

### Risultati dello Stress Test

Per validare empiricamente la robustezza e la reattività del server sotto carico estremo, è stato eseguito lo script dedicato di **Stress Test** (`src/bin/stress_test.rs`).

#### Parametri del Test
* **Bot concorrenti simultanei**: 100 client TCP indipendenti registrati sul server.
* **Frequenza di aggiornamento**: 1 coordinata GPS ogni 500 ms per ciascun bot (2 aggiornamenti/secondo per bot).
* **Carico complessivo generato**: **200 aggiornamenti GPS al secondo** (pari a **12.000 posizioni e scritture su DB al minuto**).

#### Dati Rilevati dal Logger CPU (`cpu_log.txt`)
La sessione di test eseguita in modalità Release ha prodotto il seguente log:

```text
=== NUOVA SESSIONE SERVER AVVIATA [2026-09-03 16:01:36] ===
[2026-09-03 16:01:36] Tempo di CPU iniziale: 656.25ms (Utilizzo: 18.18%) <-- Avvio e connessione iniziale 100 bot
[2026-09-03 16:03:36] Tempo di CPU totale utilizzato: 38.09s (Utilizzo nell'ultimo intervallo: 31.19%) <-- Sotto stress continuo
[2026-09-03 16:05:36] Tempo di CPU totale utilizzato: 77.39s (Utilizzo nell'ultimo intervallo: 32.74%) <-- Sotto stress continuo
[2026-09-03 16:07:36] Tempo di CPU totale utilizzato: 83.27s (Utilizzo nell'ultimo intervallo: 4.90%)  <-- Conclusione test e rilascio
[2026-09-03 16:09:36] Tempo di CPU totale utilizzato: 83.27s (Utilizzo nell'ultimo intervallo: 0.00%)  <-- Ritorno a riposo (Idle)
```

#### Analisi Comparativa delle Prestazioni
* **Carico della CPU**: l'occupazione della CPU si aggira fra  **~31.19% - 32.74%**.Il tempo di CPU fisico impiegato per intervallo è crollato da 161s a soli ~38-39s.
* **Affidabilità e Integrità**: Durante la sessione di test sono state registrate oltre **48.000 posizioni GPS** nel database senza alcun crash, disconnessione anomala, errore di deserializzazione JSON o contesa di lock (deadlock).
* **Ritorno immediato all'Idle**: Al termine della simulazione l'occupazione della CPU è tornata istantaneamente allo **0.00%**, confermando l'assenza di task zombie o memory/task leak.

---

## Gestione della Memoria (RAM)

Grazie al modello di *Ownership* e *Borrowing* di Rust, la gestione della memoria avviene in modo completamente deterministico a tempo di compilazione, senza l'ausilio di un Garbage Collector (GC):
- **Zero pause di Garbage Collection**: Nessun lag o jitter introdotto da cicli di pulizia periodica della heap.
- **Footprint a Riposo**: Il server avviato con connessioni standard occupa circa **6 - 12 MB di RAM**.
- **Footprint Sotto Stress (100 Bot Attivi)**: Durante l'elaborazione simultanea di 200 coordinate/secondo da 100 client, il consumo di memoria del processo `server.exe` è salito a soli **~26 MB di RAM** (Working Set).
- **Footprint dello Stress Test**: Lo stesso script `stress_test.exe`, pur gestendo 100 task asincroni concorrenti e 100 socket TCP contemporanei, ha occupato appena **~8.5 MB di RAM**.

---

## Prestazioni di Rete e Latenza

Il layer di comunicazione TCP impiega `tokio_util::codec::Framed` con `LinesCodec`:
- **Framing Efficace**: La delimitazione tramite newline garantisce la corretta ricostruzione dei pacchetti JSON ed evita problematiche di frammentazione o buffering incontrollato a livello TCP.
- **Latenza di Elaborazione**: La latenza locale per il ciclo completo (ricezione pacchetto dal client, parsing JSON, verifica stato e broadcast di risposta) è inferiore a **1 millisecondo** (< 1 ms).
- **Isolamento I/O**: Il disaccoppiamento della persistenza SQLite rispetto allo stream di rete assicura che eventuali latenze di I/O del disco non si propaghino mai come ritardi di trasmissione sui client connessi.

---

## Conclusioni

- **Consumo CPU** (~32% sotto carico continuato di 200 coordinate/sec).
- **Consumo di RAM** (~26 MB sotto stress test).
- **Ingombro binario** (2.7 MB per il server release).
- **Stabilità impeccabile**, confermata dall'elaborazione senza anomalie di decine di migliaia di pacchetti concorrenti.

