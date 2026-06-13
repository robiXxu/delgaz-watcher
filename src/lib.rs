use core::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use mqtt5::{MqttClient, MqttError};

const EARTH_RADIUS_M: f64 = 6_371_000.0; // Earth radius in meters

#[derive(Debug)]
pub enum Proximity {
    Critical(f64), // very likely to affect the user
    Near(f64),     // could affect the area
    Area(f64),     // useful awareness of outages in the area
    Far(f64),      // ignorable, but could be interesting to know about
}
impl Proximity {
    pub fn distance_m(&self) -> f64 {
        match self {
            Proximity::Critical(d)
            | Proximity::Near(d)
            | Proximity::Area(d)
            | Proximity::Far(d) => *d,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Proximity::Critical(_) => "critical",
            Proximity::Near(_) => "near",
            Proximity::Area(_) => "area",
            Proximity::Far(_) => "far",
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum OutageType {
    #[serde(rename = "LA")]
    Unplanned,

    #[serde(rename = "LUC")]
    Planned,

    #[serde(rename = "INC")]
    Reported,
}

impl OutageType {
    pub fn label(&self) -> &'static str {
        match self {
            OutageType::Unplanned => "unplanned",
            OutageType::Planned => "planned",
            OutageType::Reported => "reported",
        }
    }
}

#[derive(Debug)]
pub struct Coordinate {
    pub latitude: f64,
    pub longitude: f64,
}
impl Coordinate {
    pub fn proximity_to(&self, other: &Coordinate) -> Proximity {
        let distance = self.distance_to_m(other);
        match distance {
            d if d <= 250.0 => Proximity::Critical(d),
            d if d <= 750.0 => Proximity::Near(d),
            d if d <= 1500.0 => Proximity::Area(d),
            _ => Proximity::Far(distance),
        }
    }

    pub fn distance_to_m(&self, other: &Coordinate) -> f64 {
        let lat1 = self.latitude.to_radians();
        let lng1 = self.longitude.to_radians();

        let lat2 = other.latitude.to_radians();
        let lng2 = other.longitude.to_radians();

        let delta_lat = lat2 - lat1;
        let delta_lng = lng2 - lng1;

        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1.cos() * lat2.cos() * (delta_lng / 2.0).sin().powi(2);

        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

        EARTH_RADIUS_M * c
    }

    pub fn within_radius(&self, other: &Coordinate, radius_m: f64) -> bool {
        self.distance_to_m(other) <= radius_m
    }
}

#[derive(Debug, Deserialize)]
pub struct Outage {
    pub county: String,
    pub locality: String,
    pub street: String,
    pub latitude: f64,
    pub longitude: f64,

    #[serde(rename = "endDate")]
    pub end_date: DateTime<Utc>,

    #[serde(rename = "type")]
    pub outage_type: OutageType,
}

#[derive(Debug, Serialize)]
pub struct OutageReport {
    pub country: String,
    pub locality: String,
    pub street: String,
    pub latitude: f64,
    pub longitude: f64,
    pub end_date: DateTime<Utc>,
    pub outage_type: &'static str,
    pub proximity_classification: &'static str,
    pub proximity_distance: f64,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub total_outages: u8,
    pub nearest_distance_m: Option<f64>,
    pub nearest_proximity: Option<&'static str>,
    pub nearest_street: Option<String>,
    pub nearest: Option<OutageReport>,
    pub outages: Vec<OutageReport>,
    pub generated_at: DateTime<Utc>,
}

impl Report {
    pub fn new(coordinate: &Coordinate, outages: Vec<Outage>) -> Self {
        let total_outages = outages.len() as u8;
        let mut nearest_distance_m: Option<f64> = None;
        let mut nearest_proximity: Option<&'static str> = None;
        let mut nearest_street: Option<String> = None;
        let mut nearest: Option<OutageReport> = None;

        let outage_reports: Vec<OutageReport> = outages
            .into_iter()
            .map(|o| {
                let proximity = coordinate.proximity_to(&Coordinate {
                    latitude: o.latitude,
                    longitude: o.longitude,
                });
                if nearest_distance_m.is_none()
                    || proximity.distance_m() < nearest_distance_m.unwrap()
                {
                    nearest_distance_m = Some(proximity.distance_m());
                    nearest_proximity = Some(proximity.label());
                    nearest_street = Some(o.street.clone());

                    nearest = Some(OutageReport {
                        country: o.county.clone(),
                        locality: o.locality.clone(),
                        street: o.street.clone(),
                        latitude: o.latitude,
                        longitude: o.longitude,
                        end_date: o.end_date,
                        outage_type: o.outage_type.label(),
                        proximity_classification: proximity.label(),
                        proximity_distance: proximity.distance_m(),
                    });
                }
                OutageReport {
                    country: o.county.clone(),
                    locality: o.locality.clone(),
                    street: o.street.clone(),
                    latitude: o.latitude,
                    longitude: o.longitude,
                    end_date: o.end_date,
                    outage_type: o.outage_type.label(),
                    proximity_classification: proximity.label(),
                    proximity_distance: proximity.distance_m(),
                }
            })
            .collect();

        Report {
            total_outages,
            nearest_distance_m,
            nearest_proximity,
            nearest_street,
            nearest,
            outages: outage_reports,
            generated_at: Utc::now(),
        }
    }
}

#[derive(Debug, Serialize)]
struct HaDevice {
    identifiers: Vec<&'static str>,
    name: &'static str,
    manufacturer: &'static str,
    model: &'static str,
}
impl HaDevice {
    pub fn new() -> Self {
        HaDevice {
            identifiers: vec!["delgaz_watcher"],
            name: "Delgaz Watcher",
            manufacturer: "robiXxu (Robert Schiriac)",
            model: "Delgaz Outage Monitor"
        }
    }
    
}

#[derive(Debug, Serialize)]
enum HaComponentType {
    #[serde(rename = "binary_sensor")]
    BinarySensor,
    #[serde(rename = "fan")]
    Fan,
    #[serde(rename = "light")]
    Light,
    #[serde(rename = "sensor")]
    Sensor,
    #[serde(rename = "switch")]
    Switch,
}
impl fmt::Display for HaComponentType{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HaComponentType::BinarySensor => write!(f, "binary_sensor"),
            HaComponentType::Sensor => write!(f, "sensor"),
            HaComponentType::Fan => write!(f, "fan"),
            HaComponentType::Light => write!(f, "light"),
            HaComponentType::Switch => write!(f, "switch"),
        }
    }
}

