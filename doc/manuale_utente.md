# Manuale Utente

## GeoRust: Sistema di geolocalizzazione per una flotta di veicoli

**Gruppo 15: Pasquinelli, Danesi, Casale, Giordano**

---
## Descrizione

Nel seguente documento viene spiegato come installare, avviare e utilizzare l'applicazione GeoRust, guidando passo passo nell'esecuzione del server e dei client (sia in modalità CLI che GUI) e illustrando tutte le funzionalità disponibili sia per l'amministratore che per l'autista, con relativi comandi ed esempi pratici d'uso.

---
## Introduzione

Il sistema supporta due modalità operative:

- **Modalità CLI** (Command Line Interface): avvio e controllo da terminale, adatta ad ambienti server o test rapidi.
- **Modalità GUI** (Graphical User Interface): interfaccia grafica completa, sia per l'amministratore sia per l'autista.

---

## Requisiti di Sistema

- Sistema operativo: **Windows** o **Linux** (testato su entrambi)
- Rust toolchain installata (`cargo`)
- Il server deve essere avviato **prima** di qualsiasi client

---

## Avvio del Sistema

### Passo 1 — Avvio del Server

Il server è il componente centrale: gestisce le connessioni, il database e il routing dei messaggi. Deve essere avviato per primo, prima di qualsiasi client.

Aprire un terminale nella directory principale del progetto e digitare:

```
cargo run --bin server
```

Il server si avvia, crea il database SQLite (`georust.db`) se non esiste, e si mette in ascolto su `127.0.0.1:8080`. Verranno stampati i messaggi di avvio:

```
Avvio GeoRust Server...
Database SQLite connesso con successo!
TCP Listener avviato. In attesa di connessioni su 127.0.0.1:8080...
Comandi disponibili: 'broadcast <msg>', 'msg <user> <msg>', 'stats', 'help'
```

### Passo 2 — Avvio dei Client

Scegliere una delle modalità disponibili (CLI o GUI). Ogni client va aperto in un **terminale separato**.

---

## Modalità CLI

### Client Veicolo (terminale)

Avvia un veicolo interamente da terminale, con simulatore GPS automatico e chat testuale.

```
cargo run --bin client
```

Il client chiederà di scegliere tra Login e Registrazione, poi richiederà username e password. Una volta autenticato, inizierà automaticamente a inviare le coordinate GPS al server ogni 30 secondi. È possibile digitare messaggi di testo da inviare al server durante la sessione.

---


## Modalità GUI

### Console Amministratore

Per avviare la dashboard grafica dell'amministratore:

```
cargo run --bin admin_gui
```

### Interfaccia Veicolo (GUI)

Per avviare l'interfaccia grafica del singolo autista:

```
cargo run --bin client_gui
```

---

## Console Amministratore — Guida alle Funzionalità

### Scheda Flotta

![](/doc/img/home_admin.png)

La schermata principale mostra lo stato operativo in tempo reale di tutti i veicoli registrati. Per ogni veicolo vengono visualizzati:

- **Nome utente / veicolo**
- **Stato** (colorato): In Movimento, Fermo, Sconnesso
- **Latitudine e Longitudine** dell'ultima posizione ricevuta
- **Timestamp** dell'ultimo aggiornamento

La lista si aggiorna automaticamente ogni 2 secondi. Il pulsante **Aggiorna** forza un aggiornamento immediato.

> Un veicolo passa in stato **Sconnesso** se il server non riceve aggiornamenti di posizione da più di 60 secondi.

### Scheda Analisi Movimento

![](/doc/img/application/admin_statistiche.png)

Permette di consultare lo storico degli spostamenti di un singolo veicolo. Selezionare il nome utente nel campo di testo, scegliere l'intervallo temporale dal menu a tendina e premere **Calcola Analisi**.

Intervalli disponibili:
- **Oggi**
- **Settimana corrente**
- **Mese corrente**
- **Tutto lo storico**

I risultati mostrano:
- Tragitto totale percorso (km)
- Velocità media (km/h)
- Tempo complessivo in movimento (ore, minuti, secondi)
- Tempo complessivo di sosta (ore, minuti, secondi)

### Scheda Messaggistica

