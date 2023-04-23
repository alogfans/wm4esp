use super::quotes::QUOTE_LIST;
use super::weather::WeatherInfo;
use super::weather_icons::extract_image;
use crate::common::Config;
use crate::display::{Color, Screen};
use crate::error::Result;
use crate::network::http::HttpServer;
use crate::network::wifi::WifiDevice;
use crate::peripheral::{dht20::DHT20, ssd1683::SSD1683};

use rand::random;
use std::thread::sleep;
use std::time::Duration;
use time::OffsetDateTime;
use time_macros::offset;

fn show_status(
    screen: &mut Screen,
    city: &str,
    wifi: &WifiDevice,
    weather: &WeatherInfo,
    now: &OffsetDateTime,
) -> Result<()> {
    let line = format!(
        "{} | {} | {:02}:{:02} | {:02}:{:02}",
        city,
        wifi.ip_addr().unwrap_or(String::from("Unknown IP")),
        weather.last_update().hour(),
        weather.last_update().minute(),
        now.hour(),
        now.minute()
    );
    screen.rectangle(
        0,
        screen.get_height() - 18,
        screen.get_width(),
        1,
        Color::Black,
    )?;
    screen.text(8, screen.get_height() - 16, 16, &line, Color::Black)?;
    Ok(())
}

fn show_top_frame(
    screen: &mut Screen,
    weather: &WeatherInfo,
    now: &OffsetDateTime,
    sensor: (f32, f32),
) -> Result<()> {
    let image = extract_image(weather.now.icon);
    if let Some(image) = image {
        screen.bitmap(8, 0, 64, 64, image, Color::Red)?;
    }

    let text = format!(
        "{} {:02}-{:02}\n{}°|",
        now.weekday().to_string(),
        now.month() as i32,
        now.day(),
        weather.now.temperature
    );

    let x_offset = screen.text(80, 0, 32, &text, Color::Red)?;
    let text = if weather.now.precipitation < 0.01 {
        format!(
            "{}\nAQI {} ({})",
            weather.now.text, weather.now.aqi, weather.now.aqi_category
        )
    } else {
        format!(
            "{} ({:1} mm/h)\nAQI {} ({})",
            weather.now.text, weather.now.precipitation, weather.now.aqi, weather.now.aqi_category
        )
    };
    screen.text(x_offset, 30, 16, &text, Color::Black)?;

    let temperature = format!("{:.1}", sensor.0);
    let humidity = format!("{:.1}", sensor.1);
    let temperature = temperature.split_once('.').unwrap();
    let humidity = humidity.split_once('.').unwrap();

    let line = format!("{}", temperature.0);
    let x_cursor = screen.text(328, 0, 32, &line, Color::Black)?;
    let line = format!(".{}°C ", temperature.1);
    screen.text(x_cursor, 12, 16, &line, Color::Black)?;

    let line = format!("{}", humidity.0);
    let x_cursor = screen.text(328, 32, 32, &line, Color::Black)?;
    let line = format!(".{}%", humidity.1);
    screen.text(x_cursor, 32 + 16 - 4, 16, &line, Color::Black)?;

    screen.rectangle(0, 64 + 3, screen.get_width(), 1, Color::Red)?;
    screen.rectangle(323, 0, 1, 64, Color::Red)?;

    let line = format!(
        "Wind: {} {} ({} km/h)\nHumidity: {}%",
        weather.now.wind_dir, weather.now.wind_scale, weather.now.wind_speed, weather.now.humidity
    );
    screen.text(8, 72, 16, &line, Color::Black)?;
    let line = format!(
        "PM  : {} ug/m3\nPM   : {} ug/m3",
        weather.now.aqi_pm10, weather.now.aqi_pm2p5
    );
    screen.text(screen.get_width() / 2, 72, 16, &line, Color::Black)?;
    let line = format!("  10\n  2.5");
    screen.text(screen.get_width() / 2, 72 + 4, 16, &line, Color::Black)?;
    screen.rectangle(0, 72 + 2 * 16 + 3, screen.get_width(), 2, Color::Red)?;
    Ok(())
}

fn get_bit(image: &[u8], size: usize, i: usize, j: usize) -> u8 {
    let pos = i * size + j;
    if image[pos / 8] & (1u8 << (7 - (pos % 8) as u8)) != 0 {
        1
    } else {
        0
    }
}

