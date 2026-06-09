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
    pub nearest: Option<OutageReport>,
    pub outages: Vec<OutageReport>,
    pub generated_at: DateTime<Utc>,
}

impl Report {
    pub fn new(coordinate: &Coordinate, outages: Vec<Outage>) -> Self {
        let total_outages = outages.len() as u8;
        let mut nearest_distance_m: Option<f64> = None;
        let mut nearest_proximity: Option<&'static str> = None;
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
            nearest,
            outages: outage_reports,
            generated_at: Utc::now(),
        }
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
                self.client.publish(&self.config.topic, payload.unwrap()).await?;
            }
        }

        Ok(())
    }
}
