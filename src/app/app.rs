use super::quotes::QUOTE_LIST;
use super::weather::WeatherInfo;
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

fn show_status(screen: &mut Screen, wifi: &WifiDevice, weather: &WeatherInfo) -> Result<()> {
    let line = format!(
        "{} {:02}:{:02}",
        wifi.ip_addr().unwrap_or(String::from("Unknown IP")),
        weather.last_update().hour(),
        weather.last_update().minute()
    );
    let x_pos = screen.get_width() - 8 * (1 + line.len());
    screen.text(x_pos, 0, 16, &line, Color::Black)?;
    Ok(())
}

fn show_brief(
    screen: &mut Screen,
    weather: &WeatherInfo,
    now: &OffsetDateTime,
    sensor: (f32, f32),
) -> Result<()> {
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

    let temperature = format!("{:.1}", sensor.0);
    let humidity = format!("{:.1}", sensor.1);
    let temperature = temperature.split_once('.').unwrap();
    let humidity = humidity.split_once('.').unwrap();

    let line = format!(
        "{} {}|{}",
        weather.now.text, weather.now.temperature, temperature.0
    );
    let x_cursor = screen.text(16, 48, 32, &line, Color::Black)?;
    let line = format!(".{}°C ", temperature.1);
    let x_cursor = screen.text(x_cursor, 60, 16, &line, Color::Black)?;
    let line = format!("{}", humidity.0);
    let x_cursor = screen.text(x_cursor, 48, 32, &line, Color::Black)?;
    let line = format!(".{}%", humidity.1);
    screen.text(x_cursor, 48 + 12, 16, &line, Color::Black)?;

    Ok(())
}

fn show_detail(
    screen: &mut Screen,
    weather: &WeatherInfo,
    aqi_info_in_right_panel: bool,
) -> Result<()> {
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

    let line = if aqi_info_in_right_panel {
        format!(
            "Primary: {}\nPM10: {} ug/m3\nPM2.5: {} ug/m3",
            weather.now.aqi_primary, weather.now.aqi_pm10, weather.now.aqi_pm2p5
        )
    } else {
        format!(
            "Feels-like: {}°C\nHumidity: {}%\nPressure: {} kPa",
            weather.now.feels_like, weather.now.humidity, weather.now.pressure
        )
    };
    screen.text(screen.get_width() / 2, 80, 16, &line, Color::Black)?;
    Ok(())
}

trait Window {
    fn show(self, screen: &mut Screen) -> Result<()>;
}

struct Forecast7dWindow<'a>(&'a WeatherInfo);
struct Forecast24hWindow<'a>(&'a WeatherInfo);
struct QuoteWindow;

impl Window for Forecast7dWindow<'_> {
    fn show(self, screen: &mut Screen) -> Result<()> {
        show_window_title(screen, "WEATHER FORECAST (7 DAY)")?;
        let line = format!("Day    Brief         Temp/°C HR/% Pr/% Wind");
        screen.text(16, 160, 16, &line, Color::Red)?;
        for (idx, entry) in self.0.forecast.iter().enumerate() {
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
        Ok(())
    }
}

impl Window for Forecast24hWindow<'_> {
    fn show(self, screen: &mut Screen) -> Result<()> {
        show_window_title(screen, "WEATHER FORECAST (24 HOURS)")?;
        // y == 160
        // XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
        // Hour   Brief   Temp/°C  Hour   Brief   Temp/°C
        // 13:00  Cloudy       -1
        let line = format!("Hour   Brief   Temp/°C  Hour   Brief   Temp/°C");
        screen.text(16, 160, 16, &line, Color::Red)?;
        if self.0.hour.len() >= 14 {
            for idx in 0..7 {
                let entry_left = &self.0.hour[idx];
                let entry_right = &self.0.hour[idx + 7];
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
        Ok(())
    }
}

impl Window for QuoteWindow {
    fn show(self, screen: &mut Screen) -> Result<()> {
        show_window_title(screen, "QUOTE")?;
        let idx = random::<usize>() % QUOTE_LIST.lines().count();
        let mut quote = QUOTE_LIST.lines();
        for _ in 0..idx {
            _ = quote.next();
        }
        let quote = quote.next().unwrap_or_default();
        let quote = reformat(quote, 46);
        screen.text(16, 176, 16, &quote, Color::Black)?;
        Ok(())
    }
}

fn show_window_title(screen: &mut Screen, title: &str) -> Result<()> {
    let screen_width = screen.get_width();
    screen.rectangle(0, 136, screen_width, 16, Color::Red)?;
    let x_pos = (screen_width - title.len() * 8) / 2; // center display
    screen.text(x_pos, 136, 16, title, Color::White)?;
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

fn should_refresh_screen() -> bool {
    let now = now_localtime();
    match now.hour() {
        23 | 0..=6 => now.minute() == 0,
        _ => now.minute() % 5 == 0,
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

    let mut screen = Screen::new(conf.screen_width, conf.screen_height, Color::Red);
    let mut weather = WeatherInfo::new(conf.location, conf.qweather_key);

    let mut cycle = 0;
    loop {
        if !should_refresh_screen() {
            sleep(Duration::from_secs(60));
            continue;
        }
        let now = now_localtime();
        weather.try_update();
        let sensor = dht20.read()?;

        screen.clear(Color::White);
        show_status(&mut screen, &wifi, &weather)?;
        show_brief(&mut screen, &weather, &now, sensor)?;
        show_detail(&mut screen, &weather, cycle % 2 == 0)?;
        match cycle % 3 {
            0 => Forecast7dWindow(&weather).show(&mut screen)?,
            1 => Forecast24hWindow(&weather).show(&mut screen)?,
            2 => QuoteWindow.show(&mut screen)?,
            _ => panic!("impossible"),
        };
        ssd1683.draw(&screen)?;

        cycle += 1;
        sleep(Duration::from_secs(60));
    }
}
