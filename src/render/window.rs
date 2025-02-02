use std::sync::Arc;
use std::time::{Duration, Instant};

use super::state::State;
use imgui::{Context, FontSource, MouseCursor};
use imgui_wgpu::{Renderer, RendererConfig};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use winit::{
    event::{KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard,
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

struct ImguiState {
    context: imgui::Context,
    platform: WinitPlatform,
    renderer: Renderer,
    clear_color: wgpu::Color,
    demo_open: bool,
    last_frame: Instant,
    last_cursor: Option<MouseCursor>,
}

struct App<'a> {
    fps_counter: FpsCounter,
    state: State<'a>,
    // winit_platform: WinitPlatform,
    // imgui_context: Context,
    imgui: Option<ImguiState>,
}

impl<'a> App<'a> {
    pub fn new(state: State<'a>) -> App<'a> {
        let fps_counter = FpsCounter::new();
        // let mut imgui_context = Context::create();
        // let mut winit_platform = WinitPlatform::new(&mut imgui_context); // step 1
        // winit_platform.attach_window(imgui_context.io_mut(), &state.window(), HiDpiMode::Default); // step 2
        Self {
            fps_counter,
            state,
            imgui: None,
            // winit_platform,
            // imgui_context,
        }
    }

    pub fn setup_imgui(&mut self) {
        let mut context = imgui::Context::create();
        let mut platform = imgui_winit_support::WinitPlatform::new(&mut context);
        platform.attach_window(
            context.io_mut(),
            self.state.window(),
            imgui_winit_support::HiDpiMode::Default,
        );
        context.set_ini_filename(None);

        let hidpi_factor = 1.0;
        let font_size = (13.0 * hidpi_factor) as f32;
        context.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

        context.fonts().add_font(&[FontSource::DefaultFontData {
            config: Some(imgui::FontConfig {
                oversample_h: 1,
                pixel_snap_h: true,
                size_pixels: font_size,
                ..Default::default()
            }),
        }]);

        //
        // Set up dear imgui wgpu renderer
        //
        let clear_color = wgpu::Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        };

        let renderer_config = RendererConfig {
            texture_format: self.state.config.format,
            ..Default::default()
        };

        let renderer = Renderer::new(
            &mut context,
            &self.state.device,
            &self.state.queue,
            renderer_config,
        );
        let last_frame = Instant::now();
        let last_cursor = None;
        let demo_open = true;

        self.imgui = Some(ImguiState {
            context,
            platform,
            renderer,
            clear_color,
            demo_open,
            last_frame,
            last_cursor,
        })
    }
}

impl<'a> winit::application::ApplicationHandler for App<'a> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        // todo: make create window here
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // self.winit_platform
        //     .prepare_frame(self.imgui_context.io_mut(), &self.state.window()) // step 4
        //     .expect("Failed to prepare frame");

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
                // let imgui = &mut self.imgui_context;
                // let drawData = imgui.render();
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

        // self.winit_platform.handle_event::<()>(
        //     self.imgui_context.io_mut(),
        //     &self.state.window(),
        //     &Event::WindowEvent { window_id, event },
        // );
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

    let state = State::new(window.clone()).await;
    let mut app = App::new(state);

    event_loop.run_app(&mut app).unwrap();
}
