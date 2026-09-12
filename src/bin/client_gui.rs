use eframe::egui;
use futures::{SinkExt, StreamExt};
use georust::network::protocol::{ClientMessage, ServerMessage};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::time::{sleep, timeout, Duration as TokioDuration};
use tokio_util::codec::{Framed, LinesCodec};

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([750.0, 550.0])
            .with_title("Benvenuto su GeoRust!"),
        ..Default::default()
    };

    eframe::run_native(
        "GeoRust Client GUI",
        options,
        Box::new(|_cc| Box::new(ClientApp::default())),
    )
}

struct ClientApp {
    // Runtime Tokio dedicato: gira in background su thread propri, separati
    // da quello della GUI (che egui richiede per sé). Tutta la logica di
    // rete e di simulazione GPS viene eseguita come task async su questo
    // runtime; la GUI comunica con essi tramite canali tokio::sync::mpsc.
    runtime: tokio::runtime::Runtime,

    // Auth inputs
    server_addr: String,
    username: String,
    password: String,
    is_login_mode: bool,

    // Status
    is_authenticated: bool,
    auth_error: Option<String>,

    // GPS Simulation
    last_lat: Option<f64>,
    last_lon: Option<f64>,
    is_stopped: Arc<AtomicBool>,

    // Chat
    message_to_send: String,
    chat_logs: Vec<String>,

    // Canali verso/da i task in background
    tx_to_server: Option<UnboundedSender<String>>,
    rx_from_server: Option<UnboundedReceiver<String>>,
    rx_gps: Option<UnboundedReceiver<(f64, f64)>>,
}

impl Default for ClientApp {
    fn default() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .thread_name("georust-client-rt")
            .build()
            .expect("Impossibile avviare il runtime Tokio in background");

        Self {
            runtime,
            server_addr: "127.0.0.1:8080".to_string(),
            username: "driver_gui".to_string(),
            password: "password123".to_string(),
            is_login_mode: true,
            is_authenticated: false,
            auth_error: None,
            last_lat: None,
            last_lon: None,
            is_stopped: Arc::new(AtomicBool::new(false)),
            message_to_send: String::new(),
            chat_logs: Vec::new(),
            tx_to_server: None,
            rx_from_server: None,
            rx_gps: None,
        }
    }
}

