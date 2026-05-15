use chrono::Utc;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes};
use glyphon::{
    Buffer, Color, FontSystem, Metrics, Resolution, SwashCache, TextArea, TextAtlas,
    TextRenderer,
};
use cues_mantle::heli::HeliRuntime;
use cues_mantle::{NETWORK_GENESIS_MS, TROPICAL_YEAR_MS};


#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum TestMode {
    #[default]
    None,
    SolarNoon,
    SolarMidnight,
}

struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    
    // Text rendering fields
    font_system: FontSystem,
    swash_cache: SwashCache,
    viewport: glyphon::Viewport,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    text_buffer: Buffer,

    // Heli state
    heli_runtime: HeliRuntime,
    test_mode: TestMode,
}

fn get_solar_status(heli: &mut HeliRuntime, test_mode: TestMode) -> (String, wgpu::Color) {
    let now = match test_mode {
        TestMode::SolarNoon => NETWORK_GENESIS_MS, 
        TestMode::SolarMidnight => NETWORK_GENESIS_MS + (12 * 3600 * 1000), 
        TestMode::None => Utc::now().timestamp_millis(),
    };
    
    let orbital_degree = heli.get_orbital_degree(now).unwrap_or(0.0);
    let network_age = (now - NETWORK_GENESIS_MS) as f64 / TROPICAL_YEAR_MS;

    // Use Truth Longitude for solar check
    let truth_lat = 0.0; // Equator for network-wide reference
    // The WASM implementation treats East as negative and West as positive?
    // Based on test results, +41.5 gives the expected overhead sun at Genesis.
    let truth_lon = 41.5; 
    let zenith = heli.get_zenith_angle(truth_lat, truth_lon, now).unwrap_or(90.0);
    
    let is_day = zenith < 90.0;

    // Background color: Bright blue for High Noon (zenith near 0), black for Midnight (zenith near 180)
    // Map zenith 0-180 to bright blue -> black
    let factor = (1.0 - (zenith / 180.0)).powi(2); // Squared for deeper blacks
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

impl State {
    async fn new(window: Arc<Window>, test_mode: TestMode) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &config);

        // Heli Runtime setup
        let mut heli_runtime = HeliRuntime::new("assets/heli_clock_bg.wasm").expect("Failed to initialize Heli WASM");

        // Text setup
        let mut font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = glyphon::Cache::new(&device);
        let viewport = glyphon::Viewport::new(&device, &cache);
        let mut atlas = TextAtlas::new(&device, &queue, &cache, surface_format);
        let text_renderer = TextRenderer::new(&mut atlas, &device, Default::default(), None);
        let mut text_buffer = Buffer::new(&mut font_system, Metrics::new(32.0, 42.0));

        let (status, _bg) = get_solar_status(&mut heli_runtime, test_mode);
        text_buffer.set_size(&mut font_system, Some(size.width as f32), Some(size.height as f32));
        text_buffer.set_text(&mut font_system, &status, glyphon::Attrs::new().family(glyphon::Family::SansSerif), cosmic_text::Shaping::Advanced);
        text_buffer.shape_until_scroll(&mut font_system, false);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            font_system,
            swash_cache,
            viewport,
            atlas,
            text_renderer,
            text_buffer,
            heli_runtime,
            test_mode,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            
            self.text_buffer.set_size(&mut self.font_system, Some(new_size.width as f32), Some(new_size.height as f32));
            self.viewport.update(&self.queue, Resolution {
                width: new_size.width,
                height: new_size.height,
            });
        }
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let (solar_status, clear_color) = get_solar_status(&mut self.heli_runtime, self.test_mode);

        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
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
                &self.device,
                &self.queue,
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

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    state: Option<State>,
    test_mode: TestMode,
}

impl App {
    fn new(test_mode: TestMode) -> Self {
        Self {
            window: None,
            state: None,
            test_mode,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_none() {
            let window_attributes = WindowAttributes::default()
                .with_title("Cues Mantle")
                .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0));
            
            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
            self.window = Some(window.clone());

            let state = pollster::block_on(State::new(window, self.test_mode));
            self.state = Some(state);
            
            log::info!("Cues Mantle Awake: Graphics substrate and text engine initialized.");
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state: ElementState::Pressed,
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        ..
                    },
                ..
            } => {
                log::info!("Exiting Mantle...");
                _event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                if let Some(state) = &mut self.state {
                    state.resize(physical_size);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(state) = &mut self.state {
                    match state.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                        Err(wgpu::SurfaceError::OutOfMemory) => _event_loop.exit(),
                        Err(e) => log::error!("{:?}", e),
                    }
                }
            }
            _ => (),
        }
    }
}

fn main() {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    
    let args: Vec<String> = std::env::args().collect();
    let mut test_mode = TestMode::None;
    
    if args.iter().any(|arg| arg == "--test-solar-noon") {
        test_mode = TestMode::SolarNoon;
    } else if args.iter().any(|arg| arg == "--test-solar-midnight") {
        test_mode = TestMode::SolarMidnight;
    }

    let event_loop = EventLoop::new().expect("Failed to build event loop");
    event_loop.set_control_flow(ControlFlow::Wait);
    
    let mut app = App::new(test_mode);
    event_loop.run_app(&mut app).expect("Failed to run app");
}
