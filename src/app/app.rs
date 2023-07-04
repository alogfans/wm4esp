use super::weather::{DailyWeather, WeatherInfo};
use super::weather_icons::extract_icon;
use crate::config::Config;
use crate::display::{Color, Display};
use crate::error::Result;
use crate::network::http::HttpServer;
use crate::network::wifi::WifiDevice;
use crate::peripheral::{dht20::DHT20, ssd1683::SSD1683};

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle, StyledDrawable};
use u8g2_fonts::{fonts, types::*, FontRenderer};

use std::thread::sleep;
use std::time::Duration;
use time::{OffsetDateTime, Weekday};
use time_macros::offset;

fn show_status(display: &mut Display, wifi: &WifiDevice, now: &OffsetDateTime) -> Result<()> {
    let content = format!(
        "{} | {:02}:{:02} | V2.2",
        wifi.ip_addr().unwrap_or(String::from("N/A")),
        now.hour(),
        now.minute()
    );

    let position = Point::new(display.get_width() as i32, display.get_height() as i32);
    let font = FontRenderer::new::<fonts::u8g2_font_6x10_mf>().with_ignore_unknown_chars(true);
    font.render_aligned(
        &content as &str,
        position,
        VerticalPosition::Bottom,
        HorizontalAlignment::Right,
        FontColor::Transparent(Color::Black),
        display,
    )?;

    Ok(())
}

fn draw_today(display: &mut Display, base_point: Point, now: &OffsetDateTime) -> Result<()> {
    Rectangle::new(
        base_point,
        Size {
            width: 128,
            height: 128,
        },
    )
    .draw_styled(&PrimitiveStyle::with_fill(Color::Red), display)?;

    // let elapsed_days = now.date().ordinal();
    // let width = elapsed_days as u32 * 128 / 365;
    // Rectangle::new(base_point, Size { width, height: 4 })
    //     .draw_styled(&PrimitiveStyle::with_fill(Color::White), display)?;

    // Draw Day
    let content = format!("{}", now.day());
    let position = base_point
        + Point {
            x: 128 / 2,
            y: 128 / 2,
        };
    let font =
        FontRenderer::new::<fonts::u8g2_font_logisoso46_tn>().with_ignore_unknown_chars(true);
    font.render_aligned(
        &content as &str,
        position,
        VerticalPosition::Center,
        HorizontalAlignment::Center,
        FontColor::Transparent(Color::White),
        display,
    )?;

    // Draw YY/MM and Weekday
    let content = format!(
        "{}/{} {}",
        now.year(),
        now.month() as i32,
        weekday_to_string(now.weekday())
    );
    let font =
        FontRenderer::new::<fonts::u8g2_font_wqy16_t_gb2312>().with_ignore_unknown_chars(true);
    let position = base_point
        + Point {
            x: 128 / 2,
            y: 128 - 8,
        };

    font.render_aligned(
        &content as &str,
        position,
        VerticalPosition::Bottom,
        HorizontalAlignment::Center,
        FontColor::Transparent(Color::White),
        display,
    )?;

    Ok(())
}

fn draw_attribute(display: &mut Display, base_point: Point, key: &str, value: &str) -> Result<()> {
    let font =
        FontRenderer::new::<fonts::u8g2_font_wqy12_t_gb2312a>().with_ignore_unknown_chars(true);

    let position = base_point + Point { x: 0, y: 0 };
    font.render_aligned(
        key,
        position,
        VerticalPosition::Top,
        HorizontalAlignment::Left,
        FontColor::Transparent(Color::Red),
        display,
    )?;

    let font =
        FontRenderer::new::<fonts::u8g2_font_logisoso16_tr>().with_ignore_unknown_chars(true);
    let position = base_point + Point { x: 0, y: 17 };
    font.render_aligned(
        value,
        position,
        VerticalPosition::Top,
        HorizontalAlignment::Left,
        FontColor::Transparent(Color::Red),
        display,
    )?;

    Ok(())
}

fn draw_top_banner(
    display: &mut Display,
    base_point: Point,
    weather: &WeatherInfo,
    sensor: (f32, f32),
) -> Result<()> {
    if let Some(bitmap) = extract_icon(weather.now.icon) {
        display.bitmap(
            base_point.x as usize,
            base_point.y as usize,
            64,
            64,
            bitmap,
            Color::Red,
        )?;
    }

    let content = if weather.now.aqi_primary == "NA" {
        format!(
            "{} {} {} 级\n空气质量 {} ({})",
            weather.now.text,
            weather.now.wind_dir,
            weather.now.wind_scale,
            weather.now.aqi_category,
            weather.now.aqi
        )
    } else {
        format!(
            "{} {} {} 级\n空气质量 {} ({}) {}",
            weather.now.text,
            weather.now.wind_dir,
            weather.now.wind_scale,
            weather.now.aqi_category,
            weather.now.aqi,
            weather.now.aqi_primary
        )
    };

    if weather.valid {
        let font =
            FontRenderer::new::<fonts::u8g2_font_wqy16_t_gb2312>().with_ignore_unknown_chars(true);
        font.render_aligned(
            &content as &str,
            base_point + Point::new(64 + 8, 4),
            VerticalPosition::Top,
            HorizontalAlignment::Left,
            FontColor::Transparent(Color::Black),
            display,
        )?;
    }

    let position = base_point + Point::new(64 + 8, 24 + 20);
    let content = format!("{}|{}", weather.now.temperature, weather.now.humidity);
    if weather.valid {
        draw_attribute(display, position, "室外 °C|%", &content)?;
    }

    let position = base_point + Point::new(64 + 8 + 96, 24 + 20);
    let content = format!("{:.1}|{:.1}", sensor.0, sensor.1);
    draw_attribute(display, position, "室内 °C|%", &content)?;

    if !weather.valid {
        return Ok(());
    }

    let position = base_point + Point::new(0, 24 + 64);
    let content = format!("{}", weather.now.aqi_pm10);
    draw_attribute(display, position, "PM10 ug", &content)?;

    let position = position + Point::new(36 + 16, 0);
    let content = format!("{}", weather.now.aqi_pm2p5);
    draw_attribute(display, position, "PM2.5 ug", &content)?;

    let position = position + Point::new(36 + 16, 0);
    let content = format!("{:.1}", weather.now.precipitation);
    draw_attribute(display, position, "降水 mm", &content)?;

    let position = position + Point::new(36 + 16, 0);
    let content = format!("{:.1}", weather.now.feels_like);
    draw_attribute(display, position, "体感 °C", &content)?;

    let position = position + Point::new(36 + 16, 0);
    let content = format!("{}", weather.now.pressure);
    draw_attribute(display, position, "气压 hPa", &content)?;

    Ok(())
}