fn build_32x32_image(code: i32) -> Vec<u8> {
    if let Some(image) = extract_image(code) {
        let mut new_image = Vec::new();
        new_image.resize(32 * 32 / 8, 0);
        for i in 0..32 {
            for j in 0..32 {
                let val = get_bit(image, 64, i * 2, j * 2)
                    + get_bit(image, 64, i * 2 + 1, j * 2)
                    + get_bit(image, 64, i * 2, j * 2 + 1)
                    + get_bit(image, 64, i * 2 + 1, j * 2 + 1);
                if val >= 2 {
                    let pos = i * 32 + j;
                    new_image[pos / 8] |= 1u8 << (7 - (pos % 8) as u8);
                }
            }
        }
        new_image
    } else {
        Vec::new()
    }
}

fn show_left_frame(screen: &mut Screen, weather: &WeatherInfo) -> Result<()> {
    // start from y = 112
    let mut y_cursor = 112;
    for idx in [2, 5, 8] {
        let entry = &weather.hour[idx];
        let image = build_32x32_image(entry.icon);
        if !image.is_empty() {
            screen.bitmap(8, y_cursor, 32, 32, &image, Color::Red)?;
        }
        let text = format!("{}\n{}°C", &entry.time[11..=15], entry.temperature);
        screen.text(8 + 32, y_cursor, 16, &text, Color::Black)?;
        y_cursor += 32;
    }

    screen.rectangle(0, y_cursor + 4, 114, 1, Color::Red)?;
    y_cursor += 8;

    for idx in [1, 2] {
        let entry = &weather.forecast[idx];
        let image = build_32x32_image(entry.icon);
        if !image.is_empty() {
            screen.bitmap(8, y_cursor, 32, 32, &image, Color::Red)?;
        }
        let text = format!(
            "{}\n{}~{}°C",
            &entry.date[5..=9],
            entry.temp_min,
            entry.temp_max
        );
        screen.text(8 + 32, y_cursor, 16, &text, Color::Black)?;
        y_cursor += 32;
    }

    screen.rectangle(114, 112, 2, 170, Color::Red)?;

    Ok(())
}

fn show_quote(screen: &mut Screen) -> Result<()> {
    let idx = random::<usize>() % QUOTE_LIST.lines().count();
    let mut quote = QUOTE_LIST.lines();
    for _ in 0..idx {
        _ = quote.next();
    }
    let quote = quote.next().unwrap_or_default();
    let quote = reformat(quote, 30);
    screen.text(120, 112, 16, "Quote", Color::InvRed)?;
    screen.text(120, 112 + 16, 16, &quote, Color::Black)?;
    Ok(())
}

fn now_localtime() -> OffsetDateTime {
    time::OffsetDateTime::now_utc().to_offset(offset!(+8))
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

#[derive(Clone, PartialEq, Copy, Debug)]
enum Action {
    Screen,
    TimeOnly,
    None,
}

fn refresh_action() -> Action {
    let now = now_localtime();
    if now.second() != 0 {
        return Action::None;
    }
    let screen_update = match now.hour() {
        23 | 0..=6 => now.minute() == 0,
        _ => now.minute() % 10 == 0,
    };
    if screen_update {
        Action::Screen
    } else {
        Action::TimeOnly
    }
}

pub fn app_main(
    mut ssd1683: SSD1683,
    mut dht20: DHT20,
    wifi: WifiDevice,
    conf: Config,
) -> Result<()> {
    let mut httpd = HttpServer::new()?;
    httpd.add_handlers()?;

    let mut screen = Screen::new(conf.screen_width, conf.screen_height, Color::White);
    let mut weather = WeatherInfo::new(conf.location, conf.qweather_key);

    let mut cycle = 0;
    let mut first_draw = true;
    loop {
        let mut action = refresh_action();
        if first_draw {
            action = Action::Screen;
            first_draw = false;
        }

        if action == Action::None {
            sleep(Duration::from_secs(1));
            continue;
        }

        screen.clear(Color::White);
        let now = now_localtime();
        if action == Action::TimeOnly {
            sleep(Duration::from_secs(1));
            continue;
        }

        assert_eq!(action, Action::Screen);
        weather.try_update();
        let sensor = dht20.read()?;
        show_top_frame(&mut screen, &weather, &now, sensor)?;
        show_left_frame(&mut screen, &weather)?;
        show_quote(&mut screen)?;

        show_status(&mut screen, conf.city, &wifi, &weather, &now)?;
        ssd1683.draw(&screen, cycle % 3 != 0)?;
        cycle += 1;
        sleep(Duration::from_secs(1));
    }
}
