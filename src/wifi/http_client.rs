use crate::error::Result;

use embedded_svc::http::client::Client;
use embedded_svc::http::{Headers, Status};
use embedded_svc::io::Read;
use esp_idf_svc::http::client::EspHttpConnection;
use flate2::read::GzDecoder;
use std::io::Read as _;

pub trait HttpClient {
    fn get(&mut self, uri: &str) -> Result<Option<String>>;
}

pub struct Esp32HttpClient {
    client: Client<EspHttpConnection>,
}

impl Esp32HttpClient {
    pub fn new() -> Result<Self> {
        let conn = EspHttpConnection::new(&esp_idf_svc::http::client::Configuration {
            use_global_ca_store: true,
            crt_bundle_attach: Some(esp_idf_sys::esp_crt_bundle_attach),
            ..Default::default()
        })?;
        let client = Client::wrap(conn);
        Ok(Esp32HttpClient { client })
    }
}

impl HttpClient for Esp32HttpClient {
    fn get(&mut self, url: &str) -> Result<Option<String>> {
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
                    return Ok(Some(result));
                } else {
                    let result = String::from(std::str::from_utf8(&result)?);
                    return Ok(Some(result));
                }
            }
            _ => {
                return Ok(None);
            }
        }
    }
}
