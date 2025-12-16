mod config;
mod probe;

use std::io;
use std::thread::sleep;
use std::time;

use prometheus_exporter::{
    self,
    prometheus::{register_counter_vec, register_gauge_vec},
};

use config::load_config;
use probe::{discover_probes, Probe};

fn run_loop(probes: &[Probe], port: u16, interval: time::Duration) -> Result<(), Box<dyn std::error::Error>> {
    let binding = format!("0.0.0.0:{port}").parse().expect("parse binding");
    prometheus_exporter::start(binding)?;

    let temp_readings = register_gauge_vec!(
        "dash_temp_readings",
        "readings from the temperature probes",
        &["probe"]
    )?;

    let temp_read_errors = register_counter_vec!(
        "dash_temp_read_errors_total",
        "total number of failed temperature reads",
        &["probe", "error_type"]
    )?;

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
            eprintln!("error loading config: {}", e);
            std::process::exit(1);
        }
    };

    let probe_interval = time::Duration::from_secs(config.settings.probe_interval);

    println!("discovering ds18b20 temperature probes...");
    match discover_probes(&config.probe_labels) {
        Ok(probes) => {
            println!("found {} probe(s):\n", probes.len());
            if !probes.is_empty() {
                for probe in &probes {
                    if let Err(e) = probe.set_resolution(config.settings.probe_resolution) {
                        eprintln!("warning: failed to set resolution for {}: {}", probe.name, e);
                    }
                }
                if let Err(e) = run_loop(&probes, config.settings.metrics_port, probe_interval) {
                    eprintln!("error on loop initialisation: {e}");
                };
            }
        }
        Err(e) => {
            eprintln!("error discovering probes: {}", e);
        }
    }
}
