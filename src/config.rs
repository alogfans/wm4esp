#[toml_cfg::toml_config]
pub struct Config {
    #[default("")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_psk: &'static str,
    #[default("")]
    pub qweather_key: &'static str,
    #[default("")]
    pub location: &'static str,
    #[default("")]
    pub city: &'static str,
}
