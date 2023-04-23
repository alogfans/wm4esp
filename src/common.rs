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
    #[default(400)]
    pub screen_width: usize,
    #[default(300)]
    pub screen_height: usize,
    #[default("")]
    pub city: &'static str,
}
