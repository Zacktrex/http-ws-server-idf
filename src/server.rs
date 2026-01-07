//! HTTP server and WiFi access point setup

use crate::config::{CHANNEL, PASSWORD, SSID, STACK_SIZE};
use anyhow::Result;
use embedded_svc::wifi::{self, AccessPointConfiguration, AuthMethod};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    http::server::EspHttpServer,
    nvs::EspDefaultNvsPartition,
    wifi::{BlockingWifi, EspWifi},
};
use esp_idf_svc::hal::modem::Modem;
use log::*;

/// Create and configure the HTTP server with WiFi access point
pub fn create_server(modem: Modem) -> Result<EspHttpServer<'static>> {
    info!("Creating HTTP server...");

    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;

    let wifi_configuration = wifi::Configuration::AccessPoint(AccessPointConfiguration {
        ssid: SSID.try_into().unwrap(),
        ssid_hidden: false, // Set to false to make SSID visible in WiFi scan lists
        auth_method: AuthMethod::WPA2Personal,
        password: PASSWORD.try_into().unwrap(),
        channel: CHANNEL,
        ..Default::default()
    });

    info!("Configuring Wi-Fi access point...");
    wifi.set_configuration(&wifi_configuration)?;
    wifi.start()?;
    wifi.wait_netif_up()?;

    info!("Created Wi-Fi with WIFI_SSID `{SSID}` and WIFI_PASS `{PASSWORD}`");

    let server_configuration = esp_idf_svc::http::server::Configuration {
        stack_size: STACK_SIZE,
        ..Default::default()
    };

    // Keep wifi running beyond when this function returns (forever)
    // Do not call this if you ever want to stop or access it later.
    // Otherwise it should be returned from this function and kept somewhere
    // so it does not go out of scope.
    // https://doc.rust-lang.org/stable/core/mem/fn.forget.html
    core::mem::forget(wifi);

    let server = EspHttpServer::new(&server_configuration)?;
    info!("HTTP server created successfully");
    Ok(server)
}

