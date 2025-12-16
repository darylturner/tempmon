use std::env;
use std::fs;
use std::io;
use std::path::Path;
use std::thread::sleep;
use std::time;

use prometheus_exporter::{
    self,
    prometheus::{register_counter_vec, register_gauge_vec},
};

const W1_DEVICES_PATH: &str = "/sys/bus/w1/devices";

struct Probe {
    id: String,
    path: String,
}

impl Probe {
    fn friendly_name(&self) -> String {
        env::var(format!("PROBE_{}", self.id))
            .unwrap_or_else(|_| self.id.clone())
    }

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

fn discover_probes() -> io::Result<Vec<Probe>> {
    let mut probes = Vec::new();

    if !Path::new(W1_DEVICES_PATH).exists() {
        eprintln!(
            "warning: {} not found. make sure w1-gpio is enabled.",
            W1_DEVICES_PATH
        );
        return Ok(probes);
    }

    for entry in fs::read_dir(W1_DEVICES_PATH)? {
        let name = entry?.file_name().to_string_lossy().to_string();

        if name.starts_with("28-") {
            probes.push(Probe {
                id: name.clone(),
                path: format!("{}/{}/w1_slave", W1_DEVICES_PATH, name),
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
            let name = p.friendly_name();
            match p.read_temperature() {
                Ok(temp) => {
                    temp_readings.with_label_values(&[&name]).set(temp.into());
                    println!("probe: {}, temperature: {:.2}°c", name, temp);
                }
                Err(e) => {
                    let error_type = match e.kind() {
                        io::ErrorKind::NotFound => "not_found",
                        io::ErrorKind::PermissionDenied => "permission_denied",
                        io::ErrorKind::InvalidData => "invalid_data",
                        _ => "other",
                    };
                    temp_read_errors
                        .with_label_values(&[&name, error_type])
                        .inc();
                    println!("probe: {}, error reading temperature: {}", name, e);
                }
            }
        }
        sleep(interval);
    }
}

fn main() {
    let metrics_port: u16 = env::var("METRICS_PORT")
        .unwrap_or("9184".to_string())
        .parse()
        .unwrap();

    let probe_interval = time::Duration::from_secs(
        env::var("PROBE_INTERVAL")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30)
    );

    /* 
    DS18B20 Resolution Options
    The sensor supports 4 resolution settings:

    | Bits | Resolution | Conversion Time  |
    |------|------------|------------------|
    | 9    | 0.5°C      | ~93.75ms         |
    | 10   | 0.25°C     | ~187.5ms         | <-- our default
    | 11   | 0.125°C    | ~375ms           |
    | 12   | 0.0625°C   | ~750ms (default) | <-- hardware default
    */
    let probe_resolution: u8 = env::var("PROBE_RESOLUTION")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);

    println!("discovering ds18b20 temperature probes...\n");
    match discover_probes() {
        Ok(probes) => {
            println!("found {} probes(s):\n", probes.len());
            if !probes.is_empty() {
                // set resolution for all probes
                for probe in &probes {
                    if let Err(e) = probe.set_resolution(probe_resolution) {
                        eprintln!("warning: failed to set resolution for {}: {}", probe.friendly_name(), e);
                    }
                }
                run_loop(&probes, metrics_port, probe_interval);
            }
        }
        Err(e) => {
            eprintln!("error discovering probes: {}", e);
        }
    }
}
