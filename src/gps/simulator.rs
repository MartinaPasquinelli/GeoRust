use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};

// Estrae le coordinate (lat, lon) da una riga di testo formato "lat,lon".
pub fn parse_coordinate(line: &str) -> Option<(f64, f64)> {
    let parts: Vec<&str> = line.trim().split(',').collect();
    if parts.len() == 2 {
        if let (Ok(lat), Ok(lon)) = (parts[0].trim().parse::<f64>(), parts[1].trim().parse::<f64>()) {
            return Some((lat, lon));
        }
    }
    None
}

// Task asincrono che legge un file e invia le coordinate nel canale mpsc.
pub async fn run_simulation(
    file_path: &str,
    tx: mpsc::Sender<(f64, f64)>,
    interval_ms: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(file_path).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    // intervallo minimo per evitare panico (deve essere > 0)
    let mut ticker = interval(Duration::from_millis(std::cmp::max(1, interval_ms)));

    while let Some(line) = lines.next_line().await? {
        ticker.tick().await;

        if let Some(coords) = parse_coordinate(&line) {
            // Se l'invio fallisce (es. il client si è disconnesso), fermiamo la simulazione
            if tx.send(coords).await.is_err() {
                break;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncWriteExt;

    #[test]
    fn test_parse_coordinate_valid() {
        let coords = parse_coordinate("45.464203,9.189982");
        assert!(coords.is_some());
        let (lat, lon) = coords.unwrap();
        assert!((lat - 45.464203).abs() < 1e-6);
        assert!((lon - 9.189982).abs() < 1e-6);

        // Con spazi
        let coords_spaces = parse_coordinate("  45.0 , 9.0  ");
        assert_eq!(coords_spaces, Some((45.0, 9.0)));
    }

    #[test]
    fn test_parse_coordinate_invalid() {
        assert!(parse_coordinate("").is_none());
        assert!(parse_coordinate("solo_un_valore").is_none());
        assert!(parse_coordinate("45.0,non_un_numero").is_none());
        assert!(parse_coordinate("1,2,3").is_none());
    }

    #[tokio::test]
    async fn test_run_simulation_stream() {
        let temp_path = "temp_test_route.csv";
        let mut f = File::create(temp_path).await.unwrap();
        f.write_all(b"45.0,9.0\n45.1,9.1\n").await.unwrap();
        drop(f);

        let (tx, mut rx) = mpsc::channel(10);
        let handle = tokio::spawn(async move {
            run_simulation(temp_path, tx, 1).await.unwrap();
        });

        let mut received = Vec::new();
        while let Some(coord) = rx.recv().await {
            received.push(coord);
        }
        let _ = handle.await;
        let _ = tokio::fs::remove_file(temp_path).await;

        assert_eq!(received.len(), 2);
        assert_eq!(received[0], (45.0, 9.0));
        assert_eq!(received[1], (45.1, 9.1));
    }
}
