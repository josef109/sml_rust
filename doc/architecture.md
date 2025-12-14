# Architecture

## High-Level Design

```
Smart Meter
   │
   ▼
Serial / TCP Input
   │  (socat)
   ▼
SML Decoder (Rust)
   │
   ▼
Processing / Storage / Export
```

### Components

- **Input Layer**: Receives raw SML telegrams via serial or TCP socat (pty)
- **Parser**: Decodes SML frames into structured data
- **Processing Layer**: Validates and transforms meter values
- **Output Layer**: Exports data (e.g. logging, RRD, network)

### Design Goals

- Deterministic behavior
- Low memory footprint
- Embedded compatibility
- Clear separation of concerns
---
```mermaid
sequenceDiagram
    participant Meter as Smart Meter
    participant Reader as SML Reader (Rust)
    participant MQTT as MQTT Broker
    participant RRD as RRD Database
    participant State as Shared State
    participant Web as Web Client

    Meter->>Reader: Send SML Message (Binary)
    Reader->>Reader: Parse OBIS Codes
    
    par Update Outputs
        Reader->>MQTT: Publish Payload (JSON)
        Reader->>RRD: Update Values (rrdtool update)
        Reader->>State: Update Mutex & Broadcast Event
    end

    Note over RRD: Graph Loop periodically<br/>reads this to create PNGs

    State->>Web: SSE Push (JSON: {time, value, total_energy})
 ```   
---

