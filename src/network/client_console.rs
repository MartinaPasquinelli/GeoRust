use std::io;
use tokio::sync::mpsc;
use tokio::task;

pub fn start_client_console(tx_stdin: mpsc::Sender<String>) {
    task::spawn_blocking(move || {
        let stdin = io::stdin();
        let mut line = String::new();
        loop {
            line.clear();
            if let Ok(n) = stdin.read_line(&mut line) {
                if n == 0 { break; }
                let input = line.trim().to_string();
                if !input.is_empty() {
                    if tx_stdin.blocking_send(input).is_err() { 
                        break; 
                    }
                }
            } else {
                break;
            }
        }
    });
}
