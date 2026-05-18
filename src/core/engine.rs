use std::sync::Arc;
use winit::window::Window;
use glyphon::{
    Buffer, Color, FontSystem, Metrics, Resolution, SwashCache, TextArea, TextAtlas,
    TextRenderer,
};
use chrono::Utc;
use crate::conduction::clock::HeliRuntime;
use crate::conduction::runtime::{MantleRuntime};
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

    // Conduction (now optional for instant startup)
    pub heli_runtime: Option<HeliRuntime>,
    pub js_runtime: Option<MantleRuntime>,
    pub bridge: Option<Bridge>,
    
    pub particle_buffer: Option<wgpu::Buffer>,
    pub test_mode: TestMode,
    pub boot_start: std::time::Instant,
}

impl Engine {
    pub async fn new(window: Arc<Window>, test_mode: TestMode) -> Self {
        let size = window.inner_size();
        let render_pipeline = RenderPipeline::new(window).await;
        
        // Text setup
        let mut font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = glyphon::Cache::new(&render_pipeline.device);
        let viewport = glyphon::Viewport::new(&render_pipeline.device, &cache);
        let mut atlas = TextAtlas::new(&render_pipeline.device, &render_pipeline.queue, &cache, render_pipeline.format);
        let text_renderer = TextRenderer::new(&mut atlas, &render_pipeline.device, Default::default(), None);
        let mut text_buffer = Buffer::new(&mut font_system, Metrics::new(32.0, 42.0));

        // Initial placeholder text for instant feedback
        text_buffer.set_size(&mut font_system, Some(size.width as f32), Some(size.height as f32));
        text_buffer.set_text(&mut font_system, "bring to be ...", glyphon::Attrs::new().family(glyphon::Family::SansSerif), cosmic_text::Shaping::Advanced);
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
            heli_runtime: None, 
            js_runtime: None,
            bridge: None,
            particle_buffer: None,
            test_mode,
            boot_start: std::time::Instant::now(),
        }
    }

    pub fn activate(&mut self, heli: HeliRuntime, js: MantleRuntime, bridge: Bridge) {
        use crate::conduction::bridge::Particle;
        
        // GPU Buffer for particles
        let particle_buffer = self.render_pipeline.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle Buffer"),
            size: (bridge.particles.len() * std::mem::size_of::<Particle>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        self.heli_runtime = Some(heli);
        self.js_runtime = Some(js);
        self.bridge = Some(bridge);
        self.particle_buffer = Some(particle_buffer);
        
        log::info!("Engine Activated: Components integrated.");
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
        // 1. Check if we are still booting
        if self.js_runtime.is_none() || self.bridge.is_none() {
            return self.render_booting();
        }

        // 2. Metabolic Phase / JS Tick
        if let Some(js) = &mut self.js_runtime {
            js.tick();
        }

        // 3. Conduction Phase: Sync Bridge to GPU
        if let (Some(bridge), Some(pb)) = (&self.bridge, &self.particle_buffer) {
            self.render_pipeline.queue.write_buffer(
                pb,
                0,
                bytemuck::cast_slice(&bridge.particles),
            );
        }

        let (solar_status, mut clear_color) = Self::get_solar_status(self.heli_runtime.as_mut(), self.test_mode);

        // Override clear color if impulse file has set it
        if let Some(js) = &self.js_runtime {
            if let Some(override_color) = js.get_clear_color() {
                clear_color = override_color;
            }
        }

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

    fn render_booting(&mut self) -> Result<(), wgpu::SurfaceError> {
        let elapsed = self.boot_start.elapsed().as_secs_f32();
        let pulse = (elapsed * 2.0).sin() * 0.5 + 0.5;
        
        let clear_color = wgpu::Color {
            r: 0.02 * pulse as f64,
            g: 0.02 * pulse as f64,
            b: 0.05 * pulse as f64,
            a: 1.0,
        };

        let output = self.render_pipeline.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.render_pipeline.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("Boot Encoder") });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Boot Pass"),
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

            self.text_buffer.set_text(&mut self.font_system, "bring to be ...", glyphon::Attrs::new().family(glyphon::Family::SansSerif), cosmic_text::Shaping::Advanced);
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
                    default_color: Color::rgb(200, 200, 255),
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

    pub fn get_solar_status(heli: Option<&mut HeliRuntime>, test_mode: TestMode) -> (String, wgpu::Color) {
        use crate::{NETWORK_GENESIS_MS, TROPICAL_YEAR_MS};
        use chrono::Utc;

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
