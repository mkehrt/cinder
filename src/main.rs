use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use tokio;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

mod vulkan;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = Application::default();
    event_loop.run_app(&mut app)?;

    Ok(())
}

#[derive(Default)]
struct Application {
    window: Option<Window>,
    vulkan: Option<vulkan::Vulkan>,
}

// From https://docs.rs/winit/0.30.9/winit/index.html
impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(Window::default_attributes())
            .expect("Failed to create window.");

        let raw_display_handle = window
            .display_handle()
            .expect("Failed to get display handle.");
        let raw_window_handle = window
            .window_handle()
            .expect("Failed to get window handle.");
        
        let vulkan = vulkan::Vulkan::new(&raw_display_handle, &raw_window_handle).unwrap(); // Panic should print error.

        self.window = Some(window);
        self.vulkan = Some(vulkan);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => (),
        }
    }
}
