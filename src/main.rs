mod config;
mod probe;
mod html;
mod server;

use std::collections::HashMap;
use std::io;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time;

use prometheus::{register_counter_vec, register_gauge_vec};

use config::load_config;
use probe::{discover_probes, Probe};
use server::TempData;

fn run_loop(probes: &[Probe], port: u16, interval: time::Duration, calibration_offsets: &HashMap<String, f32>) -> Result<(), Box<dyn std::error::Error>> {
    let current_temps: TempData = Arc::new(Mutex::new(HashMap::new()));

    // sets up a scope so that the lock is dropped once done
    {
        let mut temps = current_temps.lock().unwrap();
        for probe in probes {
            temps.insert(probe.name.clone(), None);
        }
    }

    let prom_temp_readings = register_gauge_vec!(
        "dash_temp_readings",
        "calibrated readings from the temperature probes",
        &["probe"]
    )?;

    let prom_temp_readings_raw = register_gauge_vec!(
        "dash_temp_readings_raw",
        "uncalibrated readings from the temperature probes",
        &["probe"]
    )?;

    let prom_temp_read_errors = register_counter_vec!(
        "dash_temp_read_errors_total",
        "total number of failed temperature reads",
        &["probe", "error_type"]
    )?;

    // start http server
    server::start(port, Arc::clone(&current_temps))?;

    // probe loop
    loop {
        for p in probes {
            match p.read_temperature() {
                Ok(raw_temp) => {
                    let offset = calibration_offsets.get(&p.id).copied().unwrap_or(0.0);
                    let temp = raw_temp + offset;

                    prom_temp_readings_raw.with_label_values(&[&p.name]).set(raw_temp.into());
                    prom_temp_readings.with_label_values(&[&p.name]).set(temp.into());

                    let mut temps = current_temps.lock().unwrap();
                    temps.insert(p.name.clone(), Some(temp));

                    println!("probe: {}, temperature: {:.2}°c", p.name, temp);
                }
                Err(e) => {
                    let error_type = match e.kind() {
                        io::ErrorKind::NotFound => "not_found",
                        io::ErrorKind::PermissionDenied => "permission_denied",
                        io::ErrorKind::InvalidData => "invalid_data",
                        _ => "other",
                    };
                    prom_temp_read_errors
                        .with_label_values(&[&p.name, error_type])
                        .inc();

                    let mut temps = current_temps.lock().unwrap();
                    temps.insert(p.name.clone(), None);

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
                if let Err(e) = run_loop(&probes, config.settings.metrics_port, probe_interval, &config.calibration_offsets) {
                    eprintln!("error on loop initialisation: {e}");
                };
            }
        }
        Err(e) => {
            eprintln!("error discovering probes: {}", e);
        }
    }
}
