use delgaz_watcher::{Coordinate, MqttConfig, MqttPublisher, Outage, Report};
use std::{env, ops::Mul};

const API_URL: &str = "https://om.eonsn.ro/api/outages";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>>{

    let max_radius: f64 = env::var("MAX_RADIUS")
        .unwrap_or_else(|_| "2000.0".to_string())
        .parse::<f64>()
        .unwrap_or(2000.0);

    let sleep_duration: u64 = env::var("SLEEP_DURATION")
        .unwrap_or_else(|_| "1".to_string())
        .parse::<u64>()
        .unwrap_or(1);

    let latitude: f64 = env::var("LATITUDE")
        .expect("LATITUDE environment variable not set")
        .parse()
        .expect("LATITUDE must be a valid float");

    let longitude: f64 = env::var("LONGITUDE")
        .expect("LONGITUDE environment variable not set")
        .parse()
        .expect("LONGITUDE must be a valid float");

    let watch_location: Coordinate = Coordinate { latitude, longitude };

    let client_id: String = match env::var("MQTT_CLIENT_ID") {
        Ok(v) => v,
        _ => String::from("delgaz-watcher")
    };

    let url: String = match env::var("MQTT_URL") {
        Ok(v) => v,
        _ => String::from("mqtt://10.10.10.10:1883")
    };

    let topic: String = match env::var("MQTT_TOPIC") {
        Ok(v) => v,
        _ => String::from("delgaz/outages")
    };


    let publisher = MqttPublisher::new(MqttConfig { client_id, url, topic });
    publisher.connect().await?;

    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(sleep_duration.mul(60)));

    loop {
        interval.tick().await;
        println!("Checking outages");

        let result = async {
            let outages: Vec<Outage> = reqwest::get(API_URL)
                .await?
                .json::<Vec<Outage>>()
                .await?;

            let outages_within_max_radius: Vec<Outage> = outages
                .into_iter()
                .filter(|outage| {
                    let outage_location = Coordinate {
                        latitude: outage.latitude,
                        longitude: outage.longitude,
                    };
                    watch_location.within_radius(&outage_location, max_radius)
                }).collect();

            let report = Report::new(&watch_location, outages_within_max_radius);

            println!("{:#?}", report);

            publisher.publish(&report).await?;
            Ok::<(), Box<dyn std::error::Error>>(())
        }.await;
        if let Err(err) = result {
            eprintln!("watcher error {err}");
        }

    }
}
