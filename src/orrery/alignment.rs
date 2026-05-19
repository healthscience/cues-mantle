use crate::conduction::clock::HeliRuntime;
use crate::orrery::hub::TestMode;
use crate::{NETWORK_GENESIS_MS, TROPICAL_YEAR_MS};
use chrono::Utc;

pub struct HeliAlignment;

impl HeliAlignment {
    pub fn get_solar_status(heli: Option<&mut HeliRuntime>, test_mode: TestMode) -> (String, wgpu::Color) {
        if heli.is_none() {
            return (
                "Cues Mantle | Age: ??? | Temporal Axis Initializing...".to_string(),
                wgpu::Color { r: 0.05, g: 0.05, b: 0.1, a: 1.0 }
            );
        }
        let heli = heli.unwrap();

        let now = match test_mode {
            TestMode::SolarNoon => NETWORK_GENESIS_MS, 
            TestMode::SolarMidnight => NETWORK_GENESIS_MS + (12 * 3600 * 1000), 
            TestMode::None => Utc::now().timestamp_millis(),
        };
        
        let orbital_degree = heli.get_orbital_degree(now).unwrap_or(0.0);
        let network_age = (now - NETWORK_GENESIS_MS) as f64 / TROPICAL_YEAR_MS;

        let truth_lat = 0.0;
        let truth_lon = 41.5; 
        let zenith = heli.get_zenith_angle(truth_lat, truth_lon, now).unwrap_or(90.0);
        
        let is_day = zenith < 90.0;

        let factor = (1.0f64 - (zenith / 180.0)).powi(2);
        let background_color = if is_day {
            wgpu::Color {
                r: 0.1 * factor,
                g: 0.4 * factor,
                b: 0.9 * factor,
                a: 1.0,
            }
        } else {
            wgpu::Color {
                r: 0.02 * factor,
                g: 0.02 * factor,
                b: 0.05 * factor,
                a: 1.0,
            }
        };

        let status = format!(
            "Cues Mantle | Age: {:.8} | Degree: {:.4}° | Zenith: {:.2}° | {}",
            network_age,
            orbital_degree,
            zenith,
            if test_mode != TestMode::None { "TEST MODE" } else if is_day { "Day (Truth)" } else { "Night (Truth)" }
        );

        (status, background_color)
    }
}
