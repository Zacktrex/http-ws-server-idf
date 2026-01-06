//! RSSI (Received Signal Strength Indicator) and distance calculation

use log::*;

/// Calculate distance from RSSI using log-distance path loss model
/// RSSI: Received Signal Strength Indicator in dBm
/// Returns distance in meters
/// Accounts for walls and obstacles which significantly weaken signal
pub fn calculate_distance_from_rssi(rssi: i8) -> f32 {
    // Path loss parameters for indoor environments with walls/obstacles
    // Path loss exponent:
    //   2.0 = free space (no obstacles)
    //   2.5-3.0 = indoor with few walls
    //   3.0-4.0 = indoor with many walls/obstacles (typical home/office)
    //   4.0+ = heavy obstacles (concrete walls, multiple floors)
    // Using 3.5 for realistic indoor environment with walls
    const PATH_LOSS_EXPONENT: f32 = 3.5;

    // Reference distance (1 meter) and reference RSSI at 1m
    // RSSI at 1m is typically -30 to -40 dBm for ESP32-C3
    const REFERENCE_DISTANCE: f32 = 1.0;
    const RSSI_AT_1M: f32 = -35.0; // Typical RSSI at 1 meter distance (no obstacles)

    let rssi_f32 = rssi as f32;

    // Log-distance path loss model with wall attenuation:
    // RSSI = RSSI_AT_1M - 10 * N * log10(distance / reference_distance) - wall_loss
    // Solving for distance:
    // distance = reference_distance * 10^((RSSI_AT_1M - RSSI) / (10 * N))
    // Higher path loss exponent accounts for walls reducing signal strength

    let distance =
        REFERENCE_DISTANCE * 10.0_f32.powf((RSSI_AT_1M - rssi_f32) / (10.0 * PATH_LOSS_EXPONENT));

    info!(
        "RSSI: {} dBm, Calculated distance (before clamp): {:.2} m",
        rssi, distance
    );

    // Increased range to account for walls weakening signal
    // Walls can reduce signal by 10-20 dB per wall, making devices appear further
    // Clamp to reasonable values (0.1m to 200m for indoor/outdoor with obstacles)
    let clamped_distance = distance.max(0.1).min(200.0);

    if clamped_distance == 200.0 && distance > 200.0 {
        warn!(
            "Distance calculated as {:.2}m, clamped to 200m. Signal very weak (possibly through many walls).",
            distance
        );
    }

    clamped_distance
}

/// Get RSSI from connected station
/// Note: This is a simplified implementation that gets RSSI from the first connected station
pub fn get_station_rssi() -> Option<i8> {
    unsafe {
        use esp_idf_svc::sys::*;

        // Allocate buffer for station list
        let mut sta_list: wifi_sta_list_t = std::mem::zeroed();
        let ret = esp_wifi_ap_get_sta_list(&mut sta_list);

        info!(
            "esp_wifi_ap_get_sta_list returned: {}, num stations: {}",
            ret, sta_list.num
        );

        if ret == ESP_OK as i32 && sta_list.num > 0 {
            // Get RSSI from first connected station
            // In a real scenario, you'd match the station by MAC address
            let rssi = sta_list.sta[0].rssi;
            info!("Station RSSI: {} dBm", rssi);
            Some(rssi)
        } else {
            warn!("No connected stations or error getting station list");
            None
        }
    }
}

