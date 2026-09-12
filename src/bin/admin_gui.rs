use eframe::egui;
use futures::{SinkExt, StreamExt};
use georust::db::init_db;
use georust::db::positions::get_positions_by_user;
use georust::db::users::get_all_users;
use georust::gps::analytics::{analyze_movement, filter_positions_by_timerange};
use georust::models::{MovementAnalytics, TimeRange, UserState};
use georust::network::protocol::{ClientMessage, ServerMessage};
use rusqlite::Connection;
use std::fs;
use chrono::Utc; 
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio_util::codec::{Framed, LinesCodec};

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 650.0])
            .with_title("Console Amministratore Flotta"),
        ..Default::default()
    };

    eframe::run_native(
        "GeoRust Admin GUI",
        options,
        Box::new(|_cc| Box::new(AdminApp::default())),
    )
}

#[derive(PartialEq)]
enum ActiveTab {
    FleetStatus,
    MovementAnalytics,
    Messaging,
    CpuPerformance,
}

struct AdminApp {
    // Runtime Tokio dedicato, eseguito in background rispetto al thread della GUI. La connessione TCP verso il server e l'I/O di rete
    // avvengono tutti come task async su questo runtime.
    runtime: tokio::runtime::Runtime,

    active_tab: ActiveTab,

    // Database connection
    db_conn: Option<Connection>,

    // Fleet state
    users_state: Vec<(String, UserState, Option<f64>, Option<f64>, String)>,

    // Analytics state
    selected_user: String,
    selected_range: TimeRange,
    analytics_result: Option<MovementAnalytics>,

    // Messaging state e persistent connection
    server_addr: String,
    target_user: String,
    message_text: String,
    is_broadcast: bool,
    messaging_logs: Vec<String>,
    is_connected: bool,

    tx_to_server: Option<UnboundedSender<String>>,
    rx_from_server: Option<UnboundedReceiver<String>>,
    cpu_logs: String,
    last_auto_refresh: std::time::Instant,
}

impl Default for AdminApp {
    fn default() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(2)
            .thread_name("georust-admin-rt")
            .build()
            .expect("Impossibile avviare il runtime Tokio in background");

        let conn = init_db("georust.db").ok();
        let mut app = Self {
            runtime,
            active_tab: ActiveTab::FleetStatus,
            db_conn: conn,
            users_state: Vec::new(),
            selected_user: String::new(),
            selected_range: TimeRange::Today,
            analytics_result: None,
            server_addr: "127.0.0.1:8080".to_string(),
            target_user: String::new(),
            message_text: String::new(),
            is_broadcast: true,
            messaging_logs: vec!["Benvenuto nella Console Amministratore GeoRust.".to_string()],
            is_connected: false,
            tx_to_server: None,
            rx_from_server: None,
            cpu_logs: String::new(),
            last_auto_refresh: std::time::Instant::now(),
        };
        app.refresh_fleet();
        app.refresh_cpu_logs();
        app.ensure_connected();
        app
    }
}

