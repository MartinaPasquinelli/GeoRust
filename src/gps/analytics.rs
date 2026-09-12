use crate::models::{MovementAnalytics, Position, TimeRange};
use chrono::{DateTime, Datelike, Duration, NaiveDateTime, Utc};

const EARTH_RADIUS_KM: f64 = 6371.0;
const MOVEMENT_THRESHOLD_METERS: f64 = 5.0; // Distanza minima per considerare il veicolo in movimento

// Calcola la distanza in km tra due coordinate geografiche (Formula di Haversine)
pub fn calculate_distance_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();

    let a =
        (dlat / 2.0).sin().powi(2) + lat1_rad.cos() * lat2_rad.cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS_KM * c
}

/// Converte la stringa timestamp del database in DateTime<Utc>
pub fn parse_db_timestamp(ts: &str) -> Option<DateTime<Utc>> {
    let clean = ts.trim();
    // Prova con ISO 8601 / RFC 3339 con fuso orario
    if let Ok(dt) = DateTime::parse_from_rfc3339(clean) {
        return Some(dt.with_timezone(&Utc));
    }
    // Prova formato SQLite con millisecondi/frazioni: "YYYY-MM-DD HH:MM:SS.ffffff"
    if let Ok(naive) = NaiveDateTime::parse_from_str(clean, "%Y-%m-%d %H:%M:%S%.f") {
        return Some(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc));
    }
    // Prova formato SQLite standard: "YYYY-MM-DD HH:MM:SS"
    if let Ok(naive) = NaiveDateTime::parse_from_str(clean, "%Y-%m-%d %H:%M:%S") {
        return Some(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc));
    }
    // Prova formato ISO senza offset: "YYYY-MM-DDTHH:MM:SS%.f"
    if let Ok(naive) = NaiveDateTime::parse_from_str(clean, "%Y-%m-%dT%H:%M:%S%.f") {
        return Some(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc));
    }

    if let Ok(naive) = NaiveDateTime::parse_from_str(clean, "%Y-%m-%dT%H:%M:%S") {
        return Some(DateTime::<Utc>::from_naive_utc_and_offset(naive, Utc));
    }
    None
}

/// Filtra una lista di posizioni in base all'intervallo temporale scelto
pub fn filter_positions_by_timerange<'a>(
    positions: &'a [Position],
    range: TimeRange,
    now: DateTime<Utc>,
) -> Vec<&'a Position> {
    positions
        .iter()
        .filter(|p| {
            if range == TimeRange::AllTime {
                return true;
            }
            if let Some(ts) = parse_db_timestamp(&p.timestamp) {
                match range {
                    TimeRange::Today => ts.date_naive() == now.date_naive(),
                    TimeRange::ThisWeek => {
                        let days_from_monday = now.weekday().num_days_from_monday() as i64;
                        let start_of_week = (now - Duration::days(days_from_monday)).date_naive();
                        ts.date_naive() >= start_of_week
                    }
                    TimeRange::ThisMonth => ts.year() == now.year() && ts.month() == now.month(),
                    TimeRange::AllTime => true,
                }
            } else {
                // Se non è parsabile, includi comunque in AllTime per evitare perdite di dati
                true
            }
        })
        .collect()
}

