use crate::error::{Result, WmError};
use crate::network::http::HttpClient;
use serde_json::Map;
use serde_json::Value;
use time::OffsetDateTime;
use time_macros::offset;

#[derive(Default)]
pub struct CurrentWeather {
    pub text: String,
    pub temperature: i32,
    pub feels_like: i32,
    pub humidity: i32,
    pub pressure: i32,
    pub precipitation: f32,
    pub wind_dir: String,
    pub wind_scale: i32,
    pub wind_speed: i32,
    pub aqi: i32,
    pub aqi_category: String,
    pub aqi_primary: String,
    pub aqi_pm10: i32,
    pub aqi_pm2p5: i32,
    pub icon: i32,
}

#[derive(Default)]
pub struct HourlyWeather {
    pub time: String,
    pub text: String,
    pub temperature: i32,
    pub humidity: i32,
    pub pressure: i32,
    pub precipitation: f32,
    pub wind_dir: String,
    pub wind_scale: String,
    pub wind_speed: i32,
    pub icon: i32,
}

#[derive(Default)]
pub struct DailyWeather {
    pub date: String,
    pub text: String,
    pub temp_min: i32,
    pub temp_max: i32,
    pub humidity: i32,
    pub precipitation: f32,
    pub wind_dir: String,
    pub wind_scale: String,
    pub icon: i32,
    pub sunrise: String,
    pub sunset: String,
}

pub struct WeatherInfo {
    last_update: time::OffsetDateTime,
    pub now: CurrentWeather,
    pub hourly: Vec<HourlyWeather>,
    pub daily: Vec<DailyWeather>,
    param: String,
}

impl Default for WeatherInfo {
    fn default() -> Self {
        let last_update = time::OffsetDateTime::UNIX_EPOCH;
        let now = CurrentWeather {
            ..Default::default()
        };
        WeatherInfo {
            last_update,
            now,
            hourly: Vec::new(),
            daily: Vec::new(),
            param: "".into(),
        }
    }
}

fn get_json_map(url: &str, key: &str) -> Result<Map<String, Value>> {
    let mut client = HttpClient::new()?;
    let result = client.get(url)?;
    let parsed: Value = serde_json::from_str(&result)?;
    let now = parsed[key].as_object();
    if let Some(now) = now {
        Ok(now.clone())
    } else {
        Err(WmError::InvalidArgument)
    }
}

fn get_json_vector(url: &str, key: &str) -> Result<Vec<Value>> {
    let mut client = HttpClient::new()?;
    let result = client.get(url)?;
    let parsed: Value = serde_json::from_str(&result)?;
    let now = parsed[key].as_array();
    if let Some(now) = now {
        Ok(now.clone())
    } else {
        Err(WmError::InvalidArgument)
    }
}

macro_rules! json_str {
    ($entry:expr, $item:literal) => {{
        let v = $entry.get($item);
        if let Some(v) = v {
            String::from(v.as_str().unwrap_or_default())
        } else {
            String::from("")
        }
    }};
}

macro_rules! json_i32 {
    ($entry:expr, $item:literal) => {{
        let v = $entry.get($item);
        if let Some(v) = v {
            v.as_str()
                .unwrap_or_default()
                .parse::<i32>()
                .unwrap_or_default()
        } else {
            0
        }
    }};
}

macro_rules! json_f32 {
    ($entry:expr, $item:literal) => {{
        let v = $entry.get($item);
        if let Some(v) = v {
            v.as_str()
                .unwrap_or_default()
                .parse::<f32>()
                .unwrap_or_default()
        } else {
            0.0
        }
    }};
}

impl WeatherInfo {
    pub fn new(location: &str, key: &str) -> Self {
        let param = format!("location={}&key={}&lang=cn", location, key);
        WeatherInfo {
            param,
            ..Default::default()
        }
    }

    fn try_update_current_weather(&mut self, now: OffsetDateTime) {
        let url = format!("https://devapi.qweather.com/v7/weather/now?{}", self.param);
        let weather = get_json_map(&url, "now");

        let url = format!("https://devapi.qweather.com/v7/air/now?{}", self.param);
        let aqi = get_json_map(&url, "now");

        if let Ok(weather) = weather {
            if let Ok(aqi) = aqi {
                self.now = CurrentWeather {
                    text: json_str!(weather, "text"),
                    temperature: json_i32!(weather, "temp"),
                    feels_like: json_i32!(weather, "feelsLike"),
                    humidity: json_i32!(weather, "humidity"),
                    pressure: json_i32!(weather, "pressure"),
                    precipitation: json_f32!(weather, "precip"),
                    wind_dir: json_str!(weather, "windDir"),
                    wind_scale: json_i32!(weather, "windScale"),
                    wind_speed: json_i32!(weather, "windSpeed"),
                    aqi: json_i32!(aqi, "aqi"),
                    aqi_category: json_str!(aqi, "category"),
                    aqi_primary: json_str!(aqi, "primary"),
                    aqi_pm10: json_i32!(aqi, "pm10"),
                    aqi_pm2p5: json_i32!(aqi, "pm2p5"),
                    icon: json_i32!(weather, "icon"),
                };
                self.last_update = now;
            }
        }
    }

    fn try_update_daily_weather(&mut self) {
        let url = format!("https://devapi.qweather.com/v7/weather/3d?{}", self.param);
        let weather = get_json_vector(&url, "daily");
        if let Ok(weather) = weather {
            self.daily.clear();
            for entry in weather.iter() {
                if let Some(entry) = entry.as_object() {
                    let result = DailyWeather {
                        date: json_str!(entry, "fxDate"),
                        text: json_str!(entry, "textDay"),
                        temp_min: json_i32!(entry, "tempMin"),
                        temp_max: json_i32!(entry, "tempMax"),
                        humidity: json_i32!(entry, "humidity"),
                        wind_dir: json_str!(entry, "windDirDay"),
                        wind_scale: json_str!(entry, "windScaleDay"),
                        precipitation: json_f32!(entry, "precip"),
                        icon: json_i32!(entry, "iconDay"),
                        sunrise: json_str!(entry, "sunrise"),
                        sunset: json_str!(entry, "sunset"),
                    };
                    self.daily.push(result);
                }
            }
        }
    }

    fn _try_update_hourly_weather(&mut self) {
        let url = format!("https://devapi.qweather.com/v7/weather/24h?{}", self.param);
        let weather = get_json_vector(&url, "hourly");
        if let Ok(weather) = weather {
            self.hourly.clear();
            for entry in weather.iter() {
                if let Some(entry) = entry.as_object() {
                    let result = HourlyWeather {
                        time: json_str!(entry, "fxTime"),
                        text: json_str!(entry, "text"),
                        temperature: json_i32!(entry, "temp"),
                        humidity: json_i32!(entry, "humidity"),
                        pressure: json_i32!(entry, "pressure"),
                        precipitation: json_f32!(entry, "precip"),
                        wind_dir: json_str!(entry, "windDir"),
                        wind_scale: json_str!(entry, "windScale"),
                        wind_speed: json_i32!(entry, "windSpeed"),
                        icon: json_i32!(entry, "icon"),
                    };
                    self.hourly.push(result);
                }
            }
        }
    }

    pub fn try_update(&mut self) {
        let now = time::OffsetDateTime::now_utc();
        self.try_update_current_weather(now);
        self.try_update_daily_weather();
        // self._try_update_hourly_weather();
    }

    pub fn last_update(&self) -> time::OffsetDateTime {
        self.last_update.to_offset(offset!(+8))
    }
}
