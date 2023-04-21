use crate::error::Result;
use embedded_svc::wifi::{ClientConfiguration, Configuration, Wifi};
use esp_idf_hal::modem::Modem;
use esp_idf_svc::{eventloop::EspSystemEventLoop, nvs::EspDefaultNvsPartition, wifi::EspWifi};
use std::{fmt::Display, thread::sleep, time::Duration};

pub trait WifiDevice {
    fn connect(&mut self, ssid: &str, password: &str) -> Result<()>;

    fn ip_addr(&self) -> Result<String>;
}

pub struct Esp32WifiDevice<'a> {
    device: EspWifi<'a>,
}

impl Esp32WifiDevice<'_> {
    pub fn new(
        modem: Modem,
        eventloop: EspSystemEventLoop,
        nvs: Option<EspDefaultNvsPartition>,
    ) -> Result<Self> {
        let device = EspWifi::new(modem, eventloop, nvs)?;
        Ok(Esp32WifiDevice { device })
    }
}

impl Display for Esp32WifiDevice<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let result = self.device.sta_netif().get_ip_info();
        if let Ok(result) = result {
            write!(f, "Esp32Wifi {:?}", result)?;
        } else {
            write!(f, "Esp32Wifi (unknown addr)")?;
        };
        Ok(())
    }
}

impl WifiDevice for Esp32WifiDevice<'_> {
    fn connect(&mut self, ssid: &str, password: &str) -> Result<()> {
        self.device
            .set_configuration(&Configuration::Client(ClientConfiguration {
                ssid: ssid.into(),
                password: password.into(),
                ..Default::default()
            }))?;
        self.device.start()?;
        self.device.connect()?;
        while !self.device.is_connected()? {
            sleep(Duration::from_millis(50));
        }
        Ok(())
    }

    fn ip_addr(&self) -> Result<String> {
        let result = self.device.sta_netif().get_ip_info()?.ip;
        Ok(result.to_string())
    }
}
