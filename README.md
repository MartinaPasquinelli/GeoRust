# GeoRust: Vehicle Fleet Geolocation System

**Group 15: Pasquinelli, Danesi, Casale, Giordano**

---

## Description

GeoRust is a **client/server** application developed entirely in **Rust** to simulate and monitor a vehicle fleet in real time. Drivers can register, log in, and periodically send their GPS position. The server stores the history of their movements, automatically determines each vehicle's status (disconnected, stationary, or moving), provides distance and speed analytics, and supports text-based messaging (broadcast or private) between the administrator and vehicles.

The project is built on **Tokio** for asynchronous concurrency and **SQLite** (through `rusqlite`) for data persistence, with particular attention to limiting CPU usage, keeping executable sizes small, and ensuring stability under load.

---

## Main Features

- Registration and login with passwords protected by **SHA-256 hashing with a salt**
- Automatic GPS position updates every **30 seconds** (simulated using a CSV file)
- Automatic vehicle status detection:
  - **Disconnected** → no updates received for more than 60 seconds
  - **Stationary** → position unchanged for at least 3 minutes
  - **Moving** → position changes by more than 5 meters
- Historical movement analytics (today, current week, current month, all-time history): total distance, average speed, time spent moving or stationary
- **Broadcast** or **private** text messaging between the server/administrator and vehicles
- Server CPU usage logging every 2 minutes
- Multiple interfaces: CLI client, GUI client, and GUI administrator console

---

## Architecture

The system follows a **client/server** architecture based on TCP, with a central server that handles each connection through an independent asynchronous task (`tokio::spawn`).

**Main components:**

- **Server** — listens on `127.0.0.1:8080`, manages shared state (`Arc<RwLock<ServerState>>`) and database access (`Arc<Mutex<Connection>>`), both encapsulated in `AppState`
- **CLI Client** — a single thread multiplexes keyboard input, the GPS simulator, and network communication through `tokio::select!`
- **GUI Client** (`egui`/`eframe`) — uses a dedicated background Tokio runtime and communicates with the UI through non-blocking `mpsc` channels (`try_recv()`)
- **Admin GUI** — follows the same architecture as the GUI client, with direct read-only database access for fleet statistics and analytics

