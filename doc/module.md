|File|Responsibility|
|----|---------------|
|main.rs|Entry point. Initializes shared state, handles graceful, shutdown, and spawns the 3 main tasks (Serial, Graph, Web).|
|sml.rs|***The Producer.*** Reads serial stream, parses SML protocol (OBIS 1.8.0, 16.7.0), handles integration logic for feed-in energy, and updates RRD/MQTT.|
|rrd.rs|***Storage & Viz.*** Wraps librrd. Updates the database and runs the loop to generate PNG graphs (Hourly, Daily, Weekly).|
|web.rs|***The Frontend.*** An `axum` web server serving `index.html`, static images, and the SSE stream (`/events`) for live updates.|
|mqtt.rs|***Integration.*** Handles MQTT connection, reconnection logic, and Home Assistant auto-discovery payloads.|
|config.rs|***Settings.*** Defines the *Config* struct using `clap` for parsing arguments and environment variables.|
|model.rs|***Data Types.*** Defines *AppState* (shared memory), *SensorData* (internal logic), and `SseData` (JSON payload).