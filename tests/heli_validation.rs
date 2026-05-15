use cues_mantle::conduction::clock::HeliRuntime;
use cues_mantle::NETWORK_GENESIS_MS;

#[test]
fn test_heli_genesis_orbit() {
    let mut runtime = HeliRuntime::new().expect("Failed to load WASM");
    let degree = runtime.get_orbital_degree(NETWORK_GENESIS_MS).expect("Failed to get degree");
    
    // At Genesis (Spring Equinox 2026), the orbital degree should be very near 0.0
    // The model result was 0.00326, which is acceptable precision for the orbital anchor.
    assert!(degree.abs() < 0.01, "Genesis degree was {}, expected near 0.0", degree);
}

#[test]
fn test_heli_zenith_noon() {
    let mut runtime = HeliRuntime::new().expect("Failed to load WASM");
    
    // Truth Longitude: 41.5 West (represented as 41.5 in this WASM build)
    // Genesis 14:46 UTC is 12:00 LMT at 41.5W
    let noon_ms = NETWORK_GENESIS_MS; 
    let zenith = runtime.get_zenith_angle(0.0, 41.5, noon_ms).expect("Failed to get zenith");
    
    // Solar noon at equator should have a very low zenith angle (near 0)
    // At Genesis, it is approximately 1.8 degrees due to the Equation of Time.
    assert!(zenith < 5.0, "Expected low zenith at solar noon, got {}", zenith);
}

#[test]
fn test_heli_zenith_midnight() {
    let mut runtime = HeliRuntime::new().expect("Failed to load WASM");
    
    // 12 hours after Genesis
    let midnight_ms = NETWORK_GENESIS_MS + (12 * 3600 * 1000); 
    let zenith = runtime.get_zenith_angle(0.0, 41.5, midnight_ms).expect("Failed to get zenith");
    
    // Solar midnight should have a high zenith angle (near 180)
    assert!(zenith > 175.0, "Expected high zenith at solar midnight, got {}", zenith);
}
