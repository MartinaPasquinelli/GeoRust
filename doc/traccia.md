## Progetto 2.1: Georuggine: sistema di geolocalizzazione per una flotta di veicoli

Il progetto consiste nella realizzazione di un'applicazione client/server sviluppata in Rust, finalizzata alla gestione della geolocalizzazione e della comunicazione con una flotta di veicoli. Il sistema deve permettere a più utenti di registrarsi, autenticarsi, inviare periodicamente la propria posizione al server e comunicare con esso tramite messaggi di testo.

Ogni utente accede al sistema attraverso una fase di registrazione, nella quale vengono definiti un account e una password. Una volta registrato, l'utente può essere monitorato dal server attraverso coordinate geografiche, gestite in modo emulato. La posizione di ciascun utente viene trasmessa al server ogni 30 secondi, così da consentire il tracciamento continuo degli spostamenti.

Il sistema deve inoltre gestire lo stato di ogni utente. In particolare, un utente può risultare sconnesso, fermo oppure in movimento. La transizione dallo stato “fermo” allo stato “in movimento” avviene quando viene rilevato un cambiamento delle coordinate. Al contrario, il passaggio dallo stato “in movimento” allo stato “fermo” si verifica quando, per almeno tre minuti, la posizione dell’utente non cambia.

Per simulare il movimento dei veicoli è possibile adottare diverse strategie. Una soluzione può consistere nella lettura di un file contenente una sequenza di coordinate e tempi. In alternativa, si può definire un punto di partenza e un punto di arrivo, specificando anche eventuali pause lungo il tragitto. Altre possibilità includono l'inserimento manuale delle coordinate tramite interfaccia oppure l'utilizzo di un generatore pseudo-casuale di posizioni.

Il server deve essere in grado di analizzare il movimento compiuto da uno specifico utente. Le informazioni richieste riguardano il tragitto percorso, la velocità media, la durata complessiva del movimento e la durata delle pause. Tali dati devono poter essere calcolati su intervalli temporali programmabili, come il giorno corrente, la settimana corrente o il mese corrente.

Un'altra funzionalità importante riguarda la comunicazione tra server e utenti. Il server deve poter inviare messaggi di testo sia in broadcast, cioè a tutti gli utenti connessi, sia in modo diretto a un singolo utente. Allo stesso tempo, ogni utente deve poter inviare messaggi testuali al server.

Dal punto di vista tecnico, l'applicazione deve essere eseguibile su almeno due piattaforme diverse tra Windows, Linux, macOS, Android, ChromeOS e iOS. È inoltre richiesto di prestare attenzione alle prestazioni del sistema, in particolare al consumo di tempo CPU e alla dimensione dell'applicativo. Il server deve generare un file di log che riporti, ogni due minuti, i dettagli relativi al tempo di CPU utilizzato. Infine, nel report descrittivo del progetto deve essere indicata anche la dimensione del file eseguibile prodotto.

In sintesi, il progetto richiede lo sviluppo di un sistema distribuito in Rust capace di gestire utenti, posizioni geografiche simulate, stati di movimento, analisi dei percorsi e comunicazione client/server. L'obiettivo è realizzare un'applicazione efficiente, multipiattaforma e ben strutturata, prestando attenzione sia agli aspetti funzionali sia a quelli prestazionali.

---

### Specifiche:

- **Progetto di un sistema di geolocalizzazione**: Realizzare un’applicazione di tipo client/server per gestire la  geolocalizzazione e la comunicazione con una flotta di veicoli

- **Registrazione**: Gli utenti del sistema si registrano con account e password

- **Geolocalizzazione e stato degli utenti**: 
    - Ogni utente è geolocalizzato con le sue coordinate (da gestire in modo emulato)
    - Il server riceve ogni 30 secondi la posizione di ogni utente
    - Ogni utente può essere in uno dei seguenti stati:
        - sconnesso
        - in movimento (la transizione da fermo a in movimento si ha al primo  cambiamento di coordinata)
        - Fermo (la transizione da in movimento a fermo si ha dopo 3 minuti che la coordinata non cambia).

- **Emulazione del movimento**: E’ possibile fare scelte personali per l’emulazione del movimento.
    - Ad esempio una delle seguenti strategie:
        - Attraverso la lettura di un file contenente i dati
        - Impostando il punto e l’istante di partenza e quelli di arrivo, definendo eventuali intervalli di tempo di pausa
        - Attraverso un’interfaccia in cui l’utente scrive le coordinate correnti
        - Utilizzando un generatore pseudo-casuale di coordinate
        - Ecc.ecc.

- **Analisi del movimento**: 
    - Il server può fare una analisi del movimento compiuto da uno  specifico utente disponendo delle seguenti possibili interrogazioni:
        - tragitto percorso
        - velocità media 
        - durata del movimento e delle pause.
    - Le precedenti informazioni possono far riferimento ad intervalli  temporali programmabili:
        - Giorno corrente
        - Settimana corrente
        - Mese corrente.

- **Comunicazione**: 
    - Il server può comunicare con gli utenti mandando un messaggio di  testo in broadcast oppure diretto ad uno specifico utente
    - Ogni utente può inviare un messaggio di testo al server.

- **Specifiche tecniche**:
    - Il programma deve girare su almeno 2 tra le diverse piattaforme  disponibili (Windows, Linux, MacOS, Android, ChromeOS, iOS).
    - Si richiede di porre attenzione alle prestazioni del sistema in termini di consumo di tempo di CPU e di dimensione dell’applicativo. 
    - L’applicazione deve generare un file di log, riportando, ogni 2 minuti, i dettagli sul tempo di CPU utilizzato da parte del server
    - Si richiede inoltre di riportare nel report descrittivo del progetto la dimensione del file eseguibile.

![](/doc/img/Esempio.png)