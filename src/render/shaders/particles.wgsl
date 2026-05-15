struct Particle {
    pos: vec2<f32>,
    vel: vec2<f32>,
};

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;

@vertex
fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> @builtin(position) vec4<f32> {
    // Very simple particle rendering logic for verification
    let p = particles[in_vertex_index / 6u];
    // ... logic for drawing a quad or similar ...
    return vec4<f32>(p.pos, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}
