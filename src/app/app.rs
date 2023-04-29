use super::weather::WeatherInfo;
use super::weather_icons::extract_image;
use crate::config::Config;
use crate::display::{Color, Display};
use crate::error::Result;
use crate::network::http::HttpServer;
use crate::network::wifi::WifiDevice;
use crate::peripheral::{dht20::DHT20, ssd1683::SSD1683};

use embedded_graphics::prelude::*;
use std::thread::sleep;
use std::time::Duration;
use time::{OffsetDateTime, Weekday};
use time_macros::offset;
use u8g2_fonts::types::*;

fn show_status(
    screen: &mut Display,
    city: &str,
    wifi: &WifiDevice,
    weather: &WeatherInfo,
    now: &OffsetDateTime,
) -> Result<()> {
    let content = format!(
        "{} | {} | {:02}:{:02} | {:02}:{:02} (V2.1)",
        city,
        wifi.ip_addr().unwrap_or(String::from("N/A")),
        weather.last_update().hour(),
        weather.last_update().minute(),
        now.hour(),
        now.minute()
    );
    let position = Point::new(screen.get_width() as i32, screen.get_height() as i32);
    screen.render_text(
        &content,
        position,
        8,
        VerticalPosition::Bottom,
        HorizontalAlignment::Right,
        Color::Black,
    )?;
    Ok(())
}

fn show_top_frame(screen: &mut Display, weather: &WeatherInfo, now: &OffsetDateTime) -> Result<()> {
    let image = extract_image(weather.now.icon);
    if let Some(image) = image {
        screen.bitmap(0, 0, 64, 64, image, Color::Red)?;
    }

    let content = format!(
        "{:02}月{:02}日 {}\n{}°|",
        now.month() as i32,
        now.day(),
        weekday_to_string(now.weekday()),
        weather.now.temperature
    );

    let position = Point::new(80, 0);
    let x_offset = screen.render_text_legacy(&content, position, Color::Red)?;

    let content = format!(
        "{} ({} {} 级)\nAQI {}({}) PM2.5 {}",
        weather.now.text,
        weather.now.wind_dir,
        weather.now.wind_scale,
        weather.now.aqi,
        weather.now.aqi_category,
        weather.now.aqi_pm2p5
    );

    let position = Point::new(x_offset as i32, 32);
    screen.render_text(
        &content,
        position,
        16,
        VerticalPosition::Top,
        HorizontalAlignment::Left,
        Color::Black,
    )?;

    Ok(())
}

fn show_left_frame(screen: &mut Display, weather: &WeatherInfo, sensor: (f32, f32)) -> Result<()> {
    let content: String = format!("室温: {:.1}°C\n湿度: {:.1}%", sensor.0, sensor.1);
    let position = Point::new(0, 70);
    screen.render_text(
        &content,
        position,
        16,
        VerticalPosition::Top,
        HorizontalAlignment::Left,
        Color::Black,
    )?;

    if weather.hour.is_empty() || weather.forecast.is_empty() {
        return Ok(());
    }

    let mut y_cursor = 110;
    for idx in [2, 5, 8] {
        let entry = &weather.hour[idx];
        let image = build_32x32_image(entry.icon);
        if !image.is_empty() {
            screen.bitmap(0, y_cursor, 32, 32, &image, Color::Red)?;
        }
        let content: String = format!("{}\n{}°C", &entry.time[11..=15], entry.temperature);
        let position = Point::new(8 + 32, y_cursor as i32);
        screen.render_text(
            &content,
            position,
            14,
            VerticalPosition::Top,
            HorizontalAlignment::Left,
            Color::Black,
        )?;
        y_cursor += 36;
    }

    y_cursor += 8;

    for idx in [1, 2] {
        let entry = &weather.forecast[idx];
        let image = build_32x32_image(entry.icon);
        if !image.is_empty() {
            screen.bitmap(0, y_cursor, 32, 32, &image, Color::Red)?;
        }
        let content = format!(
            "{}\n{}~{}°C",
            &entry.date[5..=9],
            entry.temp_min,
            entry.temp_max
        );
        let position = Point::new(8 + 32, y_cursor as i32);
        screen.render_text(
            &content,
            position,
            14,
            VerticalPosition::Top,
            HorizontalAlignment::Left,
            Color::Black,
        )?;
        y_cursor += 36;
    }

    Ok(())
}

fn show_right_frame(screen: &mut Display, content: &str) -> Result<()> {
    let position = Point::new(120, 70);
    screen.render_text(
        content,
        position,
        16,
        VerticalPosition::Top,
        HorizontalAlignment::Left,
        Color::Black,
    )?;
    Ok(())
}

fn require_refresh(now: &OffsetDateTime) -> bool {
    now.minute() % 30 == 0 && now.second() == 0
}

pub fn app_main(
    mut ssd1683: SSD1683,
    mut dht20: DHT20,
    wifi: WifiDevice,
    conf: Config,
) -> Result<()> {
    let mut httpd = HttpServer::new()?;
    httpd.add_handlers()?;

    let mut display = Display::new(400, 300, Color::White);
    let mut weather = WeatherInfo::new(conf.location, conf.qweather_key);
    let mut first_draw = true;
    loop {
        let now = now_localtime();
        if first_draw || httpd.get_refresh_flag()? || require_refresh(&now) {
            first_draw = false;
            weather.try_update();
            let sensor = dht20.read()?;
            let content: String = httpd.get_note_content()?;
            display.clear(Color::White);
            show_top_frame(&mut display, &weather, &now)?;
            show_left_frame(&mut display, &weather, sensor)?;
            show_right_frame(&mut display, &content)?;
            show_status(&mut display, conf.city, &wifi, &weather, &now)?;
            ssd1683.draw(&display, false)?;
        }
        sleep(Duration::from_secs(1));
    }
}

fn weekday_to_string(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Monday => "星期一",
        Weekday::Tuesday => "星期二",
        Weekday::Wednesday => "星期三",
        Weekday::Thursday => "星期四",
        Weekday::Friday => "星期五",
        Weekday::Saturday => "星期六",
        Weekday::Sunday => "星期日",
    }
}

fn now_localtime() -> OffsetDateTime {
    time::OffsetDateTime::now_utc().to_offset(offset!(+8))
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
