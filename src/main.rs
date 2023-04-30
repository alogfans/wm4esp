mod app;
mod config;
mod display;
mod error;
mod network;
mod peripheral;

use esp_idf_sys::{self as _};
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

use esp_idf_hal::peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;

use config::CONFIG;
use network::wifi::WifiDevice;
use peripheral::dht20::DHT20;
use peripheral::ssd1683::{SSD1683Gpio, SSD1683};
use std::error::Error;

fn main() -> std::result::Result<(), Box<dyn Error>> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();
    println!("Hello world from ESP 32 device");

    let conf = CONFIG;
    let peripherals = peripherals::Peripherals::take().unwrap();
    let eventloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take().unwrap();

    let mut wifi = WifiDevice::new(peripherals.modem, eventloop, Some(nvs))?;
    wifi.connect(conf.wifi_ssid, conf.wifi_psk)?;

    let gpio = SSD1683Gpio {
        gpio5: peripherals.pins.gpio5,
        gpio12: peripherals.pins.gpio12,
        gpio13: peripherals.pins.gpio13,
        gpio14: peripherals.pins.gpio14,
        gpio18: peripherals.pins.gpio18,
        gpio23: peripherals.pins.gpio23,
    };

    let ssd1683 = SSD1683::new(gpio, peripherals.spi2)?;

    let dht20 = DHT20::new(
        peripherals.i2c1,
        peripherals.pins.gpio21,
        peripherals.pins.gpio22,
    )?;

    app::app_main(ssd1683, dht20, wifi, conf)?;
    Ok(())
}