impl ClientApp {
    fn connect_and_authenticate(&mut self) {
        self.auth_error = None;
        let clean_user = self.username.trim().to_string();
        let clean_pass = self.password.trim().to_string();

        if clean_user.is_empty() || clean_pass.is_empty() {
            self.auth_error = Some("Username e password non possono essere vuoti.".into());
            return;
        }

        let addr = self.server_addr.clone();
        let is_login = self.is_login_mode;

        let auth_msg = if is_login {
            ClientMessage::Login {
                username: clean_user.clone(),
                password_hash: clean_pass,
            }
        } else {
            ClientMessage::Register {
                username: clean_user.clone(),
                password_hash: clean_pass,
            }
        };

        let auth_json = match serde_json::to_string(&auth_msg) {
            Ok(j) => j,
            Err(e) => {
                self.auth_error = Some(format!("Errore di serializzazione: {}", e));
                return;
            }
        };

        // Connessione TCP asincrona + invio/attesa della risposta di autenticazione.
        let auth_outcome = self.runtime.block_on(async {
            let stream = TcpStream::connect(&addr).await?;
            let mut framed = Framed::new(stream, LinesCodec::new());

            framed
                .send(auth_json)
                .await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

            match timeout(TokioDuration::from_secs(3), framed.next()).await {
                Ok(Some(Ok(line))) => Ok((framed, line)),
                Ok(Some(Err(e))) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
                Ok(None) => Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "Connessione chiusa dal server",
                )),
                Err(_) => Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "Timeout nell'attesa di risposta dal server",
                )),
            }
        });

        let (framed, line) = match auth_outcome {
            Ok(v) => v,
            Err(e) => {
                self.auth_error = Some(format!("Impossibile connettersi al server: {}", e));
                return;
            }
        };

        match serde_json::from_str::<ServerMessage>(&line) {
            Ok(ServerMessage::AuthResult { success, msg }) => {
                if !success {
                    self.auth_error = Some(format!("Autenticazione fallita: {}", msg));
                    return;
                }

                self.is_authenticated = true;
                self.chat_logs
                    .push(format!("Autenticazione riuscita! Utente: {}", clean_user));

                let (mut sink, mut stream) = framed.split();

                let (tx_net_out, mut rx_net_out) = unbounded_channel::<String>();
                let (tx_net_in, rx_net_in) = unbounded_channel::<String>();
                let (tx_gps, rx_gps) = unbounded_channel::<(f64, f64)>();

                // Task async: legge dal socket e inoltra le righe ricevute alla GUI
                self.runtime.spawn(async move {
                    while let Some(result) = stream.next().await {
                        match result {
                            Ok(line) => {
                                if tx_net_in.send(line).is_err() {
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                });

                // Task async: scrive sul socket i messaggi prodotti dalla GUI
                self.runtime.spawn(async move {
                    while let Some(msg) = rx_net_out.recv().await {
                        if sink.send(msg).await.is_err() {
                            break;
                        }
                    }
                });

                self.tx_to_server = Some(tx_net_out.clone());
                self.rx_from_server = Some(rx_net_in);
                self.rx_gps = Some(rx_gps);

                // Task simulatore GPS, invia la posizione ogni 30s come da specifica
                self.start_gps_simulator(tx_net_out, tx_gps);
            }
            Ok(other) => {
                self.auth_error = Some(format!("Risposta inattesa dal server: {:?}", other));
            }
            Err(e) => {
                self.auth_error = Some(format!("Errore nella lettura della risposta: {}", e));
            }
        }
    }

    fn start_gps_simulator(
        &mut self,
        tx_out: UnboundedSender<String>,
        tx_gps: UnboundedSender<(f64, f64)>,
    ) {
        let is_stopped = self.is_stopped.clone();

        // Carica le coordinate dal file CSV
        let route_coords = Self::load_route_from_csv("data/route.csv");

        self.runtime.spawn(async move {
            let mut index = 0;
            loop {
                if !is_stopped.load(Ordering::Relaxed) {
                    let (lat, lon) = route_coords[index % route_coords.len()];
                    index += 1;

                    let _ = tx_gps.send((lat, lon));
                    let update_msg = ClientMessage::UpdatePosition { lat, lon };

                    if let Ok(json) = serde_json::to_string(&update_msg) {
                        if tx_out.send(json).is_err() {
                            break;
                        }
                    }
                } else {
                    // When stopped, do not send any GPS data to the GUI
                    // (previously sent last position as placeholder).
                }

                // Intervallo richiesto dalla traccia: invio posizione ogni 30 secondi.
                sleep(TokioDuration::from_secs(30)).await;
            }
        });
    }

    // Legge le coordinate dal file CSV. Se il file non esiste, usa un fallback hardcoded.
    fn load_route_from_csv(path: &str) -> Vec<(f64, f64)> {
        if let Ok(content) = std::fs::read_to_string(path) {
            let coords: Vec<(f64, f64)> = content
                .lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.trim().split(',').collect();
                    if parts.len() == 2 {
                        if let (Ok(lat), Ok(lon)) =
                            (parts[0].parse::<f64>(), parts[1].parse::<f64>())
                        {
                            return Some((lat, lon));
                        }
                    }
                    None
                })
                .collect();
            if !coords.is_empty() {
                return coords;
            }
        }
        // Fallback se il file non è leggibile
        vec![
            (45.464203, 9.189982),
            (45.464520, 9.190100),
            (45.465000, 9.191000),
            (45.466000, 9.192000),
            (45.467000, 9.193000),
        ]
    }

    fn send_chat_message(&mut self) {
        if self.message_to_send.trim().is_empty() {
            return;
        }
        let txt = self.message_to_send.trim().to_string();
        let msg = ClientMessage::SendDirectText { text: txt.clone() };

        if let Ok(json) = serde_json::to_string(&msg) {
            if let Some(ref tx) = self.tx_to_server {
                if tx.send(json).is_ok() {
                    self.chat_logs.push(format!("[TU -> SERVER]: {}", txt));
                    self.message_to_send.clear();
                }
            }
        }
    }

    fn poll_incoming_messages(&mut self) {
        if let Some(ref mut rx) = self.rx_from_server {
            while let Ok(line) = rx.try_recv() {
                if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&line) {
                    match server_msg {
                        ServerMessage::DirectText { from, text, .. } => {
                            if from == self.username
                                || from.starts_with(&format!("{} (", self.username))
                            {
                                continue;
                            }
                            self.chat_logs
                                .push(format!("[DA {} (Privato)]: {}", from, text));
                        }
                        ServerMessage::BroadcastText { from, text } => {
                            if from == self.username
                                || from.starts_with(&format!("{} (", self.username))
                            {
                                continue;
                            }
                            self.chat_logs
                                .push(format!("[BROADCAST da {}]: {}", from, text));
                        }
                        ServerMessage::AuthResult { success: _, msg } => {
                            self.chat_logs.push(format!("Info Server: {}", msg));
                        }
                        ServerMessage::Error { reason } => {
                            self.chat_logs.push(format!("[ERRORE SERVER]: {}", reason));
                        }
                        _ => {}
                    }
                }
            }
        }

        if let Some(ref mut rx) = self.rx_gps {
            while let Ok((lat, lon)) = rx.try_recv() {
                self.last_lat = Some(lat);
                self.last_lon = Some(lon);
            }
        }
    }
}

