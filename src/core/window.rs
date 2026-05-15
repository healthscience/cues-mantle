use std::sync::Arc;
use winit::window::{Window, WindowAttributes};
use winit::event_loop::ActiveEventLoop;

pub struct WindowSurface {
    pub window: Arc<Window>,
}

impl WindowSurface {
    pub fn new(event_loop: &ActiveEventLoop, title: &str) -> Self {
        let window_attributes = WindowAttributes::default()
            .with_title(title)
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0));
        
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
        
        Self { window }
    }
}
