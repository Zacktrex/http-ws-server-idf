//! Configuration constants and environment variable handling

macro_rules! get_env_or_default {
    ($env:literal, $default:literal) => {
        match option_env!($env) {
            Some(val) => val,
            None => $default,
        }
    };
}

pub const SSID: &str = get_env_or_default!("WIFI_SSID", "ESP32-Game");
pub const PASSWORD: &str = get_env_or_default!("WIFI_PASS", "password123");

pub static INDEX_HTML: &str = include_str!("http_ws_server_page.html");

// Max payload length for guessing game
pub const MAX_LEN: usize = 8;

// Need lots of stack to parse JSON
pub const STACK_SIZE: usize = 10240;

// Wi-Fi channel, between 1 and 11
pub const CHANNEL: u8 = 11;

