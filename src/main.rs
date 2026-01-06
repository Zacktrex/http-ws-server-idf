//! HTTP/WebSocket Server with contexts
//!
//! Go to http://192.168.71.1 to play

mod config;
mod guessing_game;
// mod oled;
mod rssi;
mod server;
mod utils;

use core::cmp::Ordering;
use embedded_svc::{http::Method, io::Write, ws::FrameType};
use esp_idf_svc::sys::{EspError, ESP_ERR_INVALID_SIZE};
use log::*;
use std::{collections::BTreeMap, ffi::CStr, sync::Mutex};

use crate::config::{INDEX_HTML, MAX_LEN};
use crate::guessing_game::GuessingGame;
// use crate::oled::display_message;
use crate::rssi::{calculate_distance_from_rssi, get_station_rssi};
use crate::server::create_server;
use crate::utils::{nth, rand};


fn main() -> anyhow::Result<()> {
    esp_idf_svc::sys::link_patches();
    esp_idf_svc::log::EspLogger::initialize_default();

    info!("Starting HTTP/WebSocket server...");

    let mut server = create_server()?;

    server.fn_handler("/", Method::Get, |req| {
        info!("Serving index page to client from {}", req.uri());
        let mut resp = req
            .into_response(200, Some("OK"), &[
                ("Content-Type", "text/html; charset=utf-8"),
                ("Cache-Control", "no-cache, no-store, must-revalidate"),
                ("Pragma", "no-cache"),
                ("Expires", "0"),
                ("Connection", "keep-alive"),
            ])
            .map_err(|e| {
                error!("Error creating response: {:?}", e);
                EspError::from_infallible::<ESP_ERR_INVALID_SIZE>()
            })?;
        resp.write_all(INDEX_HTML.as_bytes()).map_err(|e| {
            error!("Error writing response: {:?}", e);
            EspError::from_infallible::<ESP_ERR_INVALID_SIZE>()
        })?;
        info!("Index page served successfully");
        Ok::<(), EspError>(())
    })?;

    // Health check endpoint
    server.fn_handler("/health", Method::Get, |req| {
        info!("Health check request from {}", req.uri());
        let mut resp = req
            .into_response(200, Some("OK"), &[("Content-Type", "text/plain")])
            .map_err(|e| {
                error!("Error creating health response: {:?}", e);
                EspError::from_infallible::<ESP_ERR_INVALID_SIZE>()
            })?;
        resp.write_all(b"OK").map_err(|e| {
            error!("Error writing health response: {:?}", e);
            EspError::from_infallible::<ESP_ERR_INVALID_SIZE>()
        })?;
        Ok::<(), EspError>(())
    })?;

    // Add endpoint to get RSSI and distance
    server.fn_handler("/rssi", Method::Get, |req| {
        info!("RSSI request received");
        let rssi = get_station_rssi();

        let response = if let Some(rssi_value) = rssi {
            let distance = calculate_distance_from_rssi(rssi_value);
            info!(
                "Sending RSSI response: RSSI={} dBm, Distance={:.2} m",
                rssi_value, distance
            );
            format!(
                r#"{{"rssi": {}, "distance": {:.2}, "unit": "meters", "raw_distance": {:.4}}}"#,
                rssi_value, distance, distance
            )
        } else {
            warn!("No RSSI available - no connected stations");
            r#"{"rssi": null, "distance": null, "error": "No connected station"}"#.to_string()
        };

        let mut resp = req
            .into_response(200, Some("OK"), &[("Content-Type", "application/json")])
            .map_err(|e| {
                error!("Error creating response: {:?}", e);
                EspError::from_infallible::<ESP_ERR_INVALID_SIZE>()
            })?;
        resp.write_all(response.as_bytes()).map_err(|e| {
            error!("Error writing response: {:?}", e);
            EspError::from_infallible::<ESP_ERR_INVALID_SIZE>()
        })?;
        Ok::<(), EspError>(())
    })?;

    let guessing_games = Mutex::new(BTreeMap::<i32, GuessingGame>::new());

    server.ws_handler("/ws/guess", move |ws| {
        let mut sessions = guessing_games.lock().unwrap();
        if ws.is_new() {
            let secret = (rand() % 100) + 1;
            sessions.insert(ws.session(), GuessingGame::new(secret));
            info!(
                "New WebSocket session {} ({} open)",
                ws.session(),
                sessions.len()
            );

            // Send welcome message
            let welcome_msg = "Welcome to the guessing game! Enter a number between 1 and 100".to_string();

            ws.send(FrameType::Text(false), welcome_msg.as_bytes())?;
            return Ok(());
        } else if ws.is_closed() {
            sessions.remove(&ws.session());
            info!(
                "Closed WebSocket session {} ({} open)",
                ws.session(),
                sessions.len()
            );
            return Ok(());
        }

        let session = sessions.get_mut(&ws.session()).unwrap();

        // NOTE: Due to the way the underlying C implementation works, ws.recv()
        // may only be called with an empty buffer exactly once to receive the
        // incoming buffer size, then must be called exactly once to receive the
        // actual payload.
        let (_frame_type, len) = match ws.recv(&mut []) {
            Ok(frame) => {
                let len = frame.1;
                debug!("Received frame of length: {}", len);
                frame
            }
            Err(e) => {
                error!("Error receiving frame: {:?}", e);
                return Err(e);
            }
        };

        if len > MAX_LEN {
            warn!("Request too big: {} bytes (max: {})", len, MAX_LEN);
            ws.send(FrameType::Text(false), "Request too big".as_bytes())?;
            ws.send(FrameType::Close, &[])?;
            return Err(EspError::from_infallible::<ESP_ERR_INVALID_SIZE>());
        }

        let mut buf = [0; MAX_LEN]; // Small digit buffer can go on the stack
        ws.recv(buf.as_mut())?;

        let Ok(user_string) = CStr::from_bytes_until_nul(&buf[..len]) else {
            warn!("Failed to decode C string from buffer");
            ws.send(FrameType::Text(false), "[CStr decode Error]".as_bytes())?;
            return Ok(());
        };

        let Ok(user_string) = user_string.to_str() else {
            warn!("Failed to decode UTF-8 string");
            ws.send(FrameType::Text(false), "[UTF-8 Error]".as_bytes())?;
            return Ok(());
        };

        let Some(user_guess) = GuessingGame::parse_guess(user_string) else {
            info!("Invalid guess from client: {}", user_string);
            ws.send(
                FrameType::Text(false),
                "Please enter a number between 1 and 100".as_bytes(),
            )?;
            return Ok(());
        };

            match session.guess(user_guess) {
            (Ordering::Greater, n) => {
                let reply = format!("Your {} guess was too high", nth(n));
                info!("Sending reply: {}", reply);
                ws.send(FrameType::Text(false), reply.as_ref())?;
            }
            (Ordering::Less, n) => {
                let reply = format!("Your {} guess was too low", nth(n));
                info!("Sending reply: {}", reply);
                ws.send(FrameType::Text(false), reply.as_ref())?;
            }
            (Ordering::Equal, n) => {
                let reply = format!(
                    "You guessed {} on your {} try! Refresh to play again",
                    session.secret(),
                    nth(n)
                );
                info!("Game won! Sending reply: {}", reply);
                ws.send(FrameType::Text(false), reply.as_ref())?;
                ws.send(FrameType::Close, &[])?;
            }
        }

        Ok::<(), EspError>(())
    })?;


    info!("Server started successfully. Waiting for connections...");

    // Keep server running beyond when main() returns (forever)
    // Do not call this if you ever want to stop or access it later.
    // Otherwise you can either add an infinite loop so the main task
    // never returns, or you can move it to another thread.
    // https://doc.rust-lang.org/stable/core/mem/fn.forget.html
    core::mem::forget(server);

    // Main task no longer needed, free up some memory
    Ok(())
}

