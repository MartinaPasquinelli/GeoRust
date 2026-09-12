# Documento di Analisi

## GeoRust: Sistema di geolocalizzazione per una flotta di veicoli

**Gruppo 15: Pasquinelli, Danesi, Casale, Giordano**

---

## Descrizione

Nel seguente documento viene svolta l'analisi dei requisiti dell'applicazione GeoRust, distinguendo tra requisiti funzionali e non funzionali e motivandone la scelta in base alle esigenze del sistema. Viene inoltre fornito un riepilogo riassuntivo dei requisiti individuati e il diagramma dei casi d'uso, così da offrire una visione d'insieme delle funzionalità richieste e dei vincoli progettuali da rispettare.

---

## Abstract

GeoRust è un'applicazione client/server sviluppata in Rust per simulare e monitorare una flotta di veicoli. Il sistema permette agli utenti di registrarsi e autenticarsi, inviare periodicamente la propria posizione geografica e comunicare con il server tramite messaggi di testo. Il server può inviare messaggi in broadcast a tutti gli utenti connessi oppure a un utente specifico, analizzare lo storico degli spostamenti di ciascun utente e visualizzare i log di CPU.

---

## Analisi dei requisiti

### Requisiti funzionali

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

## Requisiti non fuzionali

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

## Riassunto

| Requisiti funzionali                         | Requisiti non funzionali           |
| -------------------------------------------- | ---------------------------------- |
| Registrazione e login                        | Architettura client/server         |
| Gestione utenti                              | Sviluppo in Rust                   |
| Invio posizione ogni 30 s                    | Multipiattaforma                   |
| Gestione stati (sconnesso, fermo, movimento) | Prestazioni CPU                    |
| Simulazione del movimento                    | Dimensione ridotta dell'eseguibile |
| Memorizzazione coordinate                    | Affidabilità                       |
| Analisi tragitto                             | Logging persistente                |
| Calcolo velocità media                       | Accuratezza temporale              |
| Calcolo tempi di movimento e sosta           | Scalabilità                        |
| Analisi per giorno/settimana/mese            | Manutenibilità                     |
| Messaggi server→client                       | Sicurezza delle password           |
| Messaggi client→server                       |                                    |
| Gestione utenti multipli                     |                                    |
| Logging del tempo CPU                        |                                    |

---

## Use Case

![](/doc/img/UseCase.png)
