# Orale — GeoRust

**Gruppo 15:** Pasquinelli, Danesi, Casale, Giordano
**Progetto:** GeoRust — Sistema di geolocalizzazione per una flotta di veicoli

> **Nota**: rispetto alla bozza originale, questa versione non si limita a elencare *cosa* è stato fatto, ma si sofferma anche sul *perché* e sul *come*, con paragoni ed esempi pensati per chi non conosce ancora bene Rust, Tokio o la programmazione asincrona. È pensata sia come discorso da esporre a voce, sia come traccia di studio da rileggere prima dell'esame. I paragrafi introdotti da 🎓 sono spiegazioni "per non addetti ai lavori": se il pubblico è già esperto, si possono anche saltare a voce e tenerli solo come rete di sicurezza per le domande.

---

## 1. Introduzione e obiettivo del progetto

>*😂"Buongiorno professore, buongiorno a tutti. <br>Oggi vi presentiamo **GeoRust**, un per la gestione, la geolocalizzazione e la comunicazione in tempo reale con una flotta di veicoli aziendali".* 

Prima di entrare nei dettagli tecnici, vogliamo spiegare in due parole il problema che il sistema risolve, perché aiuta a capire tutte le scelte che seguiranno: 
>[!NOTE] immaginate un'azienda di trasporti con decine di furgoni in giro per la città. Il responsabile logistico ha bisogno di sapere, in ogni istante, dove si trova ogni mezzo, se è fermo o in movimento, quanta strada ha fatto oggi, e deve potergli mandare un messaggio, a tutta la flotta o a un singolo autista, senza dover telefonare uno per uno. **GeoRust** è esattamente questo: un sistema che tiene traccia della posizione dei veicoli e permette una comunicazione bidirezionale tra la centrale operativa e gli autisti.

L'obiettivo posto dalla traccia era realizzare un'architettura distribuita di tipo **client/server**, sviluppata interamente in linguaggio **Rust**, capace di soddisfare requisiti sia funzionali sia prestazionali piuttosto stringenti.

> 🎓 *Cosa significa "client/server"?* È il modello architetturale più diffuso per le applicazioni di rete: c'è un programma centrale, il **server**, che resta sempre acceso e in ascolto, e tanti programmi periferici, i **client** (in questo caso i veicoli), che si collegano al server per scambiare informazioni. Il server è l'unico punto che conosce lo stato completo del sistema (chi è online, dove si trova ogni veicolo, lo storico delle posizioni); i client, invece, conoscono solo la propria situazione e comunicano tutto al server.

Per riassumere i punti cardine della traccia:

