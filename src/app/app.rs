use super::quotes::QUOTE_LIST;
use super::weather::WeatherInfo;
use crate::common::Config;
use crate::display::{Color, Screen};
use crate::error::Result;
use crate::network::wifi::WifiDevice;
use crate::peripheral::{dht20::DHT20, ssd1683::SSD1683};

use rand::random;
use std::ops::Sub;
use std::thread::sleep;
use std::time::Duration;
use time_macros::offset;

pub fn app_main(
    mut ssd1683: SSD1683,
    mut dht20: DHT20,
    wifi: WifiDevice,
    conf: Config,
) -> Result<()> {
    let screen_height = conf.screen_height;
    let screen_width = conf.screen_width;

    let mut screen = Screen::new(screen_height, screen_width, Color::Black);

    let mut weather = WeatherInfo::new(conf.location, conf.qweather_key);
    let mut cycle = 0;
    let mut refresh_time = time::OffsetDateTime::UNIX_EPOCH;
    loop {
        let (temperature, humidity) = dht20.read()?;

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
            "{} {:02}:{:02}:{:02}",
            wifi.ip_addr().unwrap_or(String::from("N/A")),
            weather.last_update().hour(),
            weather.last_update().minute(),
            weather.last_update().second()
        );

        let x_pos = screen_width - 8 * (1 + line.len());
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
        screen.text(screen_width / 2, 80, 16, &line, Color::Black)?;

        if !weather.forecast.is_empty() && cycle % 3 == 0 {
            screen.rectangle(0, 136, screen_width, 16, Color::Red)?;
            let line = "WEATHER FORECAST";
            let x_pos = (screen_width - line.len() * 8) / 2; // center display
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
            screen.rectangle(0, 136, screen_width, 16, Color::Red)?;
            let line = "WEATHER FORECAST";
            let x_pos = (screen_width - line.len() * 8) / 2; // center display
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
            screen.rectangle(0, 136, screen_width, 16, Color::Red)?;
            let line = "QUOTE";
            let x_pos = (screen_width - line.len() * 8) / 2; // center display
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

        ssd1683.draw(&screen)?;
        cycle += 1;
    }
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
