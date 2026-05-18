use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::event_loop::{ControlFlow, EventLoop};
use cues_mantle::core::engine::{Engine, TestMode};
use cues_mantle::core::window::WindowSurface;
use std::sync::mpsc::{channel, Receiver};
use cues_mantle::conduction::clock::HeliRuntime;
use cues_mantle::conduction::runtime::MantleRuntime;
use cues_mantle::conduction::bridge::Bridge;

struct RuntimeAssets {
    heli: HeliRuntime,
    js: MantleRuntime,
    bridge: Bridge,
}

struct App {
    window_surface: Option<WindowSurface>,
    engine: Option<Engine>,
    test_mode: TestMode,
    receiver: Option<Receiver<RuntimeAssets>>,
}

impl App {
    fn new(test_mode: TestMode) -> Self {
        Self {
            window_surface: None,
            engine: None,
            test_mode,
            receiver: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window_surface.is_none() {
            let window_surface = WindowSurface::new(event_loop, "Cues Mantle");
            self.window_surface = Some(window_surface);

            if let Some(ws) = &self.window_surface {
                let engine = pollster::block_on(Engine::new(ws.window.clone(), self.test_mode));
                self.engine = Some(engine);
                ws.window.request_redraw();
                
                // Spawn background loader
                let (tx, rx) = channel();
                self.receiver = Some(rx);
                
                std::thread::spawn(move || {
                    log::info!("Background Loader: Igniting Mantle & Heli...");
                    
                    let heli = HeliRuntime::new().expect("Failed to initialize Heli WASM");
                    let mut js = MantleRuntime::new().expect("Failed to initialize Mantle JS");
                    
                    // Step 3: Igniting the First Ripple - Run baseline sanity off-thread
                    if let Ok(sanity_script) = std::fs::read_to_string("assets/baseline_sanity.js") {
                        let _ = js.execute_string(&sanity_script);
                    }
                    
                    let bridge = Bridge::new(100);
                    bridge.inject(js.env()).expect("Failed to inject bridge");
                    js.load("assets/particles.js").ok();
                    
                    // Load the first local impulse file
                    if let Ok(mantle_js) = std::fs::read_to_string("target/release/mantle.js") {
                        log::info!("Background Loader: Injecting mantle.js impulse...");
                        let _ = js.execute_string(&mantle_js);
                    }
                    
                    let _ = tx.send(RuntimeAssets { heli, js, bridge });
                    log::info!("Background Loader: Assets ready.");
                });
            }
            
            log::info!("Cues Mantle Awake: Window surface ready, background loading started.");
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
                if let Some(engine) = &mut self.engine {
                    engine.resize(physical_size);
                }
                if let Some(ws) = &self.window_surface {
                    ws.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(engine) = &mut self.engine {
                    // log::debug!("Redraw Requested");
                    match engine.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => engine.resize(engine.size),
                        Err(wgpu::SurfaceError::OutOfMemory) => _event_loop.exit(),
                        Err(e) => log::error!("{:?}", e),
                    }
                }
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        // Check for background loader results
        if let Some(rx) = &self.receiver {
            if let Ok(assets) = rx.try_recv() {
                if let Some(engine) = &mut self.engine {
                    engine.activate(assets.heli, assets.js, assets.bridge);
                }
                self.receiver = None; // Loader finished
            }
        }

        if let Some(ws) = &self.window_surface {
            ws.window.request_redraw();
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
    event_loop.set_control_flow(ControlFlow::Poll);
    
    let mut app = App::new(test_mode);
    event_loop.run_app(&mut app).expect("Failed to run app");
}