impl AdminApp {
    fn ensure_connected(&mut self) {
        if self.is_connected {
            return;
        }

        let addr = self.server_addr.clone();

        // Connessione TCP asincrona: unica operazione per cui blocchiamo brevemente il thread della GUI (come nella versione precedente),
        // ma l'I/O sottostante è non bloccante grazie a Tokio.
        let stream = match self.runtime.block_on(TcpStream::connect(&addr)) {
            Ok(s) => s,
            Err(_) => return,
        };

        let framed = Framed::new(stream, LinesCodec::new());
        let (mut sink, mut stream_reader) = framed.split();

        let (tx_net_out, mut rx_net_out) = unbounded_channel::<String>();
        let (tx_net_in, rx_net_in) = unbounded_channel::<String>();

        // Task legge dal socket e inoltra le righe alla GUI
        self.runtime.spawn(async move {
            while let Some(result) = stream_reader.next().await {
                match result {
                    Ok(msg) => {
                        if tx_net_in.send(msg).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Task scrive sul socket i messaggi prodotti dalla GUI
        self.runtime.spawn(async move {
            while let Some(msg) = rx_net_out.recv().await {
                if sink.send(msg).await.is_err() {
                    break;
                }
            }
        });

        // Registrazione/Login sessione Admin
        let login_msg = ClientMessage::Login {
            username: "ADMIN_CONSOLE".to_string(),
            password_hash: "admin_pass".to_string(),
        };

        if let Ok(json) = serde_json::to_string(&login_msg) {
            let _ = tx_net_out.send(json);
        }

        self.is_connected = true;
        self.tx_to_server = Some(tx_net_out);
        self.rx_from_server = Some(rx_net_in);
        self.messaging_logs.push("Connesso al Server TCP come ADMIN_CONSOLE.".into());
    }

    fn poll_incoming_messages(&mut self) {
        if let Some(ref mut rx) = self.rx_from_server {
            while let Ok(line) = rx.try_recv() {
                if let Ok(server_msg) = serde_json::from_str::<ServerMessage>(&line) {
                    match server_msg {
                        // Aggiorna lo stato utente in tempo reale tramite TCP
                        ServerMessage::UserStatus { username, state } => {
                            if let Some(entry) = self.users_state.iter_mut().find(|(u, _, _, _, _)| *u == username) {
                                entry.1 = state;
                            }
                        }
                        ServerMessage::DirectText { from, text, .. } => {
                            if from.starts_with("ADMIN_CONSOLE") {
                                continue;
                            }
                            self.messaging_logs.push(format!("[RICEVUTO DA {}]: {}", from, text));
                        }
                        ServerMessage::BroadcastText { from, text } => {
                            if from.starts_with("ADMIN_CONSOLE") {
                                continue;
                            }
                            self.messaging_logs.push(format!("[BROADCAST DA {}]: {}", from, text));
                        }
                        ServerMessage::AuthResult { success: _, msg } => {
                            self.messaging_logs.push(format!("Info Server: {}", msg));
                        }
                        ServerMessage::Error { reason } => {
                            self.messaging_logs.push(format!("[ERRORE]: {}", reason));
                        }
                    }
                }
            }
        }
    }

    fn refresh_fleet(&mut self) {
        // Salva gli stati correnti ricevuti via TCP prima di ricaricare la lista
        let prev_states: std::collections::HashMap<String, UserState> = self
            .users_state
            .iter()
            .map(|(u, s, _, _, _)| (u.clone(), s.clone()))
            .collect();

        self.users_state.clear();
        if let Some(ref conn) = self.db_conn {
            if let Ok(users) = get_all_users(conn) {
                for user in users {
                    // Recupera solo coordinate e timestamp dall'ultima posizione in DB
                    let (lat_opt, lon_opt, ts_str) =
                        match get_positions_by_user(conn, &user.username) {
                            Ok(positions) if !positions.is_empty() => {
                                let last = positions.last().unwrap();
                                (Some(last.lat), Some(last.lon), last.timestamp.clone())
                            }
                            _ => (None, None, "N/A".to_string()),
                        };

                    // Lo stato viene esclusivamente dai messaggi UserStatus TCP del server
                    // Se non ancora ricevuto nessun messaggio per questo utente → Sconnesso
                    let state = prev_states
                        .get(&user.username)
                        .cloned()
                        .unwrap_or(UserState::Sconnesso);

                    self.users_state
                        .push((user.username, state, lat_opt, lon_opt, ts_str));
                }
            }
        }
    }



    fn run_analytics(&mut self) {
        // digitato esplicitamente un nome utente da cercare.
        if self.selected_user.trim().is_empty() {
            self.analytics_result = None;
            return;
        }
        if let Some(ref conn) = self.db_conn {
            if let Ok(positions) = get_positions_by_user(conn, self.selected_user.trim()) {
                let now = Utc::now();
                let filtered = filter_positions_by_timerange(&positions, self.selected_range, now);
                let filtered_pos: Vec<_> = filtered.into_iter().cloned().collect();
                self.analytics_result = Some(analyze_movement(&filtered_pos));
            } else {
                self.analytics_result = None;
            }
        }
    }

    fn refresh_cpu_logs(&mut self) {
        if let Ok(content) = fs::read_to_string("cpu_log.txt") {
            let lines: Vec<&str> = content.lines().collect();
            let start = lines.len().saturating_sub(30);
            self.cpu_logs = lines[start..].join("\n");
        } else {
            self.cpu_logs = "File cpu_log.txt non ancora generato dal server.".to_string();
        }
    }

    fn send_message(&mut self) {
        if self.message_text.trim().is_empty() {
            return;
        }

        self.ensure_connected();

        let txt = self.message_text.trim().to_string();
        let target = self.target_user.trim().to_string();

        let server_msg = if self.is_broadcast {
            ServerMessage::BroadcastText {
                from: "ADMIN_CONSOLE".to_string(),
                text: txt.clone(),
            }
        } else {
            if target.is_empty() {
                self.messaging_logs
                    .push("Inserire un destinatario per il messaggio privato.".into());
                return;
            }
            ServerMessage::DirectText {
                target_user: target.clone(),
                from: "ADMIN_CONSOLE".to_string(),
                text: txt.clone(),
            }
        };

        if let Ok(json) = serde_json::to_string(&server_msg) {
            if let Some(ref tx) = self.tx_to_server {
                if tx.send(json).is_ok() {
                    let log_entry = if self.is_broadcast {
                        format!("[BROADCAST INVIATO]: {}", txt)
                    } else {
                        format!("[PRIVATO INVIATO a {}]: {}", target, txt)
                    };
                    self.messaging_logs.push(log_entry);
                    self.message_text.clear();
                } else {
                    self.is_connected = false;
                    self.messaging_logs
                        .push("Connessione persa. Riconnessione al prossimo invio.".into());
                }
            }
        }
    }
}

impl eframe::App for AdminApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(500));
        self.poll_incoming_messages();

        // Aggiornamento automatico ogni 2 secondi senza dover premere "Aggiorna"
        if self.last_auto_refresh.elapsed() >= Duration::from_secs(2) {
            self.refresh_fleet();
            self.refresh_cpu_logs();
            self.run_analytics();
            self.last_auto_refresh = std::time::Instant::now();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Dashboard Amministratore Flotta");
            ui.separator();

            // Navigation Bar
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(self.active_tab == ActiveTab::FleetStatus, "Flotta")
                    .clicked()
                {
                    self.active_tab = ActiveTab::FleetStatus;
                    self.refresh_fleet();
                }
                if ui
                    .selectable_label(
                        self.active_tab == ActiveTab::MovementAnalytics,
                        "Analisi Movimento",
                    )
                    .clicked()
                {
                    self.active_tab = ActiveTab::MovementAnalytics;
                }
                if ui
                    .selectable_label(self.active_tab == ActiveTab::Messaging, "Messaggistica")
                    .clicked()
                {
                    self.active_tab = ActiveTab::Messaging;
                }
                if ui
                    .selectable_label(
                        self.active_tab == ActiveTab::CpuPerformance,
                        "Prestazioni CPU",
                    )
                    .clicked()
                {
                    self.active_tab = ActiveTab::CpuPerformance;
                    self.refresh_cpu_logs();
                }
            });

            ui.separator();

            match self.active_tab {
                ActiveTab::FleetStatus => {
                    ui.horizontal(|ui| {
                        ui.label("Lista Veicoli e Stato Operativo");
                        if ui.button("Aggiorna").clicked() {
                            self.refresh_fleet();
                        }
                    });
                    ui.add_space(8.0);

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        egui::Grid::new("fleet_grid")
                            .striped(true)
                            .min_col_width(120.0)
                            .show(ui, |ui| {
                                ui.heading("Utente / Veicolo");
                                ui.heading("Stato");
                                ui.heading("Latitudine");
                                ui.heading("Longitudine");
                                ui.heading("Ultimo Aggiornamento");
                                ui.end_row();

                                for (uname, state, lat, lon, ts) in &self.users_state {
                                    ui.label(uname);

                                    let (state_str, color) = match state {
                                        UserState::InMovimento => {
                                            ("In Movimento", egui::Color32::GREEN)
                                        }
                                        UserState::Fermo => ("Fermo", egui::Color32::YELLOW),
                                        UserState::Sconnesso => ("Sconnesso", egui::Color32::RED),
                                    };

                                    ui.colored_label(color, state_str);
                                    ui.label(
                                        lat.map_or("N/D".to_string(), |v| format!("{:.4}", v)),
                                    );
                                    ui.label(
                                        lon.map_or("N/D".to_string(), |v| format!("{:.4}", v)),
                                    );
                                    ui.label(ts);
                                    ui.end_row();
                                }
                            });
                    });
                }
                ActiveTab::MovementAnalytics => {
                    ui.heading("Report Movimento Veicoli");
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label("Utente:");
                        ui.text_edit_singleline(&mut self.selected_user);

                        ui.label("Intervallo:");
                        egui::ComboBox::from_id_source("timerange_combo")
                            .selected_text(match self.selected_range {
                                TimeRange::Today => "Oggi",
                                TimeRange::ThisWeek => "Settimana Corrente",
                                TimeRange::ThisMonth => "Mese Corrente",
                                TimeRange::AllTime => "Tutto lo Storico",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.selected_range,
                                    TimeRange::Today,
                                    "Oggi",
                                );
                                ui.selectable_value(
                                    &mut self.selected_range,
                                    TimeRange::ThisWeek,
                                    "Settimana Corrente",
                                );
                                ui.selectable_value(
                                    &mut self.selected_range,
                                    TimeRange::ThisMonth,
                                    "Mese Corrente",
                                );
                                ui.selectable_value(
                                    &mut self.selected_range,
                                    TimeRange::AllTime,
                                    "Tutto lo Storico",
                                );
                            });

                        if ui.button("Calcola Analisi").clicked() {
                            self.run_analytics();
                        }
                    });

                    ui.add_space(12.0);

                    if let Some(ref stats) = self.analytics_result {
                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            ui.label(format!(
                                "Tragitto Totale: {:.2} km",
                                stats.total_distance_km
                            ));
                            ui.label(format!(
                                "Velocità Media: {:.2} km/h",
                                stats.average_speed_kmh
                            ));
                            ui.label(format!(
                                "Tempo in Movimento: {}h {}m {}s",
                                stats.moving_duration_secs / 3600,
                                (stats.moving_duration_secs % 3600) / 60,
                                stats.moving_duration_secs % 60
                            ));
                            ui.label(format!(
                                "Tempo di Sosta (Pause): {}h {}m {}s",
                                stats.stopped_duration_secs / 3600,
                                (stats.stopped_duration_secs % 3600) / 60,
                                stats.stopped_duration_secs % 60
                            ));
                        });
                    } else {
                        ui.label("Inserire un nome utente valido");
                    }
                }
                ActiveTab::Messaging => {
                    ui.heading("Centro Messaggistica Flotta");
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label("Indirizzo Server:");
                        ui.text_edit_singleline(&mut self.server_addr);
                        if ui.button("Connetti").clicked() {
                            self.is_connected = false;
                            self.ensure_connected();
                        }
                        if self.is_connected {
                            ui.colored_label(egui::Color32::GREEN, "Connesso");
                        } else {
                            ui.colored_label(egui::Color32::RED, "Disconnesso");
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.checkbox(
                            &mut self.is_broadcast,
                            "Invia in Broadcast a TUTTI i veicoli",
                        );
                        if !self.is_broadcast {
                            ui.label("Destinatario:");
                            ui.text_edit_singleline(&mut self.target_user);
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Messaggio:");
                        ui.text_edit_singleline(&mut self.message_text);
                        if ui.button("Invia Messaggio").clicked() {
                            self.send_message();
                        }
                    });

                    ui.separator();
                    ui.label("Log Messaggi (Inviati e Ricevuti):");

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for log in &self.messaging_logs {
                            ui.label(log);
                        }
                    });
                }
                ActiveTab::CpuPerformance => {
                    ui.horizontal(|ui| {
                        ui.heading("Prestazioni CPU Server (cpu_log.txt)");
                        if ui.button("Ricarica Log").clicked() {
                            self.refresh_cpu_logs();
                        }
                    });
                    ui.add_space(8.0);

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.code(&self.cpu_logs);
                    });
                }
            }
        });
    }
}