fn draw_forecast_item(
    display: &mut Display,
    base_point: Point,
    entry: &DailyWeather,
    is_today: bool,
) -> Result<()> {
    let icon = build_32x32_icon(entry.icon);

    if !icon.is_empty() {
        display.bitmap(
            base_point.x as usize,
            base_point.y as usize,
            32,
            32,
            &icon,
            Color::Red,
        )?;
    }

    let content = if is_today {
        format!(
            "{}\n{}~{}°C\n日出 {}\n日落 {}",
            &entry.date[5..=9],
            entry.temp_min,
            entry.temp_max,
            entry.sunrise,
            entry.sunset,
        )
    } else {
        format!(
            "{}\n{}~{}°C",
            &entry.date[5..=9],
            entry.temp_min,
            entry.temp_max
        )
    };

    let font =
        FontRenderer::new::<fonts::u8g2_font_wqy16_t_gb2312>().with_ignore_unknown_chars(true);
    font.render_aligned(
        &content as &str,
        base_point + Point::new(36, 0),
        VerticalPosition::Top,
        HorizontalAlignment::Left,
        FontColor::Transparent(Color::Black),
        display,
    )?;

    Ok(())
}

fn draw_common_part(
    display: &mut Display,
    weather: &WeatherInfo,
    now: &OffsetDateTime,
    sensor: (f32, f32),
) -> Result<()> {
    let mut base_point = display.bounding_box().top_left;
    draw_today(display, base_point, now)?;
    base_point += Point::new(128 + 8, 0);
    draw_top_banner(display, base_point, weather, sensor)?;

    base_point = display.bounding_box().top_left + Point::new(0, 128 + 8);
    if weather.daily.is_empty() {
        return Ok(());
    }

    let mut position = base_point;
    for idx in [0, 1, 2] {
        let entry = &weather.daily[idx];
        if idx == 0 {
            draw_forecast_item(display, position, entry, true)?;
            position += Point::new(0, 80);
        } else {
            draw_forecast_item(display, position, entry, false)?;
            position += Point::new(0, 40);
        }
        if position.y >= display.bounding_box().size.height as i32 {
            break;
        }
    }

    Ok(())
}

fn draw_custom_part(display: &mut Display, content: &str) -> Result<()> {
    let position = Point::new(128 + 8, (128 + 300) / 2);
    let font = if content.is_ascii() {
        FontRenderer::new::<fonts::u8g2_font_courR10_tf>()
    } else {
        FontRenderer::new::<fonts::u8g2_font_wqy16_t_gb2312>()
    }
    .with_ignore_unknown_chars(true);
    font.render_aligned(
        content,
        position,
        VerticalPosition::Center,
        HorizontalAlignment::Left,
        FontColor::Transparent(Color::Red),
        display,
    )?;
    Ok(())
}

fn require_refresh(now: &OffsetDateTime) -> bool {
    if now.minute() != 0 || now.second() != 0 {
        return false;
    }
    match now.hour() {
        7..=23 => true,
        _ => false,
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
    let mut weather = WeatherInfo::new(conf.location, conf.qweather_key);
    let mut first_draw = true;
    let mut sensor = dht20.read()?;
    loop {
        let now = now_localtime();
        if now.second() == 0 && now.minute() % 5 == 0 {
            sensor = dht20.read()?;
            httpd.add_sensor_data(now, sensor)?;
        }
        if first_draw || httpd.get_refresh_flag()? || require_refresh(&now) {
            first_draw = false;
            weather.try_update();
            let content: String = httpd.get_note_content()?;
            let mut display = Display::new(400, 300, Color::White);
            display.clear(Color::White);
            draw_common_part(&mut display, &weather, &now, sensor)?;
            draw_custom_part(&mut display, &content)?;
            show_status(&mut display, &wifi, &now)?;
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

fn build_32x32_icon(code: i32) -> Vec<u8> {
    if let Some(image) = extract_icon(code) {
        let mut new_image = Vec::new();
        new_image.resize(32 * 32 / 8, 0);
        for i in 0..32 {
            for j in 0..32 {
                let val = get_bit(image, 64, i * 2, j * 2)
                    + get_bit(image, 64, i * 2 + 1, j * 2)
                    + get_bit(image, 64, i * 2, j * 2 + 1)
                    + get_bit(image, 64, i * 2 + 1, j * 2 + 1);
                if val > 2 {
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