#[derive(Debug, Serialize)]
struct HaDiscoveryConfig {
    name: String,
    object_id: String,
    unique_id: String,
    state_topic: String,
    value_template: String,
    component: HaComponentType,

    #[serde(skip_serializing_if = "Option::is_none")]
    icon: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    unit_of_measurement: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    state_class: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    device_class: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    payload_on: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    payload_off: Option<String>,

    device: HaDevice,
}
impl HaDiscoveryConfig {
    fn new(name: &str, object_id: &str, component: HaComponentType, value_template: &str, icon: &str, state_topic: &str) -> HaDiscoveryConfig {
        HaDiscoveryConfig {
            name: name.to_string(),
            object_id: object_id.to_lowercase(),
            unique_id: object_id.to_lowercase(),
            component,
            state_topic: state_topic.to_string(),
            value_template: value_template.to_string(),
            icon: Some(icon.to_string()),
            unit_of_measurement: None,
            state_class: None,
            device_class: None,
            payload_on: None,
            payload_off: None,
            device: HaDevice::new()
        }
    }

    pub fn discovery_topic(&self) -> String {  
        let topic = format!("homeassistant/{}/{}/config", &self.component, &self.object_id);
        println!("TOPIC: {}", topic);
        topic
    }
}

#[derive(Debug)]
pub struct MqttConfig {
    pub client_id: String,
    pub url: String,
    pub topic: String,
}

pub struct MqttPublisher {
    pub client: MqttClient,
    pub config: MqttConfig,
}
impl MqttPublisher {
    pub fn new(config: MqttConfig) -> Self {
        let client = MqttClient::new(&config.client_id);
        Self { client, config }
    }

