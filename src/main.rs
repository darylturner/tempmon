mod config;
mod probe;

use std::collections::HashMap;
use std::io;
use std::sync::{Arc, Mutex};
use std::thread::{self, sleep};
use std::time;

use prometheus::{Encoder, TextEncoder, register_counter_vec, register_gauge_vec};
use tiny_http::{Response, Server};

use config::load_config;
use probe::{discover_probes, Probe};

type TempData = Arc<Mutex<HashMap<String, Option<f32>>>>;

fn run_loop(probes: &[Probe], port: u16, interval: time::Duration) -> Result<(), Box<dyn std::error::Error>> {
    let current_temps: TempData = Arc::new(Mutex::new(HashMap::new()));

    // sets up a scope so that the lock is dropped once done
    {
        let mut temps = current_temps.lock().unwrap();
        for probe in probes {
            temps.insert(probe.name.clone(), None);
        }
    }

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

    let server = Server::http(format!("0.0.0.0:{port}"))
        .map_err(|e| format!("failed to start http server: {}", e))?;
    println!("http server listening on 0.0.0.0:{}", port);

    let temps_for_server = Arc::clone(&current_temps);
    thread::spawn(move || {
        for request in server.incoming_requests() {
            let temps = temps_for_server.lock().unwrap();

            match request.url() {
                "/metrics" => {
                    let encoder = TextEncoder::new();
                    let metric_families = prometheus::gather();
                    let mut buffer = vec![];
                    encoder.encode(&metric_families, &mut buffer).unwrap();

                    let response = Response::from_data(buffer)
                        .with_header(
                            tiny_http::Header::from_bytes(&b"Content-Type"[..],
                            &b"text/plain; version=0.0.4"[..]).unwrap()
                        );
                    let _ = request.respond(response);
                }
                "/" => {
                    let html = generate_temperature_page(&temps);
                    let response = Response::from_string(html)
                        .with_header(
                            tiny_http::Header::from_bytes(&b"Content-Type"[..],
                            &b"text/html; charset=utf-8"[..]).unwrap()
                        );
                    let _ = request.respond(response);
                }
                "/health" => {
                    let response = Response::from_string("OK");
                    let _ = request.respond(response);
                }
                _ => {
                    let response = Response::from_string("404 Not Found")
                        .with_status_code(404);
                    let _ = request.respond(response);
                }
            }
        }
    });


    loop {
        for p in probes {
            match p.read_temperature() {
                Ok(temp) => {
                    temp_readings.with_label_values(&[&p.name]).set(temp.into());

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
                    temp_read_errors
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

fn generate_temperature_page(temps: &HashMap<String, Option<f32>>) -> String {
    let mut rows = String::new();
    let mut temp_vec: Vec<_> = temps.iter().collect();
    temp_vec.sort_by_key(|(name, _)| name.as_str());

    for (name, temp) in temp_vec {
        let temp_display = match temp {
            Some(t) => {
                let color = if *t < 20.0 {
                    "#3498db" // blue
                } else if *t < 25.0 {
                    "#2ecc71" // green
                } else if *t < 30.0 {
                    "#f39c12" // orange
                } else {
                    "#e74c3c" // red
                };
                format!(
                    "<span style='color: {}; font-size: 2em; font-weight: bold;'>{:.1}°C</span>",
                    color, t
                )
            }
            None => "<span style='color: #95a5a6; font-style: italic;'>Error</span>".to_string(),
        };

        rows.push_str(&format!(
            "<tr><td style='padding: 15px; border-bottom: 1px solid #ecf0f1;'>{}</td>\
             <td style='padding: 15px; border-bottom: 1px solid #ecf0f1; text-align: right;'>{}</td></tr>",
            name, temp_display
        ));
    }

    let now = time::SystemTime::now();
    let datetime = format!("{:?}", now);

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta http-equiv="refresh" content="15">
    <title>Temperature Monitor</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
            max-width: 800px;
            margin: 40px auto;
            padding: 20px;
            background: #f5f5f5;
        }}
        .container {{
            background: white;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            padding: 30px;
        }}
        h1 {{
            color: #2c3e50;
            margin-top: 0;
            border-bottom: 3px solid #3498db;
            padding-bottom: 10px;
        }}
        table {{
            width: 100%;
            border-collapse: collapse;
            margin-top: 20px;
        }}
        th {{
            text-align: left;
            padding: 15px;
            background: #34495e;
            color: white;
            font-weight: 600;
        }}
        .footer {{
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #ecf0f1;
            color: #7f8c8d;
            font-size: 0.9em;
        }}
        .footer a {{
            color: #3498db;
            text-decoration: none;
        }}
        .footer a:hover {{
            text-decoration: underline;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>🌡️ Temperature Monitor</h1>
        <table>
            <thead>
                <tr>
                    <th>Probe</th>
                    <th style="text-align: right;">Temperature</th>
                </tr>
            </thead>
            <tbody>
                {}
            </tbody>
        </table>
        <div class="footer">
            Last updated: {} UTC (auto-refresh every 15s)<br>
            <a href="/metrics">Prometheus Metrics</a> | <a href="/health">Health Check</a>
        </div>
    </div>
</body>
</html>"#,
        rows, datetime
    )
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
