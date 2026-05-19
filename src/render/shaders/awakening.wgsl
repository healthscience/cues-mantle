struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

struct LogoUniforms {
    origin: vec2<f32>,
    scale: vec2<f32>,
    screen_size: vec2<f32>,
    tex_size: vec2<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: LogoUniforms;
@group(1) @binding(0) var t_diffuse: texture_2d<f32>;
@group(1) @binding(1) var s_diffuse: sampler;

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Calculate aspect ratios
    let screen_aspect = uniforms.screen_size.x / uniforms.screen_size.y;
    let tex_aspect = uniforms.tex_size.x / uniforms.tex_size.y;
    
    // Scale the 2D quad geometry. 
    // We treat uniforms.scale as a base scale factor.
    // We adjust by aspect ratios to maintain the original texture proportions in NDC space.
    var aspect_scale = vec2<f32>(1.0, 1.0);
    if (screen_aspect > 1.0) {
        // Wide screen: shrink X
        aspect_scale.x = tex_aspect / screen_aspect;
    } else {
        // Tall screen: shrink Y
        aspect_scale.y = (1.0 / tex_aspect) * screen_aspect;
    }
    
    let scaled_pos = model.position * uniforms.scale * aspect_scale;
    let translated_pos = scaled_pos + uniforms.origin;
    
    out.clip_position = vec4<f32>(translated_pos, 0.0, 1.0);
    out.tex_coords = model.tex_coords;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    if (color.a < 0.05) { discard; } // Clean alpha masking for the logo geometry
    return color;
}
