use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

use prometheus::{Encoder, TextEncoder};
use tiny_http::{Header, Response, Server};

use crate::html;

pub type TempData = Arc<Mutex<HashMap<String, Option<f32>>>>;

pub fn start(
    port: u16,
    current_temps: TempData,
    threads: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let server = Arc::new(
        Server::http(format!("0.0.0.0:{port}"))
            .map_err(|e| format!("failed to start http server: {}", e))?,
    );

    println!("http server listening on 0.0.0.0:{}", port);

    for _ in 0..threads {
        let server = server.clone();
        let current_temps = current_temps.clone();

        thread::spawn(move || {
            for request in server.incoming_requests() {
                match request.url() {
                    "/metrics" => {
                        let encoder = TextEncoder::new();
                        let metric_families = prometheus::gather();
                        let mut buffer = vec![];
                        encoder.encode(&metric_families, &mut buffer).unwrap();

                        let response = Response::from_data(buffer).with_header(
                            Header::from_bytes(
                                &b"Content-Type"[..],
                                &b"text/plain; version=0.0.4"[..],
                            )
                            .unwrap(),
                        );
                        let _ = request.respond(response);
                    }
                    "/" => {
                        let temps = current_temps.lock().unwrap();
                        let html = html::generate_temperature_page(&temps);
                        let response = Response::from_string(html).with_header(
                            Header::from_bytes(
                                &b"Content-Type"[..],
                                &b"text/html; charset=utf-8"[..],
                            )
                            .unwrap(),
                        );
                        let _ = request.respond(response);
                    }
                    "/health" => {
                        let response = Response::from_string("OK");
                        let _ = request.respond(response);
                    }
                    _ => {
                        let response = Response::from_string("404 Not Found").with_status_code(404);
                        let _ = request.respond(response);
                    }
                }
            }
        });
    }

    Ok(())
}