impl eframe::App for ClientApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(200));
        self.poll_incoming_messages();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🚛 Benvenuto su GeoRust");
            ui.separator();

            if !self.is_authenticated {
                ui.heading("Autenticazione Veicolo");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    ui.label("Indirizzo Server:");
                    ui.text_edit_singleline(&mut self.server_addr);
                });

                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.is_login_mode, true, "Login");
                    ui.selectable_value(&mut self.is_login_mode, false, "Registrazione");
                });

                ui.horizontal(|ui| {
                    ui.label("Username:");
                    ui.text_edit_singleline(&mut self.username);
                });

                ui.horizontal(|ui| {
                    ui.label("Password:");
                    ui.add(egui::TextEdit::singleline(&mut self.password).password(true));
                });

                ui.add_space(8.0);

                if ui.button("Connetti e Autenticati").clicked() {
                    self.connect_and_authenticate();
                }

                if let Some(ref err) = self.auth_error {
                    ui.colored_label(egui::Color32::RED, err);
                }
            } else {
                ui.horizontal(|ui| {
                    ui.label(format!("Connesso come: {}", self.username));
                    ui.label("ONLINE");
                });
                ui.separator();

                ui.heading("Simulazione Geolocalizzazione GPS");
                ui.label("Invio automatico delle coordinate al server ogni 30 secondi...");
                ui.add_space(8.0);

                // Tasto per simulare lo stato "Fermo"
                let currently_stopped = self.is_stopped.load(Ordering::Relaxed);
                let btn_label = if currently_stopped {
                    "Riprendi Movimento"
                } else {
                    "Simula Fermo"
                };
                ui.horizontal(|ui| {
                    if ui.button(btn_label).clicked() {
                        self.is_stopped.store(!currently_stopped, Ordering::Relaxed);
                    }
                    if currently_stopped {
                        ui.colored_label(
                            egui::Color32::YELLOW,
                            "VEICOLO FERMO (simulazione in corso)",
                        );
                    } else {
                        // When stopped, do not send any GPS data to the GUI
                        // (previously sent the last position as a placeholder).
                        ui.colored_label(egui::Color32::GREEN, "VEICOLO IN MOVIMENTO");
                    }
                });
                ui.add_space(4.0);

                if let (Some(lat), Some(lon)) = (self.last_lat, self.last_lon) {
                    ui.group(|ui| {
                        ui.label(format!(
                            "Ultima Posizione Trasmessa: lat={:.6}, lon={:.6}",
                            lat, lon
                        ));
                    });
                } else {
                    ui.label("Avvio trasmissione coordinate in corso...");
                }

                ui.add_space(8.0);
                ui.separator();
                ui.heading("Chat & Messaggistica con il Server");

                ui.horizontal(|ui| {
                    ui.label("Messaggio:");
                    ui.text_edit_singleline(&mut self.message_to_send);
                    if ui.button("Invia").clicked() {
                        self.send_chat_message();
                    }
                });

                ui.separator();
                ui.label("Cronologia Chat:");

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for log in &self.chat_logs {
                        ui.label(log);
                    }
                });
            }
        });
    }
}
