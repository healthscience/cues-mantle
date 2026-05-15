use std::sync::Arc;
use winit::window::Window;
use glyphon::{
    Buffer, Color, FontSystem, Metrics, Resolution, SwashCache, TextArea, TextAtlas,
    TextRenderer,
};
use chrono::Utc;
use crate::conduction::clock::HeliRuntime;
use crate::conduction::runtime::{JsRuntime, uv_default_loop};
use crate::conduction::bridge::{Bridge, Particle};
use crate::render::pipeline::RenderPipeline;
use crate::{NETWORK_GENESIS_MS, TROPICAL_YEAR_MS};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TestMode {
    #[default]
    None,
    SolarNoon,
    SolarMidnight,
}

pub struct Engine {
    pub render_pipeline: RenderPipeline,
    pub size: winit::dpi::PhysicalSize<u32>,
    
    // Text rendering fields
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,
    pub viewport: glyphon::Viewport,
    pub atlas: TextAtlas,
    pub text_renderer: TextRenderer,
    pub text_buffer: Buffer,

    // Conduction
    pub heli_runtime: HeliRuntime,
    pub js_runtime: JsRuntime,
    pub bridge: Bridge,
    
    pub particle_buffer: wgpu::Buffer,
    pub test_mode: TestMode,
}

impl Engine {
    pub async fn new(window: Arc<Window>, test_mode: TestMode) -> Self {
        let size = window.inner_size();
        let render_pipeline = RenderPipeline::new(window).await;

        // Conduction Setup
        let mut heli_runtime = HeliRuntime::new().expect("Failed to initialize Heli WASM");
        
        let uv_loop = unsafe { uv_default_loop() };
        let mut js_runtime = JsRuntime::new(uv_loop).expect("Failed to initialize bare-js");
        
        let bridge = Bridge::new(100);
        bridge.inject(js_runtime.env()).expect("Failed to inject bridge into JS");
        
        // Load particle logic
        js_runtime.load("assets/particles.js").ok(); // Optional for now

        // GPU Buffer for particles
        let particle_buffer = render_pipeline.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Buffer"),
            size: (bridge.particles.len() * std::mem::size_of::<Particle>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Text setup
        let mut font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = glyphon::Cache::new(&render_pipeline.device);
        let viewport = glyphon::Viewport::new(&render_pipeline.device, &cache);
        let mut atlas = TextAtlas::new(&render_pipeline.device, &render_pipeline.queue, &cache, render_pipeline.format);
        let text_renderer = TextRenderer::new(&mut atlas, &render_pipeline.device, Default::default(), None);
        let mut text_buffer = Buffer::new(&mut font_system, Metrics::new(32.0, 42.0));

        let (status, _bg) = Self::get_solar_status(&mut heli_runtime, test_mode);
        text_buffer.set_size(&mut font_system, Some(size.width as f32), Some(size.height as f32));
        text_buffer.set_text(&mut font_system, &status, glyphon::Attrs::new().family(glyphon::Family::SansSerif), cosmic_text::Shaping::Advanced);
        text_buffer.shape_until_scroll(&mut font_system, false);

        Self {
            render_pipeline,
            size,
            font_system,
            swash_cache,
            viewport,
            atlas,
            text_renderer,
            text_buffer,
            heli_runtime,
            js_runtime,
            bridge,
            particle_buffer,
            test_mode,
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.render_pipeline.resize(new_size.width, new_size.height);
            
            self.text_buffer.set_size(&mut self.font_system, Some(new_size.width as f32), Some(new_size.height as f32));
            self.viewport.update(&self.render_pipeline.queue, Resolution {
                width: new_size.width,
                height: new_size.height,
            });
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // 1. Metabolic Phase / JS Tick
        self.js_runtime.tick();

        // 2. Conduction Phase: Sync Bridge to GPU
        self.render_pipeline.queue.write_buffer(
            &self.particle_buffer,
            0,
            bytemuck::cast_slice(&self.bridge.particles),
        );

        let (solar_status, clear_color) = Self::get_solar_status(&mut self.heli_runtime, self.test_mode);

        let output = self.render_pipeline.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .render_pipeline.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            self.text_buffer.set_text(&mut self.font_system, &solar_status, glyphon::Attrs::new().family(glyphon::Family::SansSerif), cosmic_text::Shaping::Advanced);

            self.text_renderer.prepare(
                &self.render_pipeline.device,
                &self.render_pipeline.queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                [TextArea {
                    buffer: &self.text_buffer,
                    left: 40.0,
                    top: 40.0,
                    scale: 1.0,
                    bounds: glyphon::TextBounds {
                        left: 0,
                        top: 0,
                        right: self.size.width as i32,
                        bottom: self.size.height as i32,
                    },
                    default_color: Color::rgb(255, 255, 255),
                    custom_glyphs: &[],
                }],
                &mut self.swash_cache,
            ).unwrap();

            self.text_renderer.render(&self.atlas, &self.viewport, &mut render_pass).unwrap();
        }

        self.render_pipeline.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn get_solar_status(heli: &mut HeliRuntime, test_mode: TestMode) -> (String, wgpu::Color) {
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

        let factor = (1.0 - (zenith / 180.0)).powi(2);
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