- Il sistema doveva permettere a più utenti di **registrarsi** con un proprio account e una password protetta, **autenticarsi**, e iniziare a trasmettere periodicamente al server la propria posizione geografica emulata, con una cadenza fissa di **30 secondi**.
- Il server doveva gestire e monitorare lo **stato operativo** di ciascun veicolo, distinguendo in ogni istante se un mezzo si trova in stato **Sconnesso**, **Fermo** oppure **In Movimento**, secondo regole di transizione precise: il passaggio da "Fermo" a "In Movimento" deve scattare non appena viene rilevata una variazione delle coordinate; il passaggio inverso, da "In Movimento" a "Fermo", deve invece verificarsi solo quando la posizione del veicolo rimane invariata per **almeno tre minuti consecutivi**.
- Il server doveva fornire un motore di **analisi statistica del movimento** per ciascun veicolo: tragitto totale percorso in chilometri, velocità media, durata dei periodi di movimento e delle soste, con la possibilità di aggregare questi dati su intervalli temporali (giorno corrente, settimana corrente, mese corrente, oppure l'intero storico).
- Serviva una **comunicazione testuale bidirezionale**: l'autista può inviare messaggi alla centrale, e il server può inoltrare comunicazioni mirate a un singolo veicolo (modalità privata) oppure a tutti i veicoli connessi (broadcast).
- Infine, la traccia poneva vincoli non funzionali molto precisi: l'applicativo doveva essere **multipiattaforma** (eseguibile su almeno due sistemi operativi tra Windows, Linux, macOS, Android, ChromeOS e iOS), doveva prestare attenzione all'efficienza in termini di **tempo CPU** e **occupazione di memoria**, doveva generare un **file di log persistente** che registrasse ogni due minuti il consumo di CPU del server, e doveva mantenere una **dimensione degli eseguibili** contenuta, da riportare nel report finale.

Il diagramma dei casi d'uso che abbiamo prodotto in fase di analisi riassume bene questi due punti di vista — quello dell'utente/autista e quello del sistema nel suo complesso:

![Diagramma dei casi d'uso di GeoRust](/doc/img/UseCase.png)

A sinistra vedete cosa può fare l'**Utente** (registrarsi, autenticarsi, inviare messaggi, inviare periodicamente le coordinate); a destra cosa deve fare il **sistema GeoRust** internamente per rendere possibili quelle azioni: creare il file di log, ricevere le coordinate, gestire lo stato, analizzare il movimento, e gestire i messaggi sia in broadcast che verso singoli utenti — questi ultimi due rappresentati come *estensioni* dell'invio messaggi, perché condividono la stessa logica di base ma si comportano in modo diverso a seconda del destinatario.

---

## 2. Documento di Analisi e organizzazione del lavoro

Partendo da questa traccia, abbiamo redatto il nostro Documento di Analisi iniziale, formalizzando **16 requisiti funzionali** (cosa il sistema deve fare) e **9 requisiti non funzionali** (come il sistema deve farlo, in termini di qualità, prestazioni e vincoli tecnici).

<details>

| id  | Requisito                                                                                                                                                                                                 |
| :-- | :-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | Il sistema deve consentire a un nuovo utente di registrarsi inserendo un account e una password.                                                                                                          |
| R2  | Il sistema deve permettere agli utenti registrati di effettuare il login tramite account e password.                                                                                                      |
| R3  | Il sistema deve mantenere lo stato di connessione degli utenti, distinguendo tra utenti connessi e sconnessi nel caso in cui il server non riceva le coordinate della posizione del client dopo i 60 sec. |
| R4  | Il client deve inviare automaticamente al server la propria posizione geografica emulata ogni 30 secondi.                                                                                                 |
| R5  | Il server deve ricevere e memorizzare le coordinate inviate da ciascun utente con il relativo timestamp.                                                                                                  |
| R6  | Il server deve determinare automaticamente lo stato dell'utente tra, sconnesso, fermo, in movimento.                                                                                                      |
| R7  | Il sistema deve impostare lo stato In movimento quando rileva una variazione delle coordinate rispetto alla posizione precedente.                                                                         |
| R8  | Il sistema deve impostare lo stato Fermo quando la posizione rimane invariata per almeno 3 minuti consecutivi.                                                                                            |
| R9  | Il client deve consentire di simulare il movimento del veicolo attraverso la lettura da file.                                                                                                             |
| R10 | Il server deve permettere di visualizzare il percorso effettuato da uno specifico utente in un determinato intervallo temporale.                                                                          |
| R11 | Il server deve calcolare la velocità media di un utente nel periodo selezionato.                                                                                                                          |
| R12 | Il server deve calcolare il tempo complessivo trascorso in stato di fermo.                                                                                                                                |
| R13 | Le analisi devono poter essere effettuate relativamente a:giorno corrente, settimana corrente,mese corrente.                                                                                              |
| R14 | Il server deve poter inviare messaggi di testo ad un singolo utente o in broadcast a tutti gli utenti connessi.                                                                                           |
| R15 | Ogni client deve poter inviare messaggi di testo al server.                                                                                                                                               |
| R16 | Il server deve registrare ogni 2 minuti in un file di log il tempo di CPU utilizzato.                                                                                                                     |


| id  | Requisito                                                                                                                         |
| :-- | :-------------------------------------------------------------------------------------------------------------------------------- |
| RN1 | Il sistema deve essere sviluppato secondo un'architettura client/server.                                                          |
| RN2 | L'applicazione deve essere eseguibile su almeno due sistemi operativi.                                                            |
| RN3 | Il sistema deve minimizzare il consumo di CPU sia sul server sia sui client.                                                      |
| RN4 | L'applicazione deve mantenere una dimensione dell'eseguibile contenuta; tale dimensione dovrà essere riportata nel report finale. |
| RN5 | Il server deve continuare a gestire gli altri utenti anche in caso di disconnessione improvvisa di uno o più client.              |
| RN6 | Il file di log deve essere salvato su memoria permanente e aggiornato automaticamente ogni 2 minuti.                              |
| RN7 | L'invio delle coordinate deve avvenire con una periodicità di circa 30 secondi.                                                   |
| RN8 | Il server deve essere progettato per gestire contemporaneamente più client senza degradare significativamente le prestazioni.     |
| RN9 | Il codice deve essere organizzato in moduli separati per facilitarne manutenzione ed estensione.                                  |


</details>

<br>

>[!TIP] Tra i requisiti non funzionali, abbiamo individuato alcune sfide architetturali cruciali:
>1. la gestione **concorrente** di centinaia di client senza far degradare le prestazioni del server;
>2. l'assenza totale di **cicli di polling attivo** (cioè di controlli ripetuti "a vuoto" fatti in loop), per azzerare il consumo di CPU quando il sistema è a riposo;
>3. l'**isolamento dei crash di rete**, in modo che la disconnessione improvvisa di un client non impatti in alcun modo sugli altri utenti connessi né sul server.

Vedremo tra poco come ognuna di queste tre sfide abbia guidato una scelta architetturale precisa: rispettivamente, l'uso dei ***task* asincroni di Tokio** al posto dei thread tradizionali, l'uso di **timer e canali** invece di cicli attivi, e l'architettura "una task per connessione" con pulizia automatica alla disconnessione.

Per affrontare lo sviluppo in modo efficiente e modulare, ci siamo suddivisi le responsabilità in quattro aree ben distinte:

1. Sottosistema di **persistenza degli utenti e dell'autenticazione**: la gestione del database SQLite per la tabella `users`, l'hashing e la verifica delle password, e la gestione sicura del flusso di registrazione e login.
2. **Infrastruttura di rete centrale**: il server TCP asincrono con Tokio, l'architettura a task indipendenti per ciascun client, il protocollo di framing a righe su TCP, il simulatore GPS e il ciclo di vita delle connessioni.
3. **Il "cervello" logico e matematico del sistema**: la macchina a stati del veicolo con la gestione della regola dei 3 minuti di sosta, il modulo di calcolo geometrico e statistico tramite la formula di Haversine per tragitto, velocità media e pause suddivise per giorno/settimana/mese, le relative query al database e l'interfaccia a riga di comando.
4. **Monitoraggio prestazionale ed ecosistema grafico**: il demone asincrono di logging della CPU ogni 2 minuti, le interfacce grafiche complete per il Client e per la Console Amministratore basate su `egui`, il sistema di messaggistica broadcast e privata, e lo script di stress test ad alto carico.

Questa suddivisione ci ha permesso di definire dei "contratti" chiari tramite modelli e tipi condivisi in `models.rs` — cioè delle strutture dati comuni su cui tutti eravamo d'accordo fin dall'inizio — consentendoci di sviluppare e testare in modo indipendente ogni componente prima dell'integrazione finale.

---

## 3. Panoramica architetturale: come "ragiona" il sistema

Prima di entrare modulo per modulo, vogliamo dare una visione d'insieme di come è organizzato GeoRust, perché tutte le scelte tecniche che vedremo dopo nascono da questa struttura di base.

Il cuore del sistema è un **server centrale**, scritto in Rust, che sfrutta il runtime asincrono **Tokio** per gestire la concorrenza, e la libreria **SQLite** (tramite la crate `rusqlite`) per la persistenza dei dati. Attorno al server ruotano quattro programmi client distinti: un client a riga di comando, un client con interfaccia grafica per l'autista, una console grafica per l'amministratore, e uno script di stress test per validare le prestazioni sotto carico.

![Diagramma di distribuzione (deployment) di GeoRust](/doc/img/DeploymentDiagram.svg)

Come si vede dal diagramma di distribuzione, tutti i client comunicano con il server tramite lo stesso protocollo TCP sulla porta `127.0.0.1:8080`; il server è l'unico componente che accede direttamente al database SQLite in scrittura, mentre la console amministratore vi accede anche in lettura diretta per le proprie statistiche di flotta.

> 🎓 Qui vale la pena fermarsi un momento, perché tutta la parte più interessante (e più delicata) del progetto riguarda **come il server riesce a gestire tante connessioni contemporaneamente senza rallentare**. Per capirlo servono due concetti.
>- **Il thread del sistema operativo** è l'unità di esecuzione "pesante" che conosciamo di solito: ogni thread ha il proprio stack di memoria (tipicamente centinaia di KB), e il sistema operativo deve fare un "cambio di contesto" (context switch) ogni volta che smette di eseguire un thread per farne partire un altro. 
>   - Se il nostro server aprisse un thread del sistema operativo per ogni singolo veicolo connesso, con centinaia di veicoli il solo consumo di memoria e i continui cambi di contesto diventerebbero un collo di bottiglia.
>- **Il modello asincrono di Tokio**, invece, funziona in modo diverso: invece di migliaia di thread pesanti, ci sono pochi thread reali del sistema operativo (un piccolo "pool", tipicamente uno per ogni core della CPU), e su questi vengono eseguite moltissime **task** — unità di lavoro leggerissime (poche decine di byte) che Tokio smista tra i thread disponibili. 
>   - La chiave sta nella parola `.await`: quando una task deve aspettare qualcosa (per esempio l'arrivo di un pacchetto di rete), invece di bloccare il thread che la sta eseguendo, "restituisce" il controllo a Tokio, che nel frattempo fa lavorare quel thread su un'altra task pronta.
>- Un paragone utile è quello di **un cameriere che serve più tavoli**: 
>   - un cameriere "sincrono" prenderebbe l'ordine a un tavolo e resterebbe fermo lì ad aspettare che la cucina prepari il piatto prima di passare al tavolo successivo — se i tavoli fossero cento, servirebbero cento camerieri. 
>   - Un cameriere "asincrono" invece prende l'ordine, lo passa in cucina, e mentre il piatto è in preparazione va a servire un altro tavolo; quando un piatto è pronto, torna a consegnarlo. 
>- Con questo approccio, pochi camerieri (i thread) riescono a servire moltissimi tavoli (le connessioni dei client) senza che nessuno resti bloccato ad aspettare.
>- Questo è esattamente il motivo per cui, come vedremo, il server apre **una task Tokio per ogni client connesso**, ma delega le operazioni davvero bloccanti — come le query al database, che sono intrinsecamente sincrone — a un pool di thread separato tramite `tokio::task::spawn_blocking`.

Con questa immagine in mente, il resto delle scelte progettuali che presenteremo — canali, lock, pattern fire-and-forget — risulteranno molto più naturali da capire.

---

## 4. Il Modulo di Autenticazione (`auth/`)

Passiamo ora al primo modulo concreto: `auth/`, suddiviso nei tre file `hash.rs`, `register.rs` e `login.rs`. La filosofia che abbiamo seguito è "ognuno fa una cosa sola": `register` e `login` decidono *cosa fare* (validare i dati, orchestrare i passaggi), mentre `hash.rs` si occupa esclusivamente della sicurezza delle password, e il modulo `db` si occupa esclusivamente della persistenza. Nessuno dei tre invade il compito dell'altro.

### 4.1 La scelta dell'algoritmo di hashing: 
#### perché SHA-256 e non Argon2

Qui abbiamo preso una decisione architetturale che vale la pena spiegare bene, perché mostra come un vincolo prestazionale possa cambiare radicalmente una scelta di sicurezza.

>🎓 *Perché non si salva mai la password in chiaro?* Se il database venisse compromesso, un attaccante non deve poter leggere le password degli utenti. Per questo si salva un **hash**: il risultato di una funzione matematica che trasforma la password in una stringa di lunghezza fissa, in modo che sia praticamente impossibile risalire alla password originale partendo dall'hash, ma sia comunque facile verificare se una password inserita corrisponde o meno.

Inizialmente avevamo pensato ad **Argon2**, l'algoritmo oggi consigliato per l'hashing delle password, perché è deliberatamente "lento" e "memory-hard": richiede molta memoria e molto tempo di CPU per essere calcolato, proprio per scoraggiare gli attacchi a forza bruta (chi prova milioni di password al secondo su un hash rubato). Il problema è che questa stessa caratteristica — l'essere volutamente costoso da calcolare — è in diretto conflitto con un requisito esplicito della traccia: **minimizzare il consumo di CPU**. In un'applicazione aziendale interna per una flotta di veicoli, dove le password sono gestite internamente e non esposte a un pubblico enorme su internet, abbiamo valutato che il rischio di un attacco a forza bruta massiccio fosse contenuto, e abbiamo quindi optato per un compromesso più leggero: **SHA-256 combinato con un salt casuale**.

>🎓 *Cos'è il salt e a cosa serve?* Se usassimo SHA-256 da solo, due utenti con la stessa password otterrebbero esattamente lo stesso hash memorizzato nel database — un problema, perché un attaccante potrebbe precompilare una tabella di hash per le password più comuni (le cosiddette *rainbow table*) e riconoscerle a colpo d'occhio. Il **salt** è una sequenza casuale di byte, generata in modo diverso per ogni singola registrazione, che viene concatenata alla password prima di calcolare l'hash. In questo modo, anche due utenti con la password identica avranno, nel database, hash completamente diversi.

Nel nostro caso il salt è composto da **16 byte casuali**, generati ad ogni registrazione con `rand::rng().fill(&mut salt)`. Nel database salviamo tutto in un'unica stringa nel formato:

```
<salt_esadecimale>:<hash_esadecimale>
```

In questo modo, quando un utente prova a fare login, la funzione `verify_password()` separa di nuovo il salt dall'hash, ricalcola SHA-256 sulla password appena digitata usando *lo stesso* salt originale, e confronta i due hash. Da notare: la password originale **non viene mai recuperata né ricostruita**, si confrontano solo gli hash.

Un ultimo dettaglio di sicurezza a cui teniamo particolarmente: per prevenire un attacco noto come **user enumeration**, la funzione `login_user` restituisce esattamente lo stesso errore generico (`AuthError::InvalidCredentials`) sia quando lo username non esiste nel database, sia quando la password è sbagliata. Se il server rispondesse in modo diverso nei due casi (per esempio "utente non trovato" contro "password errata"), un malintenzionato potrebbe usare il sistema di login stesso per scoprire, uno alla volta, quali username sono effettivamente registrati.

### 4.2 Il flusso di Registrazione e Login passo per passo

Il flusso di **registrazione** (`register_user`) è composto da quattro passaggi: normalizzazione dello username con `trim()` (per evitare problemi con spazi accidentali), validazione degli input (username non vuoto, password di almeno 4 caratteri), delega a `hash.rs` per il calcolo dell'hash, e infine inserimento nel database tramite `db::users::insert_user()`. In nessun punto di questo flusso la password in chiaro viene passata al database.

Il diagramma di sequenza seguente mostra visivamente questo scambio tra i moduli, dal momento in cui il client invia le credenziali fino alla risposta finale:

![](/doc/img/sequence/register.png)

Come si vede, `auth/register.rs` prima valida le credenziali localmente, poi chiama `auth/hash.rs` per ottenere la stringa `salt:password_hashed`, e solo a quel punto interroga `db/users.rs`, che esegue l'`INSERT` reale sulla tabella `users` del database `georust.db` e restituisce l'utente appena creato lungo tutta la catena di chiamate, fino al client.

Il flusso di **login** (`login_user`) è speculare: cerca l'utente nel database tramite username; se non lo trova, restituisce subito l'errore generico di cui parlavamo prima; se lo trova, delega a `hash.rs` la verifica della password; se la verifica fallisce, restituisce lo stesso errore generico.

![](/doc/img/sequence/login.png)

Qui il passaggio interessante è la chiamata `verify_password(password, User.password_hash)`: il modulo `auth/hash.rs` riceve l'hash già salvato (comprensivo di salt) insieme alla password appena digitata dall'utente, e restituisce semplicemente un booleano `true`/`false`, senza mai esporre all'esterno i dettagli crittografici del confronto.

---

## 5. Il Modulo Database e l'integrazione con Tokio (`db/`)

Il modulo `db/` gestisce la persistenza tramite `rusqlite`, e qui incontriamo per la prima volta, in pratica, il problema di concorrenza di cui parlavamo nell'introduzione architetturale.

### 5.1 Perché il database "blocca" e come lo isoliamo

`rusqlite::Connection` è un driver **sincrono e bloccante**: quando gli si chiede di eseguire una query, il thread che la esegue resta fermo, fisicamente in attesa, finché SQLite non ha finito di leggere o scrivere su disco. In più, `Connection` non implementa il trait `Sync` di Rust, il che significa che — per come è scritta la libreria — non può essere condivisa direttamente e in sicurezza tra più task asincrone.

Il problema pratico è questo: se avessimo eseguito una query SQL direttamente dentro l'event loop asincrono di Tokio (cioè dentro una task normale, senza precauzioni), quella singola query avrebbe **bloccato** uno dei pochi thread reali del pool di Tokio. Con centinaia di client connessi, anche poche query lente sarebbero bastate a rallentare l'intero server.

>[!TIP] La soluzione che abbiamo adottato è incapsulare la connessione SQLite dentro una struttura `Arc<Mutex<Connection>>`, contenuta nel nostro `AppState`, e delegare **ogni singola interazione con il database** al pool di thread dedicati alle operazioni bloccanti di Tokio, tramite `tokio::task::spawn_blocking`.

>🎓 *Cosa fa `spawn_blocking`?* Tokio mantiene, oltre al piccolo pool di thread "asincroni" che eseguono le task normali, anche un secondo pool di thread separato, pensato apposta per il codice bloccante che non si può rendere asincrono (come le librerie che parlano direttamente con il filesystem). Quando chiamiamo `spawn_blocking`, la chiusura (`closure`) che passiamo viene eseguita su uno di questi thread dedicati, lasciando completamente liberi i thread del pool principale di continuare a servire le connessioni di rete. 

```rust
// handler.rs
let db_arc = state.db_arc();
let result = task::spawn_blocking(move || {
    let conn = db_arc.lock().expect("DB Mutex avvelenato");
    login_user(&conn, &credentials)
}).await;
```

### 5.2 Due strategie diverse a seconda dell'operazione

Non tutte le operazioni sul database hanno la stessa urgenza, e abbiamo trattato in modo diverso due casistiche:

1. Per **login e registrazione**, il server invoca `spawn_blocking` e ne **attende** il risultato con `.await`, perché l'esito della query serve immediatamente per costruire la risposta (`AuthResult`) da rimandare al client: non possiamo rispondere "login riuscito" prima di sapere davvero se lo è stato.
2. Per **l'aggiornamento della posizione GPS** (`UpdatePosition`), che può arrivare molto più di frequente, abbiamo adottato un pattern chiamato **fire-and-forget** ("spara e dimentica"): quando arriva una nuova coordinata, lanciamo l'inserimento nel database con `spawn_blocking` **senza attendere** il completamento. Il ciclo di lettura di quel client (la reader task) rilascia immediatamente l'esecuzione e torna subito ad ascoltare il socket TCP, senza aspettare che la scrittura su disco sia finita. Le scritture su SQLite restano comunque rigorosamente serializzate e protette dal `Mutex` — cioè avvengono sempre una alla volta, in sicurezza — ma la latenza percepita dal client scende a meno di un millisecondo, perché non deve aspettare l'I/O su disco per continuare a inviare la coordinata successiva.

Il diagramma seguente mostra proprio questo secondo flusso, dal simulatore GPS fino alla tabella `position` nel database:

![Diagramma di sequenza dell'invio posizione GPS](/doc/img/sequence/simulation.png)

Da notare, nel diagramma, il passaggio attraverso `network/handler.rs`, che riceve il messaggio già deserializzato (`ClientMessage::UpdatePosition { lat, lon }`) e lo inoltra a `db/position.rs` tramite `task::spawn_blocking`, senza mai bloccare il ciclo di lettura del client.

### 5.3 Lo schema del database

Le tabelle create da `init_db()` in `connection.rs` sono due:

```sql
CREATE TABLE IF NOT EXISTS users(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS positions(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL,
    lat REAL NOT NULL,
    lon REAL NOT NULL,
    timestamp TEXT NOT NULL DEFAULT (datetime('now'))
);
```

- Il vincolo `UNIQUE` sullo username garantisce l'unicità già a livello di database (non solo a livello applicativo), gestita nel nostro codice come `DbError::UserAlreadyExists`.
- Nella tabella `users`, `insert_user()` utilizza la clausola `RETURNING` per ottenere subito tutti i dati dell'utente appena creato in un'unica query, evitando un secondo giro di andata e ritorno con il database;
- `find_user_by_username()` usa `prepare_cached()`, che riutilizza lo statement SQL già compilato tra una chiamata e l'altra, risparmiando tempo sulle interrogazioni ripetute nel tempo. 
- Nella tabella `positions`, ogni riga salva latitudine, longitudine, username e un timestamp generato automaticamente da SQLite; `get_positions_by_user()` recupera tutto lo storico ordinato cronologicamente, usando l'`id` come criterio di spareggio per i casi in cui due timestamp coincidano.

---

## 6. Il Modulo di Rete: task, canali e assenza di deadlock (`network/`)

Arriviamo ora al cuore del progetto: il modulo `network/`, composto da `handler.rs`, `protocol.rs`, `state.rs`, `app_state.rs` e dalle console interattive. È qui che si concentrano quasi tutte le scelte legate alla gestione dei thread e della concorrenza, quindi ci prendiamo il tempo di spiegarle con calma, passo per passo.

### 6.1 Il problema del framing su TCP

Il primo problema, molto pratico, riguarda il protocollo TCP in sé. TCP è un protocollo **a stream continuo di byte**: non sa nulla dei "messaggi" della nostra applicazione, sa solo trasportare un flusso ininterrotto di byte da un capo all'altro della connessione.

>🎓 *Perché è un problema?* Immaginate di dover leggere un libro in cui qualcuno ha tolto tutti gli spazi e la punteggiatura tra le parole: il testo è tutto lì, ma non sapete più dove finisce una parola e ne inizia un'altra. È lo stesso problema che avremmo noi se leggessimo il flusso TCP grezzo: due messaggi JSON inviati vicini nel tempo possono "fondersi" in un'unica lettura (*packet coalescence*), oppure un singolo messaggio può arrivare spezzato in due letture separate (*packet fragmentation*), perché la rete decide autonomamente come suddividere i dati in pacchetti fisici.

>[!TIP] Per risolvere questo problema, con un costo computazionale praticamente nullo, usiamo il wrapper `tokio_util::codec::Framed` abbinato a `LinesCodec`. 
>L'idea è semplice quanto efficace: usiamo il carattere di **fine riga** (`\n`) come "spazio tra le parole". In ricezione, `LinesCodec` accumula i byte che arrivano in un buffer interno e restituisce (*emette*) un messaggio solo nel momento esatto in cui incontra un `\n`; in trasmissione, aggiunge automaticamente un `\n` alla fine di ogni messaggio JSON prima di spedirlo. In questo modo, ogni volta che il nostro codice chiama `rx.next().await`, siamo certi di ricevere sempre e solo un messaggio JSON completo, mai un frammento e mai due messaggi incollati insieme.

Tutta la comunicazione applicativa è quindi fatta di **messaggi JSON tipizzati**, definiti tramite due enum Rust:

```rust
pub enum ClientMessage {
    Login { username: String, password_hash: String },
    Register { username: String, password_hash: String },
    UpdatePosition { lat: f64, lon: f64 },
    SendText { text: String },
    SendPrivateText { to: String, text: String },
    Disconnect,
    // ...
}

pub enum ServerMessage {
    AuthResult { success: bool, msg: String },
    Text { from: String, text: String },
    Error(String),
    // ...
}
```

Sul filo, un messaggio di login diventa letteralmente questa riga di testo, seguita da un `\n`:

```json
{"Login":{"username":"driver1","password_hash":"segreto123"}}
```

### 6.2 Una task per ogni connessione: reader e writer separati

Quando il listener TCP del server accetta una nuova connessione (`listener.accept().await`), il server genera una task asincrona dedicata con `tokio::spawn(handle_client(stream, state))`. Torniamo alla metafora del cameriere: ogni cliente che entra nel locale (ogni veicolo che si connette) ottiene il proprio "cameriere personale" — ma ricordiamoci che questi camerieri sono task leggerissime, non thread del sistema operativo: il server può permettersi di averne centinaia contemporaneamente senza problemi.

All'interno di `handle_client`, il socket TCP viene diviso a metà con `framed.split()`: da un lato uno **stream di lettura** (`rx`), dall'altro un **sink di scrittura** (`tx`). Questa divisione è necessaria perché in Rust il lato di scrittura di un socket non è clonabile: non possiamo semplicemente "passare una copia" del socket ad altri task che vogliano scrivergli sopra (per esempio, la console dell'amministratore che vuole mandare un messaggio broadcast a tutti).

La soluzione è creare, per ogni client, un **canale interno** `tokio::sync::mpsc::channel::<String>(32)` e avviare un secondo task — il **writer task** — dedicato esclusivamente a scrivere sul socket:

```rust
tokio::spawn(async move {
    while let Some(msg) = mpsc_rx.recv().await {
        if tx.send(msg).await.is_err() {
            break; // Socket chiuso: arresto pulito
        }
    }
});
```

>🎓 *Cos'è un canale `mpsc` e perché è comodo?* `mpsc` sta per *multi-producer, single-consumer*: è come una **buca delle lettere** dedicata a quel singolo client. Il lato che "imbuca" i messaggi (`Sender`, chiamato `mpsc_tx`) può essere clonato liberamente e passato a chiunque nel sistema debba mandare un messaggio a quel client — un altro task, la console dell'amministratore, un altro utente in chat — mentre il lato che "svuota la buca" (`Receiver`) appartiene solo ed esclusivamente al writer task di quel client. Nessuno tocca mai direttamente il socket TCP tranne il suo writer task dedicato: tutti gli altri si limitano a imbucare un messaggio nel canale.

Non appena l'utente effettua il login con successo, il suo `mpsc_tx` viene inserito nella mappa in memoria del server: `ServerState.online_users: HashMap<String, mpsc::Sender<String>>`, protetta da un `RwLock` (ne parliamo tra un attimo). Da quel momento, mandare un messaggio privato o un broadcast a quell'utente è banale: basta prendere il suo `mpsc_tx` dalla mappa e chiamare `sender.send(json).await` — sarà il writer task di quel client, in background, a occuparsi fisicamente di scrivere sul cavo di rete.

### 6.3 Come evitiamo i deadlock: il pattern "closure" su `AppState`

Uno degli errori più insidiosi nella programmazione asincrona in Rust è **trattenere un lock (un `Mutex` o un `RwLock`) attraverso un punto di sospensione `.await`**.

>🎓 *Perché è pericoloso?* Un lock è come la chiave di una stanza: solo chi la possiede può entrare. Se una task prende la chiave della stanza (acquisisce il lock) e poi, mentre è ancora dentro, si "addormenta" in attesa di qualcos'altro — per esempio l'invio di un pacchetto di rete, che è un'operazione `.await` — nessun'altra task potrà entrare in quella stanza finché la prima non si sveglia e restituisce la chiave. Se, nel frattempo, un'altra task avesse bisogno di quella stessa stanza per completare l'operazione che la prima sta aspettando, il sistema resterebbe bloccato per sempre: è il classico **deadlock**.

Per evitare categoricamente questo scenario, abbiamo incapsulato lo stato condiviso `ServerState` dentro `AppState`, esponendo l'accesso **esclusivamente tramite metodi basati su closure**, cioè funzioni che ricevono il lock, lo usano, e lo rilasciano immediatamente, tutto all'interno dello stesso blocco di codice sincrono:

```rust
pub fn with_state<F, R>(&self, f: F) -> R
where
    F: FnOnce(&ServerState) -> R,
{
    let guard = self.state.read().expect("RwLock avvelenato");
    f(&guard)
} // il lock viene rilasciato qui, PRIMA di qualsiasi eventuale .await successivo
```

Guardate come questo si traduce in pratica nella gestione di un messaggio in broadcast:

```rust
// 1. Prendiamo il lock in lettura solo per pochi microsecondi:
let senders: Vec<_> = state.with_state(|st| st.online_users.values().cloned().collect());
// 2. Il lock è già stato rilasciato: ORA facciamo l'invio asincrono
for sender in senders {
    let _ = sender.send(json.clone()).await;
}
```

Il lock viene acquisito, usato solo per clonare velocemente i canali `mpsc` di tutti gli utenti online in un vettore locale, e rilasciato all'istante. Solo *dopo* aver lasciato la "stanza" libera, il server esegue il ciclo di invio vero e proprio con `.await`. In nessun punto del codice un lock resta attivo mentre una operazione asincrona è in corso.

Un'osservazione tecnica in più: usiamo un `RwLock` (*read-write lock*) e non un semplice `Mutex` per la mappa degli utenti online, perché la stragrande maggioranza delle operazioni sono **letture** (chi è online? a chi mando questo messaggio?), mentre le scritture (un nuovo login, una disconnessione) sono relativamente rare. Un `RwLock` permette a più letture di avvenire **contemporaneamente**, e riserva l'esclusione reciproca solo al momento in cui qualcuno deve effettivamente modificare la mappa — massimizzando quindi la concorrenza rispetto a un `Mutex`, che invece serializzerebbe anche le sole letture.

Per il database, invece, usiamo un semplice `Mutex<Connection>`: qui non ha senso distinguere tra letture e scritture, perché SQLite stesso richiede che gli accessi siano comunque serializzati (una query alla volta sulla stessa connessione); inoltre, come spiegato prima, questo `Mutex` viene sempre acquisito e rilasciato **dentro** un `spawn_blocking`, mai attraverso un `.await` asincrono.

### 6.4 Disconnessione pulita, senza thread zombie

Se un client si disconnette volontariamente (messaggio `Disconnect`) oppure la connessione cade all'improvviso, il ciclo di lettura `rx.next()` termina naturalmente. A quel punto il server rimuove atomicamente lo username dalla mappa `online_users`, il canale `mpsc` associato si chiude automaticamente, e il writer task — che era in attesa su quel canale con `mpsc_rx.recv().await` — si accorge che il canale è chiuso ed esce dal proprio ciclo da solo, senza bisogno di essere "avvisato" esplicitamente. Nessun thread zombie, nessuna risorsa orfana in memoria.

>[!TIP] Questo è anche il motivo per cui una disconnessione improvvisa di un client non ha alcun impatto sugli altri: ogni "coppia" reader/writer vive in modo completamente isolato dalle altre, e comunica con il resto del sistema solo attraverso la mappa condivisa e i canali — mai attraverso stato privato di un altro client.

---

## 7. Il Modulo GPS: macchina a stati e analisi del movimento (`gps/`)

Passiamo ora al modulo che rappresenta la logica di tracciamento e analisi vera e propria del sistema.

### 7.1 Il simulatore GPS

Poiché non disponiamo di veicoli reali con ricevitori GPS, il movimento viene emulato: il client legge in modo asincrono un percorso da un file CSV (`data/route.csv`) e, tramite un timer periodico non bloccante (`tokio::time::interval`), emette una coordinata `(lat, lon)` esattamente ogni 30 secondi su un canale verso il loop di trasmissione. Anche qui la scelta di un canale `mpsc` non è casuale: il simulatore gira in un task separato, completamente disaccoppiato dal loop di rete che invierà poi la coordinata al server — il produttore (simulatore) non ha bisogno di sapere nulla su come il consumatore (loop di rete) userà quel dato.

### 7.2 La macchina a stati del veicolo

Lo stato di ogni veicolo è governato dalla struttura `UserTracker`, che implementa esattamente le regole di transizione richieste dalla traccia:

| Stato di partenza | → Sconnesso | → Fermo | → In Movimento |
| :--- | :--- | :--- | :--- |
| **Sconnesso** | — | alla prima posizione ricevuta | — |
| **Fermo** | nessun aggiornamento da oltre 60s | posizione invariata | distanza dalla precedente > 5 metri |
| **In Movimento** | nessun aggiornamento da oltre 60s | fermo da almeno 180 secondi (3 minuti) | distanza dalla precedente > 5 metri |

La transizione **da "Fermo" a "In Movimento"** scatta istantaneamente non appena la distanza tra la nuova coordinata e la precedente supera una soglia di **5 metri**. Questa soglia non è arbitraria: un ricevitore GPS reale (o, nel nostro caso, il modello che vogliamo emulare fedelmente) non restituisce mai coordinate perfettamente identiche, anche a veicolo completamente fermo, per via del cosiddetto *rumore di misura*. Se avessimo usato una soglia di zero metri, il sistema avrebbe continuamente scambiato piccolissime oscillazioni casuali per un vero movimento, generando falsi positivi.

La transizione **da "In Movimento" a "Fermo"** è invece temporizzata: il tracker tiene traccia dell'ultimo istante in cui è stato rilevato un vero movimento (`last_moved_time`), e ad ogni nuovo aggiornamento controlla quanto tempo è trascorso da quel momento. Solo se sono passati **almeno 180 secondi consecutivi** (cioè 6 pacchetti da 30 secondi ciascuno) senza che la posizione sia cambiata di più di 5 metri, lo stato transita a "Fermo".

Abbiamo verificato questa logica con un test dedicato che riproduce fedelmente l'intera sequenza del file `route.csv`: il veicolo parte, si muove per 4 intervalli da 30 secondi, poi resta sulla stessa coordinata per 8 intervalli (240 secondi); il test controlla matematicamente che lo stato resti "In Movimento" durante i primi 2 minuti e mezzo di sosta apparente, e scatti esattamente a "Fermo" al terzo minuto, per poi tornare "In Movimento" non appena il veicolo riparte.

### 7.3 Il calcolo delle distanze: la formula di Haversine

Per calcolare la distanza reale tra due coordinate geografiche (latitudine e longitudine) non basta la semplice distanza euclidea tra due punti su un piano: la Terra è (approssimativamente) una sfera, e due punti con la stessa differenza di longitudine sono più vicini tra loro vicino ai poli che all'equatore. Per questo motivo usiamo la **formula di Haversine**, pensata apposta per calcolare distanze tra coordinate su una sfera:

$$a = \sin^2\left(\frac{\Delta\text{lat}}{2}\right) + \cos(\text{lat}_1)\cos(\text{lat}_2)\sin^2\left(\frac{\Delta\text{lon}}{2}\right)$$
$$c = 2 \cdot \text{atan2}\left(\sqrt{a}, \sqrt{1-a}\right), \quad d = R \cdot c$$

dove $R = 6371\text{ km}$ è il raggio medio della Terra. 
>Non serve ricordare la formula a memoria durante la discussione: il punto concettuale è che questa formula tiene conto della curvatura terrestre, ed è lo standard de facto per questo tipo di calcoli in ambito GPS. L'abbiamo validata confrontando i risultati con distanze reali note tra città italiane (per esempio Roma–Milano, verificando i circa 477 km con una tolleranza inferiore all'1%).

A partire da questa formula, la funzione `analyze_movement()` scorre le posizioni storiche a coppie consecutive: se la distanza tra due punti supera i 5 metri, l'intervallo di tempo tra i due viene sommato al **tempo in movimento** e la distanza al **tragitto totale**; altrimenti viene sommato al **tempo di sosta**. La **velocità media** si ottiene semplicemente dividendo il tragitto totale (in km) per il solo tempo effettivo in movimento (in ore) — non per il tempo totale, altrimenti le soste abbasserebbero artificialmente la velocità media registrata. Infine, `filter_positions_by_timerange()` usa la crate `chrono` per restringere l'analisi a giorno corrente, settimana corrente, mese corrente o all'intero storico, gestendo correttamente i timestamp nel formato testuale usato da SQLite.

---

## 8. Le interfacce grafiche: quando l'async incontra la GUI (`bin/`)

Il sistema mette a disposizione quattro eseguibili: il server, il client a riga di comando, il client con interfaccia grafica e la console amministratore. 
>[!TIP] Il client CLI è concettualmente il più semplice: 
gira su un unico thread cooperativo che usa la macro `tokio::select!` per multiplexare — cioè gestire in parallelo, alternandosi automaticamente — tre sorgenti asincrone diverse: la digitazione da tastiera, il simulatore GPS e i messaggi in arrivo dal server. Grazie a `select!`, il processo resta completamente "addormentato" (consumo CPU vicino allo 0%) finché una qualsiasi delle tre sorgenti non produce un evento.

Le due interfacce grafiche (`client_gui` e `admin_gui`), realizzate con il framework `egui`/`eframe`, pongono invece un problema nuovo, che vale la pena spiegare.

### 8.1 Il conflitto tra GUI "immediate mode" e networking asincrono

`egui` è una libreria a **modalità immediata** (*immediate mode GUI*): il suo metodo `update()` viene richiamato circa 60 volte al secondo — una per ogni fotogramma disegnato a schermo — e viene eseguito **sul thread principale della finestra grafica del sistema operativo**. 
>[!TIP] Questo metodo non può essere `async`, e soprattutto non può mai bloccarsi: se restasse fermo anche solo per pochi millisecondi ad aspettare una risposta di rete, l'intera finestra si congelerebbe visibilmente, perché il rendering condivide lo stesso thread.

La soluzione che abbiamo adottato è avviare, all'avvio del client grafico, un **secondo runtime Tokio completamente dedicato**, che gira in background su thread propri:

```rust
let runtime = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .worker_threads(2)
    .thread_name("georust-client-rt")
    .build()
    .expect("Impossibile avviare il runtime Tokio in background");
```

Tutta la logica di rete e il simulatore GPS girano su questo runtime separato, mentre il thread della GUI resta libero di disegnare i propri fotogrammi senza mai fermarsi. La comunicazione tra i due mondi — il thread grafico sincrono e i task asincroni in background — avviene tramite canali `mpsc` non bloccanti (in questo caso `unbounded_channel`, cioè senza limite di dimensione): uno per portare i messaggi dalla rete alla GUI, uno per le coordinate GPS, e uno per inviare dalla GUI verso la rete i messaggi digitati dall'utente.

Il dettaglio tecnico chiave è la scelta di `try_recv()` al posto di `recv().await` dentro il ciclo `update()`:

```rust
while let Ok(line) = rx.try_recv() {
    // elabora i messaggi accumulati nel canale
}
```

`try_recv()` restituisce **immediatamente** un errore `Empty` se non ci sono messaggi in coda, invece di restare in attesa come farebbe `.await`. Questo permette alla GUI di controllare "c'è qualcosa di nuovo?" ad ogni fotogramma senza mai bloccarsi, mantenendo un rendering fluido a 60 fps indipendentemente da cosa stia succedendo sulla rete in quel momento.

### 8.2 Il pulsante "Simula Fermo" e `AtomicBool`

L'autista dispone di un pulsante **"Simula Fermo"**, che congela l'avanzamento del veicolo lungo il percorso simulato pur continuando a inviare l'ultima posizione nota — utile, ad esempio, per osservare dal vivo la transizione a stato "Fermo" dopo i tre minuti previsti. Questo flag booleano viene letto dal task GPS in background e scritto dal thread della GUI: due thread diversi che devono condividere in sicurezza un semplice valore vero/falso.

Per questo caso specifico non serve la "pesantezza" di un `Mutex`: usiamo invece un `Arc<AtomicBool>`.

>🎓 *Perché un `AtomicBool` invece di un `Mutex<bool>`?* Un `Mutex` è come una chiave che qualcuno deve prendere, usare, e restituire prima che un altro possa accedere alla risorsa: comporta sempre un minimo overhead di gestione, anche per un semplice booleano. Un tipo *atomico* come `AtomicBool`, invece, sfrutta istruzioni speciali del processore che garantiscono che leggere o scrivere quel singolo valore sia sempre un'operazione indivisibile e sicura anche tra thread diversi, senza bisogno di alcun lock esplicito — un po' come un interruttore della luce che, per come è fatto fisicamente, non può mai restare "a metà" tra acceso e spento. Per un singolo valore semplice come questo, è la soluzione più leggera ed efficiente possibile.

### 8.3 La Console Amministratore

La console amministratore (`admin_gui`) condivide la stessa architettura a doppio runtime appena descritta, con due aggiunte: si autentica con credenziali speciali (`ADMIN_CONSOLE`), e mantiene anche una **connessione diretta e in sola lettura al database SQLite**, usata per ricostruire in tempo reale lo stato dell'intera flotta e per calcolare le analisi di movimento senza dover passare dal server per ogni interrogazione. Offre quattro schede principali: la **Scheda Flotta** (stato in tempo reale di tutti i veicoli, con auto-refresh ogni 2 secondi), la **Scheda Analisi** (tragitto, velocità media, tempo di movimento e sosta per intervallo temporale selezionabile), la **Scheda Messaggistica** (invio broadcast o privato) e la **Scheda CPU** (lettura delle ultime righe di `cpu_log.txt`).

---

## 9. Il Logging della CPU e i risultati prestazionali

Uno dei requisiti non funzionali più specifici della traccia era la generazione di un file di log che riportasse, ogni due minuti, il consumo di CPU del server. Il modulo `logging/cpu_logger.rs` implementa questo comportamento come un task asincrono indipendente, avviato all'inizio del server con `tokio::spawn`.

>[!TIP] Un dettaglio tecnico che vale la pena spiegare: 
all'avvio, il logger effettua **due campionamenti consecutivi distanziati di 500 millisecondi**, invece di uno solo. Il motivo è che i moderni sistemi operativi calcolano la percentuale di utilizzo della CPU come un **rapporto tra il tempo di processore consumato e il tempo reale trascorso** tra due misurazioni: una singola misurazione istantanea, senza un punto di riferimento precedente, restituirebbe sempre 0%, perché non c'è nessun intervallo di tempo su cui calcolare il rapporto. Da quel primo doppio campionamento in poi, il logger si risveglia ogni **120 secondi** tramite `tokio::time::sleep`, e scrive su `cpu_log.txt` sia il tempo di CPU cumulativo sia la percentuale media nell'ultimo intervallo, usando `tokio::fs` per una scrittura asincrona che non blocca mai i worker del server.

### 9.1 I risultati dello Stress Test

Per validare empiricamente la robustezza del server, abbiamo sviluppato un binario dedicato di stress test (`src/bin/stress_test.rs`), che avvia **100 bot concorrenti**, ciascuno dei quali invia una coordinata GPS ogni 500 millisecondi — un carico complessivo di **200 aggiornamenti al secondo**, pari a circa **12.000 scritture su SQLite al minuto**.

Questo è un estratto reale del file `cpu_log.txt` prodotto durante il test:

```text
=== NUOVA SESSIONE SERVER AVVIATA [2026-09-03 16:01:36] ===
[2026-09-03 16:01:36] Tempo di CPU iniziale: 656.25ms (Utilizzo: 18.18%) <-- Avvio e connessione iniziale 100 bot
[2026-09-03 16:03:36] Tempo di CPU totale utilizzato: 38.09s (Utilizzo nell'ultimo intervallo: 31.19%) <-- Sotto stress continuo
[2026-09-03 16:05:36] Tempo di CPU totale utilizzato: 77.39s (Utilizzo nell'ultimo intervallo: 32.74%) <-- Sotto stress continuo
[2026-09-03 16:07:36] Tempo di CPU totale utilizzato: 83.27s (Utilizzo nell'ultimo intervallo: 4.90%)  <-- Conclusione test e rilascio
[2026-09-03 16:09:36] Tempo di CPU totale utilizzato: 83.27s (Utilizzo nell'ultimo intervallo: 0.00%)  <-- Ritorno a riposo (Idle)
```

I numeri raccontano bene la storia dell'intera architettura che abbiamo appena descritto:

- Con **100 client connessi contemporaneamente**, sotto un carico continuo di 200 aggiornamenti al secondo, l'occupazione della CPU si è mantenuta intorno al **31–33%** — un valore contenuto, reso possibile proprio dal fatto che nessuna operazione bloccante (come le scritture su SQLite) viene mai eseguita sul thread pool principale che gestisce la rete.
- Sono state registrate oltre **48.000 posizioni GPS** nel database, senza un solo crash, disconnessione anomala, errore di deserializzazione JSON o deadlock — una conferma diretta che il pattern "closure senza lock trattenuti su `.await`" descritto poco fa funziona davvero anche sotto carico estremo.
- Al termine del test, l'utilizzo di CPU è tornato **istantaneamente allo 0.00%**: non essendoci alcun ciclo di polling attivo da nessuna parte del sistema (solo timer, canali e `select!`), non restano task "zombie" a consumare risorse quando non c'è nulla da fare.
- La memoria occupata dal server è di circa **6–12 MB a riposo** e sale a soli **~26 MB** sotto stress con 100 bot attivi — un valore molto contenuto, reso possibile dal modello di gestione della memoria di Rust basato su *ownership* e *borrowing*, che non richiede un garbage collector e quindi non introduce pause di pulizia periodica della heap.
- La latenza locale per un ciclo completo di ricezione e broadcast è inferiore a **1 millisecondo**, grazie al disaccoppiamento tra la persistenza su disco (fire-and-forget) e il flusso di rete.
- Gli eseguibili compilati in modalità Release restano molto leggeri: **~2.7 MB** per il server, **~2.3 MB** per il client CLI, **~6.3 MB** e **~6.4 MB** rispettivamente per i due client con interfaccia grafica (il peso maggiore è dovuto all'inclusione del framework grafico `egui`/`eframe`). Va ricordato che si tratta di binari nativi e staticamente linkati, che incorporano l'intero runtime asincrono, le librerie crittografiche e il driver SQLite, senza dipendenze esterne da installare.

---

## 10. Il Piano di Testing e la Validazione Sperimentale

Per garantire la robustezza del codice, abbiamo accompagnato ogni modulo con una suite di test unitari e di integrazione — **39 test totali** all'interno di `src/lib.rs`:

- **Modulo Auth**: unicità del salt crittografico (verificando che due password identiche generino hash diversi), validazione dei campi vuoti o delle password troppo corte, e il ciclo completo di registrazione e login sia con credenziali valide sia con credenziali errate.
- **Modulo DB**: creazione delle tabelle SQLite in memoria (`init_in_memory`, usata per non toccare mai un vero file su disco durante i test), verifica del vincolo `UNIQUE` che impedisce username duplicati, e corretto ordinamento cronologico delle posizioni registrate.
- **Modulo GPS e Analytics**: validazione della formula di Haversine confrontandola con distanze reali note tra città italiane (Roma–Milano, ~477 km, con tolleranza inferiore all'1%), test della soglia di movimento a 5 metri, e — il test più elaborato — la riproduzione fedele dell'intera sequenza di `route.csv`, verificando matematicamente ogni transizione di stato descritta nel paragrafo 7.2.
- **Modulo Network**: serializzazione JSON di tutti i pacchetti del protocollo, gestione dei pacchetti malformati (per non far crashare il server con un input inatteso), e concorrenza su `AppState`, simulando 10 task paralleli che leggono e scrivono contemporaneamente senza generare contese o crash.
- **Modulo Logging**: correttezza del formato stringa scritto nel file di log, e corretto funzionamento del campionamento CPU tramite `sysinfo`.

Questi test automatizzati ci hanno permesso di verificare la correttezza matematica e funzionale del sistema ad ogni iterazione dello sviluppo, dandoci la sicurezza di poter modificare un modulo senza rompere silenziosamente il comportamento di un altro.

---

## 11. Conclusioni finali

In conclusione, il progetto **GeoRust** dimostra come le caratteristiche del linguaggio Rust — in particolare la gestione deterministica della memoria basata su *ownership* e *borrowing*, l'assenza di garbage collector, e l'ecosistema asincrono di Tokio — permettano di realizzare un sistema distribuito solido e reattivo con un impegno di risorse minimo:

1. **Completamente esente da data race, memory leak e deadlock**, grazie a scelte progettuali precise — non a caso, non per fortuna: ogni lock è racchiuso in closure che ne garantiscono il rilascio prima di qualsiasi `.await`, ogni operazione bloccante è isolata in `spawn_blocking`, e ogni connessione vive in un ciclo di vita completamente indipendente dalle altre.
2. **Capace di sostenere elevati volumi di traffico** — 100 client concorrenti, 200 aggiornamenti al secondo — con consumi di memoria minimi (poche decine di MB) e un consumo di CPU a riposo praticamente nullo, grazie all'assenza totale di cicli di polling attivo.
3. **Con un'architettura modulare e pulita**, organizzata per responsabilità (autenticazione, persistenza, rete, GPS, logging, interfacce), pronta per essere estesa in futuro con ulteriori sensori, piattaforme o funzionalità di telemetria.

Vi ringraziamo per l'attenzione e siamo a vostra completa disposizione per rispondere a qualsiasi domanda, o per mostrarvi una dimostrazione dal vivo del funzionamento del software.

---

> ## Appendice: domande frequenti a cui prepararsi
>Questa sezione non fa parte del discorso da esporre a voce, ma raccoglie alcune possibili domande della commissione con risposte pronte, utili per lo studio.
**D: Cosa succederebbe se rimuoveste il salt dalle password?**
R: Due utenti con la stessa password otterrebbero lo stesso identico hash nel database. Un attaccante che ottenesse accesso al database potrebbe usare tabelle precompilate di hash per le password più comuni (*rainbow table*) per risalire rapidamente alle password in chiaro di tutti gli utenti che condividono quella password, invece di dover attaccare ogni utente singolarmente.
**D: Perché non usare semplicemente un thread per client, invece di tutta questa architettura asincrona?**
R: Con un numero contenuto di client (poche decine) la differenza sarebbe minima. Ma la traccia richiedeva di gestire "centinaia" di connessioni senza degradare le prestazioni: ogni thread del sistema operativo occupa centinaia di KB di stack e richiede un cambio di contesto gestito dal sistema operativo per essere eseguito, mentre le task di Tokio pesano pochissimo e vengono moltiplicate su un piccolo pool di thread reali. Il nostro stress test con 100 client e ~26 MB di RAM totale sarebbe stato difficile da ottenere con un modello a thread dedicati.
**D: Cosa succede se due client tentano di registrarsi con lo stesso username esattamente nello stesso istante?**
R: Anche se il nostro codice applicativo controllasse "l'username esiste già?" prima di inserire, in teoria due richieste concorrenti potrebbero superare quel controllo entrambe nello stesso istante. Per questo il vincolo `UNIQUE` è imposto anche a livello di database SQLite: se la seconda `INSERT` arrivasse comunque, SQLite stesso la rifiuterebbe, e il nostro codice la intercetta come `DbError::UserAlreadyExists`. È un esempio di difesa "in profondità": non ci fidiamo solo del controllo applicativo, ma mettiamo un vincolo anche al livello più basso.
**D: Perché la soglia di movimento è di 5 metri e non un altro valore?**
R: È un compromesso empirico tra due rischi opposti: una soglia troppo bassa (vicina a 0) genererebbe falsi positivi per il rumore fisiologico del segnale GPS anche a veicolo fermo; una soglia troppo alta rischierebbe di non rilevare spostamenti reali ma piccoli (per esempio una manovra di parcheggio). 5 metri filtra il rumore tipico di un ricevitore GPS civile senza perdere movimenti significativi.