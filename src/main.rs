use esp_idf_sys::{self as _};
// If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

use esp_idf_hal::peripherals;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use rand::random;
use std::error::Error;
use std::ops::Sub;
use std::{thread::sleep, time::Duration};

mod display;
mod error;
mod resource;
mod sensor;
mod weather;
mod wifi;

use display::ssd1683::{SSD1683Gpio, SSD1683};
use display::{Color, Device, Screen};
use error::WmError;
use time_macros::offset;
use weather::WeatherInfo;
use wifi::{Esp32WifiDevice, WifiDevice};

use crate::resource::quotes::QUOTE_LIST;
use crate::sensor::dht20::DHT20;

#[toml_cfg::toml_config]
pub struct Config {
    #[default("")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_psk: &'static str,
    #[default("")]
    qweather_key: &'static str,
    #[default("")]
    location: &'static str,
}

fn main() -> std::result::Result<(), Box<dyn Error>> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();
    println!("Hello world from ESP 32 device");

    let app_config = CONFIG;

    const SCREEN_HEIGHT: usize = 300;
    const SCREEN_WIDTH: usize = 400;

    let peripherals = peripherals::Peripherals::take().ok_or(WmError::InvalidArgument)?;
    let eventloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take().unwrap();

    let mut wifi_device = Esp32WifiDevice::new(peripherals.modem, eventloop, Some(nvs))?;
    wifi_device.connect(app_config.wifi_ssid, app_config.wifi_psk)?;
    crate::wifi::sntp::start_sntp_service()?;

    let mut temp_sensor = DHT20::new(
        peripherals.pins.gpio21,
        peripherals.pins.gpio22,
        peripherals.i2c1,
    )?;

    let gpio = SSD1683Gpio {
        gpio5: peripherals.pins.gpio5,
        gpio12: peripherals.pins.gpio12,
        gpio13: peripherals.pins.gpio13,
        gpio14: peripherals.pins.gpio14,
        gpio18: peripherals.pins.gpio18,
        gpio23: peripherals.pins.gpio23,
    };

    let mut screen = Screen::new(SCREEN_HEIGHT, SCREEN_WIDTH, Color::Black);
    let mut device = SSD1683::new(gpio, peripherals.spi2)?;

    let mut weather = WeatherInfo::new(app_config.location, app_config.qweather_key);
    let mut cycle = 0;
    let mut refresh_time = time::OffsetDateTime::UNIX_EPOCH;
    loop {
        let (temperature, humidity) = temp_sensor.read()?;

        let now = time::OffsetDateTime::now_utc().to_offset(offset!(+8));
        let refresh_period = if now.hour() <= 6 { 3600 } else { 300 };
        if now.sub(refresh_time).whole_seconds() < refresh_period {
            sleep(Duration::from_secs(30));
            continue;
        } else {
            refresh_time = now;
        }

        weather.try_update();
        screen.clear(Color::White);

        let line = format!(
            "{:02}:{:02}:{:02}",
            weather.last_update().hour(),
            weather.last_update().minute(),
            weather.last_update().second()
        );

        let x_pos = SCREEN_WIDTH - 8 * (1 + line.len());
        screen.text(x_pos, 0, 16, &line, Color::Black)?;

        let text = format!("{:02}", now.hour());
        screen.text(16, 16, 32, &text, Color::InvRed)?;

        let text = format!(
            "   {} {:04}-{:02}-{:02}",
            now.weekday(),
            now.year(),
            now.month() as i32,
            now.day()
        );
        screen.text(16, 16, 32, &text, Color::Red)?;

        // let line = format!(
        //     "{} {}°C|{:.1}°C {:.1}%",
        //     weather.now.text, weather.now.temperature, temperature, humidity
        // );
        // screen.text(16, 48, 32, &line, Color::Black)?;

        let line = format!(
            "{} {}|{:.0}",
            weather.now.text, weather.now.temperature, temperature
        );
        let x_cursor = screen.text(16, 48, 32, &line, Color::Black)?;
        let line = format!(".{:.0}°C ", (temperature - temperature.floor()) * 10.0);
        let x_cursor = screen.text(x_cursor, 48 + 12, 16, &line, Color::Black)?;
        let line = format!("{:.0}", humidity);
        let x_cursor = screen.text(x_cursor, 48, 32, &line, Color::Black)?;
        let line = format!(".{:.0}%", (humidity - humidity.floor()) * 10.0);
        screen.text(x_cursor, 48 + 12, 16, &line, Color::Black)?;

        // Left
        let color = if weather.now.precipitation >= 50.0 {
            Color::InvRed
        } else {
            Color::Black
        };

        let line = format!("Precipitation: {}%", weather.now.precipitation);
        screen.text(16, 80, 16, &line, color)?;

        let color = match weather.now.wind_scale {
            0..=5 => Color::Black,
            _ => Color::InvRed,
        };
        let line = format!(
            "Wind: {} {} ({} km/h)",
            weather.now.wind_dir, weather.now.wind_scale, weather.now.wind_speed
        );
        screen.text(16, 96, 16, &line, color)?;

        let color = match weather.now.aqi {
            0..=100 => Color::Black,
            101..=200 => Color::Red,
            _ => Color::InvRed,
        };

        let line = format!("AQI: {} ({})", weather.now.aqi, weather.now.aqi_category);
        screen.text(16, 112, 16, &line, color)?;

        // Right part
        let line = if cycle % 2 == 0 {
            format!(
                "Feels-like: {}°C\nHumidity: {}%\nPressure: {} kPa",
                weather.now.feels_like, weather.now.humidity, weather.now.pressure
            )
        } else {
            format!(
                "Primary: {}\nPM10: {} ug/m3\nPM2.5: {} ug/m3",
                weather.now.aqi_primary, weather.now.aqi_pm10, weather.now.aqi_pm2p5
            )
        };
        screen.text(SCREEN_WIDTH / 2, 80, 16, &line, Color::Black)?;

        if !weather.forecast.is_empty() && cycle % 3 == 0 {
            screen.rectangle(0, 136, SCREEN_WIDTH, 16, Color::Red)?;
            let line = "WEATHER FORECAST";
            let x_pos = (SCREEN_WIDTH - line.len() * 8) / 2; // center display
            screen.text(x_pos, 136, 16, line, Color::White)?;

            // y == 160
            // XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
            //  11-15  Cloudy        -1~ 12  74   0.0  NW 1-2
            let line = format!("Day    Brief         Temp/°C HR/% Pr/% Wind");
            screen.text(16, 160, 16, &line, Color::Red)?;
            for (idx, entry) in weather.forecast.iter().enumerate() {
                // XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
                // 11-15  Cloudy        -1~ 12  74   0.0  NW 1-2
                let line = format!(
                    "{}  {:12} {:>3}~{:<3}  {:2} {:3}    {:2} {}",
                    &entry.date[5..],
                    entry.text,
                    entry.temp_min,
                    entry.temp_max,
                    entry.humidity,
                    entry.precipitation,
                    entry.wind_dir,
                    entry.wind_scale
                );
                screen.text(16, 176 + idx * 16, 16, &line, Color::Black)?;
            }
        }

        if !weather.hour.is_empty() && cycle % 3 == 1 {
            screen.rectangle(0, 136, SCREEN_WIDTH, 16, Color::Red)?;
            let line = "WEATHER FORECAST";
            let x_pos = (SCREEN_WIDTH - line.len() * 8) / 2; // center display
            screen.text(x_pos, 136, 16, line, Color::White)?;

            // y == 160
            // XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
            // Hour   Brief   Temp/°C  Hour   Brief   Temp/°C
            // 13:00  Cloudy       -1
            let line = format!("Hour   Brief   Temp/°C  Hour   Brief   Temp/°C");
            screen.text(16, 160, 16, &line, Color::Red)?;
            if weather.hour.len() >= 14 {
                for idx in 0..7 {
                    let entry_left = &weather.hour[idx];
                    let entry_right = &weather.hour[idx + 7];
                    let line = format!(
                        "{}  {:11} {:>3}  {}  {:11} {:>3} ",
                        &entry_left.time[11..=15],
                        entry_left.text,
                        entry_left.temperature,
                        &entry_right.time[11..=15],
                        entry_right.text,
                        entry_right.temperature,
                    );
                    screen.text(16, 176 + idx * 16, 16, &line, Color::Black)?;
                }
            }
        }

        if cycle % 3 == 2 {
            screen.rectangle(0, 136, SCREEN_WIDTH, 16, Color::Red)?;
            let line = "QUOTE";
            let x_pos = (SCREEN_WIDTH - line.len() * 8) / 2; // center display
            screen.text(x_pos, 136, 16, line, Color::White)?;
            let idx = random::<usize>() % QUOTE_LIST.lines().count();
            let mut quote = QUOTE_LIST.lines();
            for _ in 0..idx {
                _ = quote.next();
            }
            let quote = quote.next().unwrap_or_default();
            let quote = reformat(quote, 46);
            screen.text(16, 176, 16, &quote, Color::Black)?;
        }

        device.draw(&screen)?;
        cycle += 1;
    }
    // Ok(())
}

fn reformat(input: &str, width: usize) -> String {
    let mut output = String::new();
    let mut x_pos = 0;
    let blocks = input.split(' ');
    for item in blocks {
        if x_pos + item.len() > width {
            output.push('\n');
            x_pos = 0;
        }
        output.push_str(item);
        output.push(' ');
        x_pos += item.len() + 1;
    }
    output
}
