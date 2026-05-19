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
use crate::orrery::alignment::HeliAlignment;
use crate::render::pipeline::RenderPipeline;
use crate::render::awakening::{AwakeningPass, LogoUniforms};
use crate::substrate::MemorySubstrate;
use crate::substrate::schema::SubstrateSlot;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TestMode {
    #[default]
    None,
    SolarNoon,
    SolarMidnight,
}

pub struct Hub {
    pub render_pipeline: RenderPipeline,
    pub size: winit::dpi::PhysicalSize<u32>,
    
    // Text rendering fields
    pub font_system: FontSystem,
    pub swash_cache: SwashCache,
    pub viewport: glyphon::Viewport,
    pub atlas: TextAtlas,
    pub text_renderer: TextRenderer,
    pub text_buffer: Buffer,

    pub awakening_pass: AwakeningPass,

    // Conduction (now optional for instant startup)
    pub heli_runtime: Option<HeliRuntime>,
    pub js_runtime: Option<MantleRuntime>,
    pub bridge: Option<Bridge>,
    
    pub particle_buffer: Option<wgpu::Buffer>,
    pub test_mode: TestMode,
    pub boot_start: std::time::Instant,
    pub impulse_rx: Option<std::sync::mpsc::Receiver<String>>,
}

impl Hub {
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

        let awakening_pass = AwakeningPass::new(&render_pipeline);

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
            awakening_pass,
            heli_runtime: None, 
            js_runtime: None,
            bridge: None,
            particle_buffer: None,
            test_mode,
            boot_start: std::time::Instant::now(),
            impulse_rx: None,
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
        
        log::info!("Orrery Hub Activated: Components integrated.");
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
        // 0. Check for inbound script impulses waiting in the conduction channel
        if let Some(rx) = &self.impulse_rx {
            if let Ok(incoming_script) = rx.try_recv() {
                if let Some(js) = &mut self.js_runtime {
                    log::info!("Main Render Thread: Fusing incoming impulse into active Isolate context");
                    if let Err(e) = js.execute_string(&incoming_script) {
                        log::error!(" Catastrophic Exception: IMPULSE CRASH DETECTED: {:?}", e);
                    }
                }
            }
        }

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

        let (solar_status, mut clear_color) = HeliAlignment::get_solar_status(self.heli_runtime.as_mut(), self.test_mode);

        // Override clear color if impulse file has set it via shared substrate buffer
        if let Some(js) = &self.js_runtime {
            let substrate = js.read_substrate_floats();
            
            // If the alpha channel (slot 3) is set, we use the JS-driven color
            if substrate[3] > 0.0 {
                log::info!("JS-Driven Clear Color Active: [{}, {}, {}, {}]", substrate[0], substrate[1], substrate[2], substrate[3]);
                clear_color = wgpu::Color {
                    r: substrate[0] as f64,
                    g: substrate[1] as f64,
                    b: substrate[2] as f64,
                    a: substrate[3] as f64,
                };
            }

            // Update Awakening Logo Uniforms from substrate slots [4..7]
            // Default values if not set (0.0 usually means not set by JS yet, but specification says defaults are 0.0, -0.2, 0.25, 0.25)
            // We'll check if Logo Scale W is > 0.0 to assume JS has initialized it.
            if substrate[6] > 0.0 {
                self.awakening_pass.update_uniforms(&self.render_pipeline.queue, LogoUniforms {
                    origin: [substrate[4], substrate[5]],
                    scale: [substrate[6], substrate[7]],
                    screen_size: [self.size.width as f32, self.size.height as f32],
                    tex_size: [self.awakening_pass.tex_width, self.awakening_pass.tex_height],
                });
            } else {
                // Keep default aspect ratio correction even if JS hasn't touched it
                self.awakening_pass.update_uniforms(&self.render_pipeline.queue, LogoUniforms {
                    origin: [0.0, -0.2],
                    scale: [0.25, 0.25],
                    screen_size: [self.size.width as f32, self.size.height as f32],
                    tex_size: [self.awakening_pass.tex_width, self.awakening_pass.tex_height],
                });
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

            // Render Awakening UI (Logo)
            self.awakening_pass.render(&mut render_pass);
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

            // Render Awakening UI (Logo)
            // Ensure aspect ratio is correct during boot too
            self.awakening_pass.update_uniforms(&self.render_pipeline.queue, LogoUniforms {
                origin: [0.0, -0.2],
                scale: [0.25, 0.25],
                screen_size: [self.size.width as f32, self.size.height as f32],
                tex_size: [self.awakening_pass.tex_width, self.awakening_pass.tex_height],
            });
            self.awakening_pass.render(&mut render_pass);
        }

        self.render_pipeline.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}
