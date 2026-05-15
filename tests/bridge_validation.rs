use cues_mantle::conduction::bridge::Bridge;

#[test]
fn test_bridge_allocation() {
    let count = 100;
    let bridge = Bridge::new(count);
    
    assert_eq!(bridge.particles.len(), count);
    
    // Check initial state
    for (i, p) in bridge.particles.iter().enumerate() {
        assert_eq!(p.x, i as f32 * 10.0);
        assert_eq!(p.y, 360.0);
        assert_eq!(p.vx, 0.0);
        assert_eq!(p.vy, 0.0);
    }
}

#[test]
fn test_bridge_memory_layout() {
    use std::mem::size_of;
    use cues_mantle::conduction::bridge::Particle;
    
    // Ensure Particle size is what we expect (4 * f32 = 16 bytes)
    assert_eq!(size_of::<Particle>(), 16);
}