![](/doc/img/application/admin_broadcast.png)

Permette di inviare messaggi testuali ai veicoli connessi.

**Messaggio in Broadcast** — Con la spunta *"Invia in Broadcast a TUTTI i veicoli"* attiva, il messaggio viene recapitato a tutti i client attualmente connessi.

![](/doc/img/application/admin_msg_privato.png)

**Messaggio Privato** — Rimuovendo la spunta, compare il campo *Destinatario*. Inserire il nome utente del destinatario: solo quel veicolo riceverà il messaggio, garantendo la riservatezza della comunicazione.

Il log nella parte inferiore mostra la cronologia dei messaggi inviati e ricevuti durante la sessione.

### Scheda Prestazioni CPU

![](/doc/img/application/admin_CPU.png)

Visualizza le ultime 30 righe del file `cpu_log.txt` generato dal server. Mostra il consumo di CPU e il tempo totale utilizzato, aggiornato ogni 2 minuti dal server. Utile per monitorare il carico del sistema nel tempo.

---

## Interfaccia Veicolo — Guida alle Funzionalità

![](/doc/img/application/home_client.png)

### Autenticazione

All'avvio, il client mostra la schermata di autenticazione. Inserire:
- **Indirizzo server** (default: `127.0.0.1:8080`)
- **Modalità**: Login (per utenti già registrati) o Registrazione (per nuovi utenti)
- **Username** e **Password**

Premere **Connetti e Autenticati**. In caso di errore (credenziali errate, server non raggiungibile) viene mostrato un messaggio in rosso.

### Simulazione GPS

![](/doc/img/application/movimento_client.png)

Dopo l'autenticazione, il client avvia automaticamente il simulatore GPS, che legge le coordinate dal file `data/route.csv` e le invia al server ogni 30 secondi. Le ultime coordinate trasmesse vengono mostrate in tempo reale.

Il tasto **Simula Fermo** mette in pausa l'avanzamento sul percorso: il client continua a inviare l'ultima posizione nota, simulando un veicolo fermo. Il tasto diventa **Riprendi Movimento** per riprendere la simulazione.

### Chat con il Server

![](/doc/img/application/chat_client.png)

Il campo *Messaggio* permette di inviare testo al server. I messaggi ricevuti dall'amministratore (broadcast o privati) vengono visualizzati nella cronologia chat con il mittente e il tipo di messaggio (es. `[ADMIN_CONSOLE (Broadcast)]: messaggio`).

---

## Console Server da Terminale

Mentre il server è in esecuzione, è possibile digitare comandi nel terminale dove è stato avviato:

| Comando | Descrizione |
|---|---|
| `broadcast <messaggio>` | Invia un messaggio a tutti i client connessi |
| `msg <utente> <messaggio>` | Invia un messaggio privato a un utente specifico |
| `stats` | Mostra le statistiche del server (utenti online, registrati, posizioni nel DB) |
| `help` | Mostra la lista dei comandi disponibili |
---

## Messaggistica — Dimostrazione Completa

### Ricezione Broadcast lato Client

Quando l'amministratore invia un messaggio in broadcast, tutti i veicoli connessi lo ricevono immediatamente con il tag `(Broadcast)`:

- L'utente *luca* riceve il messaggio.
  ![](/doc/img/application/client2_broadcast.png)
- L'utente *driver_gui* riceve lo stesso identico messaggio.
  ![](/doc/img/application/client1_broadcast.png)

### Ricezione Messaggio Privato

Il server gestisce il routing garantendo la riservatezza:

- L'utente *luca* (destinatario corretto) riceve il messaggio con il tag `(Privato)`.
  ![](/doc/img/application/client_privato.png)
- L'utente *driver_gui* (non destinatario) non riceve nulla e la sua cronologia rimane invariata.
  ![](/doc/img/application/client2_privato.png)

---

## File Generati dal Sistema

| File | Contenuto |
|---|---|
| `georust.db` | Database SQLite con utenti e storico posizioni |
| `cpu_log.txt` | Log del consumo CPU del server, aggiornato ogni 2 minuti |
| `data/route.csv` | File con la sequenza di coordinate del percorso simulato |
