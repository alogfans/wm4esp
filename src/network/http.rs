use crate::error::Result;

use embedded_svc::http::client::Client;
use embedded_svc::http::{Headers, Status};
use embedded_svc::io::Read;
use embedded_svc::{http::Method, io::Write};
use esp_idf_svc::http::client::EspHttpConnection;
use esp_idf_svc::http::server::EspHttpServer;
use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use std::io::Read as _;
use std::sync::{Arc, Mutex};
use time::OffsetDateTime;

pub struct HttpClient {
    client: Client<EspHttpConnection>,
}

impl HttpClient {
    pub fn new() -> Result<Self> {
        let conn = EspHttpConnection::new(&esp_idf_svc::http::client::Configuration {
            use_global_ca_store: true,
            crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
            ..Default::default()
        })?;
        let client = Client::wrap(conn);
        Ok(HttpClient { client })
    }

    pub fn get(&mut self, url: &str) -> Result<String> {
        let request = self.client.get(url.as_ref())?;
        let response = request.submit()?;
        let status = response.status();
        let gzip = response
            .header("Content-Encoding")
            .unwrap_or_default()
            .contains(&"gzip");
        match status {
            200 => {
                let mut buf = [0_u8; 1024];
                let mut reader = response;
                let mut result = Vec::new();
                loop {
                    if let Ok(size) = Read::read(&mut reader, &mut buf) {
                        if size == 0 {
                            break;
                        }
                        result.extend_from_slice(&buf[..size]);
                    }
                }
                if gzip {
                    let mut d = GzDecoder::new(result.as_slice());
                    let mut result = String::new();
                    d.read_to_string(&mut result).unwrap();
                    return Ok(result);
                } else {
                    let result = String::from(std::str::from_utf8(&result)?);
                    return Ok(result);
                }
            }
            _ => {
                return Ok(String::from(""));
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct SensorRecord {
    time: String,
    temp: f32,
    humidity: f32,
}

impl SensorRecord {
    fn new(datetime: OffsetDateTime, sensor: (f32, f32)) -> Self {
        let time = format!("{:02}:{:02}", datetime.hour(), datetime.minute());
        SensorRecord {
            time,
            temp: sensor.0,
            humidity: sensor.1,
        }
    }
}

pub struct HttpServer {
    server: EspHttpServer,
    note_content: Arc<Mutex<String>>,
    refresh_flag: Arc<Mutex<bool>>,
    sensor_data: Arc<Mutex<Vec<SensorRecord>>>,
}

impl HttpServer {
    pub fn new() -> Result<Self> {
        let server = EspHttpServer::new(&esp_idf_svc::http::server::Configuration::default())?;
        let note_content = Arc::new(Mutex::new(String::from("")));
        let refresh_flag = Arc::new(Mutex::new(false));
        let sensor_data = Arc::new(Mutex::new(Vec::new()));
        Ok(HttpServer {
            server,
            note_content,
            refresh_flag,
            sensor_data,
        })
    }

    pub fn add_sensor_data(&mut self, datetime: OffsetDateTime, sensor: (f32, f32)) -> Result<()> {
        let mut sensor_data = self.sensor_data.lock().unwrap();
        let record = SensorRecord::new(datetime, sensor);
        if datetime.time().as_hms() == (0, 0, 0) {
            sensor_data.clear();
        }
        sensor_data.push(record);
        Ok(())
    }

    pub fn get_note_content(&mut self) -> Result<String> {
        let note_content = self.note_content.lock().unwrap();
        Ok(note_content.clone())
    }

    pub fn get_refresh_flag(&mut self) -> Result<bool> {
        let mut refresh_flag = self.refresh_flag.lock().unwrap();
        if *refresh_flag == true {
            *refresh_flag = false;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn add_handlers(&mut self) -> Result<()> {
        let note_content = Arc::clone(&self.note_content);
        self.server.fn_handler("/", Method::Get, move |request| {
            let template = include_str!("index.html");
            let note_content = note_content.lock().unwrap().clone();
            let html = template.replace("[[[PLACEHOLDER]]]", &note_content);
            let mut response = request.into_ok_response()?;
            response.write_all(html.as_bytes())?;
            Ok(())
        })?;

        let refresh_flag = Arc::clone(&self.refresh_flag);
        self.server
            .fn_handler("/refresh", Method::Get, move |request| {
                let mut refresh_flag = refresh_flag.lock().unwrap();
                *refresh_flag = true;

                let html = include_str!("completed.html");
                let mut response = request.into_ok_response()?;
                response.write_all(html.as_bytes())?;
                Ok(())
            })?;

        let sensor_data = Arc::clone(&self.sensor_data);
        self.server
            .fn_handler("/sensor", Method::Get, move |request| {
                let sensor_data = sensor_data.lock().unwrap();
                let json = serde_json::to_string(&*sensor_data).unwrap_or("".into());
                let mut response = request.into_ok_response()?;
                response.write_all(json.as_bytes())?;
                Ok(())
            })?;

        let note_content = Arc::clone(&self.note_content);
        self.server.fn_handler("/", Method::Post, move |request| {
            let mut buf = [0_u8; 1024];
            let mut reader = request;
            let mut result = Vec::new();
            loop {
                if let Ok(size) = reader.read(&mut buf) {
                    if size == 0 {
                        break;
                    }
                    result.extend_from_slice(&buf[..size]);
                }
            }

            let result = std::str::from_utf8(&result)?;
            let result = result.trim_start_matches("sticky=").to_string();
            let mut note_content = note_content.lock().unwrap();
            *note_content = result;

            let html = include_str!("completed.html");
            let mut response = reader.into_ok_response()?;
            response.write_all(html.as_bytes())?;
            Ok(())
        })?;

        Ok(())
    }
}
