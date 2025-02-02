use std::sync::Arc;
use std::time::{Duration, Instant};

use super::state::State;
use imgui::Context;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use winit::event::KeyEvent;
use winit::keyboard;
use winit::{
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
};

struct FpsCounter {
    timestamp: Instant,
    count: u64,
}

impl FpsCounter {
    fn new() -> Self {
        Self {
            timestamp: Instant::now(),
            count: 0,
        }
    }
    fn count(&mut self) {
        self.count += 1;
        let du = Instant::now() - self.timestamp;
        if du >= Duration::from_secs(1) {
            println!("fps: {}", self.count);
            self.count = 0;
            self.timestamp = Instant::now();
        }
    }
}

struct App<'a> {
    fps_counter: FpsCounter,
    state: State<'a>,
}

impl<'a> App<'a> {
    pub fn new(state: State<'a>) -> App<'a> {
        let fps_counter = FpsCounter::new();
        Self { fps_counter, state }
    }
}

impl<'a> winit::application::ApplicationHandler for App<'a> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        // todo: make create window here
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.state.window().request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if window_id != self.state.window().id() {
            return;
        }

        // println!(
        //     "[{:?}] receive event WindowEvent {:?}, {:?}",
        //     chrono::Local::now(),
        //     window_id,
        //     event
        // );

        self.state.input(&event);

        match event {
            WindowEvent::RedrawRequested => {
                self.state.update();
                match self.state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => self.state.resize(self.state.size),
                    Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                    Err(e) => eprintln!("{:?}", e),
                };
                self.fps_counter.count();
            }
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: keyboard::Key::Named(keyboard::NamedKey::Escape),
                        ..
                    },
                ..
            } => {
                println!("received {:?}. EXIT", event);
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                self.state.resize(physical_size);
                println!("WindowEvent::Resized {:?}", physical_size);
            }
            _ => {} // }
        }
    }
}

pub async fn render() {
    let event_loop = EventLoop::new().unwrap();

    let window_attributes = winit::window::WindowAttributes::default()
        .with_title("code adventure")
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600));

    // todo
    let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

    #[cfg(target_arch = "wasm32")]
    {
        // Winit prevents sizing with CSS, so we have to set
        // the size manually when on web.
        window.set_inner_size(winit::dpi::PhysicalSize::new(450, 400));

        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas());
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }

    // create a window

    let mut imgui = Context::create();
    let mut platform = WinitPlatform::new(&mut imgui); // step 1
    platform.attach_window(imgui.io_mut(), &window, HiDpiMode::Default); // step 2

    let state = State::new(window.clone()).await;
    let mut app = App::new(state);

    event_loop.run_app(&mut app).unwrap();
}
