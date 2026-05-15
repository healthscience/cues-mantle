use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::event_loop::{ControlFlow, EventLoop};
use cues_mantle::core::engine::{Engine, TestMode};
use cues_mantle::core::window::WindowSurface;

#[derive(Default)]
struct App {
    window_surface: Option<WindowSurface>,
    engine: Option<Engine>,
    test_mode: TestMode,
}

impl App {
    fn new(test_mode: TestMode) -> Self {
        Self {
            window_surface: None,
            engine: None,
            test_mode,
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
            }
            
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
                if let Some(engine) = &mut self.engine {
                    engine.resize(physical_size);
                }
                if let Some(ws) = &self.window_surface {
                    ws.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(engine) = &mut self.engine {
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
