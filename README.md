# Delgaz Watcher

A small Rust (for practice) program that checks Delgaz API for outages and calculates proximity to a predefined location to determine how likely is to be affected. Uses MQTT for communication for an easy integration with Home Assistant

## Tech Stack / Dependencies

- Rust (learning)
  - tokio - for async
  - reqwest - for the actual request to delgaz outages endpoint ( https://om.eonsn.ro/api/outages )
  - serde / serde_json - for Serialization/Deserialization
  - mqtt5 - for communication
  - chrono - for parsing DateTime
- MQTT

### Build & Run

```bash
cargo build --release
```

```bash
cd target/release
LATITUDE=47.639379270884255 LONGITUDE=26.240727718597064 delgaz-watcher
```

```Dockerfile
services:
  delgaz-watcher:
    image: ghcr.io/robiXxu/delgaz-watcher:latest
    container_name: delgaz-watcher
    restart: unless-stopped

    environment:
      LATITUDE: "47.639379270884255"
      LONGITUDE: "26.240727718597064"
      SLEEP_DURATION: "60"

      MQTT_CLIENT_ID: "delgaz-watcher"
      MQTT_URL: "mqtt://10.10.10.10:1883"
      MQTT_TOPIC: "homeassistant/sensor/delgaz_watcher/state"
```

If there's no mqtt service running they can be created as part of the same `docker-compose`

```
services:
  mqtt:
    image: eclipse-mosquitto:2
    container_name: mqtt
    restart: unless-stopped
    ports:
      - "1883:1883"
    volumes:
      - mosquitto_data:/mosquitto/data
      - mosquitto_log:/mosquitto/log
      - ./mosquitto.conf:/mosquitto/config/mosquitto.conf

  delgaz-watcher:
    image: ghcr.io/YOUR_USERNAME/delgaz-watcher:latest
    container_name: delgaz-watcher
    restart: unless-stopped
    depends_on:
      - mqtt
    environment:
      LATITUDE: "47.639379270884255"
      LONGITUDE: "26.240727718597064"
      SLEEP_DURATION: "60"

      MQTT_CLIENT_ID: "delgaz-watcher"
      MQTT_URL: "mqtt://mqtt:1883"
      MQTT_TOPIC: "homeassistant/sensor/delgaz_watcher/state"

volumes:
  mosquitto_data:
  mosquitto_log:
```

`mosquitto.conf`

```
listener 1883
allow_anonymous true
persistence true
persistence_location /mosquitto/data/
log_dest stdout
```
