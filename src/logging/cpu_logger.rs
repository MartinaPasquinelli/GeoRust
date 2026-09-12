use cpu_time::ProcessTime;
use std::time::Duration;
use sysinfo::{Pid, System};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::time::sleep;

pub async fn start_cpu_logger() {
    println!("Avvio del task di logging CPU in background...");

    tokio::spawn(async move {
        let mut sys = System::new_all();
        let pid = Pid::from_u32(std::process::id());

        // sysinfo richiede DUE chiamate a refresh_processes() con un intervallo
        // per calcolare il delta CPU correttamente. Una sola chiamata produce 0% o 100%.
        sys.refresh_processes();
        sleep(Duration::from_millis(500)).await;
        sys.refresh_processes();

        let cpu_time_start = ProcessTime::now().as_duration();
        let mut usage_percent_start = 0.0;
        if let Some(process) = sys.process(pid) {
            usage_percent_start = process.cpu_usage();
        }

        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("cpu_log.txt").await {
            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let start_log = format!(
                "\n=== NUOVA SESSIONE SERVER AVVIATA [{}] ===\n[{}] Tempo di CPU iniziale: {:.2?} (Utilizzo: {:.2}%)\n",
                timestamp, timestamp, cpu_time_start, usage_percent_start
            );
            let _ = file.write_all(start_log.as_bytes()).await;
        }

        loop {
            // Dormi per 2 minuti in modo asincrono 
            sleep(Duration::from_secs(120)).await;

            // Aggiorna le informazioni di sistema per calcolare l'uso della CPU
            sys.refresh_processes();

            let mut usage_percent = 0.0;
            if let Some(process) = sys.process(pid) {
                usage_percent = process.cpu_usage();
            }

            // Calcola il tempo di CPU fisico speso dal processo da quando è nato
            let cpu_time = ProcessTime::now().as_duration();

            // Crea o apri in append il file di log
            let mut file = match OpenOptions::new()
                .create(true)
                .append(true)
                .open("cpu_log.txt")
                .await
            {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Errore nell'apertura del file di log CPU: {}", e);
                    continue;
                }
            };

            let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let log_line = format!(
                "[{}] Tempo di CPU totale utilizzato: {:.2?} (Utilizzo nell'ultimo intervallo: {:.2}%)\n",
                timestamp, cpu_time, usage_percent
            );

            if let Err(e) = file.write_all(log_line.as_bytes()).await {
                eprintln!("Errore durante la scrittura del log CPU: {}", e);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sysinfo_double_sample() {
        let mut sys = System::new_all();
        let pid = Pid::from_u32(std::process::id());

        sys.refresh_processes();
        std::thread::sleep(Duration::from_millis(50));
        sys.refresh_processes();

        if let Some(process) = sys.process(pid) {
            let usage = process.cpu_usage();
            // L'utilizzo deve essere un numero valido >= 0
            assert!(usage >= 0.0);
        }
    }

    #[test]
    fn test_cpu_log_line_format() {
        let timestamp = "2026-09-05 12:00:00";
        let cpu_time = Duration::from_millis(150);
        let usage_percent = 2.5;

        let log_line = format!(
            "[{}] Tempo di CPU totale utilizzato: {:.2?} (Utilizzo nell'ultimo intervallo: {:.2}%)\n",
            timestamp, cpu_time, usage_percent
        );

        assert!(log_line.contains("2026-09-05 12:00:00"));
        assert!(log_line.contains("Utilizzo nell'ultimo intervallo: 2.50%"));
    }
}
