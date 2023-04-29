use crate::error::Result;
use std::{thread::sleep, time::Duration};

use embedded_svc::wifi::{ClientConfiguration, Configuration, Wifi};
use esp_idf_hal::modem::Modem;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sntp::{EspSntp, SyncStatus};
use esp_idf_svc::wifi::EspWifi;

pub struct WifiDevice<'a> {
    device: EspWifi<'a>,
    ntp: EspSntp,
}

impl WifiDevice<'_> {
    pub fn new(
        modem: Modem,
        eventloop: EspSystemEventLoop,
        nvs: Option<EspDefaultNvsPartition>,
    ) -> Result<Self> {
        let device = EspWifi::new(modem, eventloop, nvs)?;
        let ntp = EspSntp::new_default()?;
        Ok(WifiDevice { device, ntp })
    }

    pub fn connect(&mut self, ssid: &str, password: &str) -> Result<()> {
        self.device
            .set_configuration(&Configuration::Client(ClientConfiguration {
                ssid: ssid.into(),
                password: password.into(),
                ..Default::default()
            }))?;
        self.device.start()?;
        self.device.connect()?;
        while !self.device.is_connected()? {
            sleep(Duration::from_millis(500));
        }
        println!("Wi-Fi connection established");

        for _ in 0..20 {
            if self.ntp.get_sync_status() == SyncStatus::Completed {
                println!("NTP Server started");
                break;
            } else {
                sleep(Duration::from_millis(500));
            }
        }

        Ok(())
    }

    pub fn ip_addr(&self) -> Result<String> {
        let result = self.device.sta_netif().get_ip_info()?.ip;
        Ok(result.to_string())
    }
}