/// Calcola tragitto, velocità media, durata movimento e durata pause da una sequenza di posizioni
pub fn analyze_movement(positions: &[Position]) -> MovementAnalytics {
    if positions.len() < 2 {
        return MovementAnalytics {
            total_distance_km: 0.0,
            average_speed_kmh: 0.0,
            moving_duration_secs: 0,
            stopped_duration_secs: 0,
        };
    }

    let mut total_distance_km = 0.0;
    let mut moving_duration_secs = 0;
    let mut stopped_duration_secs = 0;

    for window in positions.windows(2) {
        let p1 = &window[0];
        let p2 = &window[1];

        let dist_km = calculate_distance_km(p1.lat, p1.lon, p2.lat, p2.lon);
        let dist_meters = dist_km * 1000.0;

        let delta_secs = match (
            parse_db_timestamp(&p1.timestamp),
            parse_db_timestamp(&p2.timestamp),
        ) {
            (Some(t1), Some(t2)) => (t2 - t1).num_seconds().max(0),
            _ => 30, // Default 30 secondi se il timestamp non è parsabile
        };

        if dist_meters > MOVEMENT_THRESHOLD_METERS {
            total_distance_km += dist_km;
            moving_duration_secs += delta_secs;
        } else {
            stopped_duration_secs += delta_secs;
        }
    }

    let average_speed_kmh = if moving_duration_secs > 0 {
        total_distance_km / (moving_duration_secs as f64 / 3600.0)
    } else {
        0.0
    };

    MovementAnalytics {
        total_distance_km,
        average_speed_kmh,
        moving_duration_secs,
        stopped_duration_secs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haversine_distance_known_points() {
        // Distanza tra Roma (Colosseo) e Milano (Duomo): circa 477 km in linea d'aria
        let roma_lat = 41.8902;
        let roma_lon = 12.4922;
        let milano_lat = 45.4641;
        let milano_lon = 9.1919;

        let dist = calculate_distance_km(roma_lat, roma_lon, milano_lat, milano_lon);
        assert!((dist - 477.0).abs() < 5.0); // Tolleranza 5 km
    }

    #[test]
    fn test_analyze_movement_calculation() {
        let positions = vec![
            Position {
                id: 1,
                username: "driver1".into(),
                lat: 45.0,
                lon: 9.0,
                timestamp: "2026-08-25 10:00:00".into(),
            },
            Position {
                id: 2,
                username: "driver1".into(),
                lat: 45.1,
                lon: 9.0,
                timestamp: "2026-08-25 10:30:00".into(), // +30 min (1800s), ~11.1 km
            },
            Position {
                id: 3,
                username: "driver1".into(),
                lat: 45.1,
                lon: 9.0,
                timestamp: "2026-08-25 10:45:00".into(), // +15 min (900s) fermo
            },
        ];

        let stats = analyze_movement(&positions);
        assert!(stats.total_distance_km > 10.0 && stats.total_distance_km < 12.0);
        assert_eq!(stats.moving_duration_secs, 1800);
        assert_eq!(stats.stopped_duration_secs, 900);
        assert!(stats.average_speed_kmh > 20.0);
    }

    #[test]
    fn test_parse_db_timestamp_formats() {
        assert!(parse_db_timestamp("2026-08-25 10:30:00").is_some());
        assert!(parse_db_timestamp("2026-08-25T10:30:00Z").is_some());
        assert!(parse_db_timestamp("2026-08-25 10:30:00.123456").is_some());
        assert!(parse_db_timestamp("invalid-date").is_none());
    }

    #[test]
    fn test_filter_positions_by_timerange() {
        let fixed_now = DateTime::parse_from_rfc3339("2026-08-25T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let positions = vec![
            Position {
                id: 1,
                username: "u".into(),
                lat: 45.0,
                lon: 9.0,
                timestamp: "2026-08-25 10:00:00".into(), // Oggi
            },
            Position {
                id: 2,
                username: "u".into(),
                lat: 45.0,
                lon: 9.0,
                timestamp: "2026-08-24 10:00:00".into(), // Ieri
            },
            Position {
                id: 3,
                username: "u".into(),
                lat: 45.0,
                lon: 9.0,
                timestamp: "2025-01-01 10:00:00".into(), // Anno scorso
            },
        ];

        let today = filter_positions_by_timerange(&positions, TimeRange::Today, fixed_now);
        assert_eq!(today.len(), 1);
        assert_eq!(today[0].id, 1);

        let all = filter_positions_by_timerange(&positions, TimeRange::AllTime, fixed_now);
        assert_eq!(all.len(), 3);
    }
}