    pub async fn connect(&self) -> Result<(), MqttError> {
        self.client.connect(&self.config.url).await
    }


    pub async fn publish(&self, report: &Report) -> Result<(), Box<dyn std::error::Error>> {
        if self.client.is_connected().await {
            let payload: Option<String> = match serde_json::to_string(report) {
                Ok(payload) => Some(payload),
                _ => None,
            };
            if !payload.is_none() {
                let state_topic = format!("{}/state", &self.config.topic);
                let payload = payload.unwrap();

                let publish_options = mqtt5::PublishOptions {
                    qos: mqtt5::QoS::AtLeastOnce,
                    retain: true,
                    properties: mqtt5::PublishProperties {
                        payload_format_indicator: None,
                        message_expiry_interval: None,
                        topic_alias: None,
                        response_topic: None,
                        correlation_data: None,
                        user_properties: Vec::new(),
                        subscription_identifiers: Vec::new(),
                        content_type: None
                    },
                    skip_codec: true
                };

                self.client.publish_with_options(&state_topic, payload.clone(), publish_options.clone()).await?;

                let entity = HaDiscoveryConfig{
                    state_class: Some("measurement".to_string()),
                    ..HaDiscoveryConfig::new(
                        "Delgaz Total Outages",
                        "delgaz_total_outages",
                        HaComponentType::Sensor,
                        "{{ value_json.total_outages }}",
                        "mdi:power-plug-off-outline",
                        &state_topic,
                    )
                };

                self.client.publish_with_options(
                    &entity.discovery_topic(),
                    serde_json::to_string(&entity).unwrap(),
                    publish_options.clone()
                ).await?;

                let entity = HaDiscoveryConfig{
                    unit_of_measurement: Some("m".to_string()),
                    state_class: Some("measurement".to_string()),
                    device_class: Some("distance".to_string()),
                    ..HaDiscoveryConfig::new(
                        "Delgaz Nearest Distance",
                        "delgaz_nearest_distance",
                        HaComponentType::Sensor,
                        "{{ value_json.nearest_distance_m }}",
                        "mdi:map-marker-distance",
                        &state_topic,
                    )
                };

                self.client.publish_with_options(
                    &entity.discovery_topic(),
                    serde_json::to_string(&entity).unwrap(),
                    publish_options.clone()
                ).await?;


                let entity = HaDiscoveryConfig{
                    ..HaDiscoveryConfig::new(
                        "Delgaz Outage",
                        "delgaz_outage",
                        HaComponentType::Sensor,
                        "{{ 'Yes' if (value_json.total_outages | int(0)) > 0 else 'No' }}",
                        "mdi:alert-circle",
                        &state_topic,
                    )
                };


                self.client.publish_with_options(
                    &entity.discovery_topic(),
                    serde_json::to_string(&entity).unwrap(),
                    publish_options.clone()
                ).await?;

                let entity = HaDiscoveryConfig{
                    unit_of_measurement: Some("m".to_string()),
                    state_class: Some("measurement".to_string()),
                    device_class: Some("distance".to_string()),
                    ..HaDiscoveryConfig::new(
                        "Delgaz Outage Proximity",
                        "delgaz_outage_proximity",
                        HaComponentType::Sensor,
                        "{{ value_json.nearest_proximity }}",
                        "mdi:vector-link",
                        &state_topic,
                    )
                };

                self.client.publish_with_options(
                    &entity.discovery_topic(),
                    serde_json::to_string(&entity).unwrap(),
                    publish_options.clone()
                ).await?;

                let entity = HaDiscoveryConfig{
                    state_class: Some("measurement".to_string()),
                    device_class: Some("distance".to_string()),
                    ..HaDiscoveryConfig::new(
                        "Delgaz Outage Street",
                        "delgaz_outage_street",
                        HaComponentType::Sensor,
                        "{{ value_json.nearest_street }}",
                        "mdi:road-variant",
                        &state_topic,
                    )
                };

                self.client.publish_with_options(
                    &entity.discovery_topic(),
                    serde_json::to_string(&entity).unwrap(),
                    publish_options.clone()
                ).await?;

            }
        }

        Ok(())
    }
}
