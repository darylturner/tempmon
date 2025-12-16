use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::Path;
use std::thread::sleep;
use std::time;

use prometheus_exporter::{
    self,
    prometheus::{register_counter_vec, register_gauge_vec},
};
use serde::Deserialize;

const W1_DEVICES_PATH: &str = "/sys/bus/w1/devices";
const CONFIG_PATH: &str = "/etc/tempmon/config.toml";

#[derive(Debug, Deserialize)]
struct Config {
    settings: Settings,
    probe_labels: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct Settings {
    metrics_port: u16,
    probe_interval: u64,
    probe_resolution: u8,
}

struct Probe {
    _id: String,
    name: String,
    path: String,
}

impl Probe {
    fn set_resolution(&self, bits: u8) -> io::Result<()> {
        let resolution_path = self.path.replace("/w1_slave", "/resolution");
        fs::write(resolution_path, bits.to_string())
    }

    fn read_temperature(&self) -> io::Result<f32> {
        let data = fs::read_to_string(&self.path)?;

        // the file format looks like:
        // 6d 01 55 05 7f a5 a5 66 3e : crc=3e YES
        // 6d 01 55 05 7f a5 a5 66 3e t=22812

        // check the crc
        if !data.contains("YES") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "crc check failed",
            ));
        }

        // find and read the temperature data
        if let Some(temp_pos) = data.find("t=") {
            let temp_str = &data[temp_pos + 2..].trim();
            if let Ok(temp_raw) = temp_str.parse::<i32>() {
                return Ok(temp_raw as f32 / 1000.0);
            }
        }

        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "failed to parse temperature",
        ))
    }
}

fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(CONFIG_PATH)?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}

fn discover_probes(labels: &HashMap<String, String>) -> io::Result<Vec<Probe>> {
    let mut probes = Vec::new();

    if !Path::new(W1_DEVICES_PATH).exists() {
        eprintln!(
            "warning: {} not found. make sure w1-gpio is enabled.",
            W1_DEVICES_PATH
        );
        return Ok(probes);
    }

    for entry in fs::read_dir(W1_DEVICES_PATH)? {
        let id = entry?.file_name().to_string_lossy().to_string();

        if id.starts_with("28-") {
            let name = labels.get(&id).cloned().unwrap_or_else(|| id.clone());
            probes.push(Probe {
                _id: id.clone(),
                name,
                path: format!("{}/{}/w1_slave", W1_DEVICES_PATH, id),
            });
        }
    }

    Ok(probes)
}

fn run_loop(probes: &Vec<Probe>, port: u16, interval: time::Duration) {
    let binding = format!("127.0.0.1:{port}").parse().expect("parse binding");
    if let Err(e) = prometheus_exporter::start(binding) {
        eprintln!("error starting prometheus exporter: {e}");
    }

    let temp_readings = register_gauge_vec!(
        "dash_temp_readings",
        "readings from the temperature probes",
        &["probe"]
    )
    .unwrap();

    let temp_read_errors = register_counter_vec!(
        "dash_temp_read_errors_total",
        "total number of failed temperature reads",
        &["probe", "error_type"]
    )
    .unwrap();

    loop {
        for p in probes {
            match p.read_temperature() {
                Ok(temp) => {
                    temp_readings.with_label_values(&[&p.name]).set(temp.into());
                    println!("probe: {}, temperature: {:.2}°c", p.name, temp);
                }
                Err(e) => {
                    let error_type = match e.kind() {
                        io::ErrorKind::NotFound => "not_found",
                        io::ErrorKind::PermissionDenied => "permission_denied",
                        io::ErrorKind::InvalidData => "invalid_data",
                        _ => "other",
                    };
                    temp_read_errors
                        .with_label_values(&[&p.name, error_type])
                        .inc();
                    println!("probe: {}, error reading temperature: {}", p.name, e);
                }
            }
        }
        sleep(interval);
    }
}

fn main() {
    let config = match load_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("error loading config from {}: {}", CONFIG_PATH, e);
            std::process::exit(1);
        }
    };

    let probe_interval = time::Duration::from_secs(config.settings.probe_interval);

    println!("discovering ds18b20 temperature probes...\n");
    match discover_probes(&config.probe_labels) {
        Ok(probes) => {
            println!("found {} probe(s):\n", probes.len());
            if !probes.is_empty() {
                // set resolution for all probes
                for probe in &probes {
                    if let Err(e) = probe.set_resolution(config.settings.probe_resolution) {
                        eprintln!("warning: failed to set resolution for {}: {}", probe.name, e);
                    }
                }
                run_loop(&probes, config.settings.metrics_port, probe_interval);
            }
        }
        Err(e) => {
            eprintln!("error discovering probes: {}", e);
        }
    }
}
