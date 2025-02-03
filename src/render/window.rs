use std::sync::Arc;
use std::time::{Duration, Instant};

use super::state::State;
use imgui::FontSource;
use imgui_wgpu::{Renderer, RendererConfig};
use imgui_winit_support::WinitPlatform;
use wgpu::TextureView;
use winit::{
    event::{Event, KeyEvent, WindowEvent},
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
    demo_open: bool,
}

struct App<'a> {
    fps_counter: FpsCounter,
    state: State<'a>,
    imgui: Option<ImguiState>,
}

impl<'a> App<'a> {
    pub fn new(state: State<'a>) -> App<'a> {
        let fps_counter = FpsCounter::new();

        let mut app = Self {
            fps_counter,
            state,
            imgui: None,
        };

        app.setup_imgui();
        app
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
        let demo_open = true;

        self.imgui = Some(ImguiState {
            context,
            platform,
            renderer,
            demo_open,
        })
    }

    fn render(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let frame = match self.state.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(e) => {
                eprintln!("dropped frame: {e:?}");
                return;
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        match self.state.scene_render(&view) {
            Ok(_) => {}
            Err(wgpu::SurfaceError::Lost) => self.state.resize(self.state.size),
            Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
            Err(e) => eprintln!("{:?}", e),
        };

        self.imgui_render(&view);

        frame.present();
    }

    fn imgui_render(&mut self, view: &TextureView) {
        let imgui = self.imgui.as_mut().unwrap();
        imgui
            .platform
            .prepare_frame(imgui.context.io_mut(), self.state.window())
            .expect("imgui failed to prepare frame");

        let ui = imgui.context.frame();

        {
            ui.show_demo_window(&mut imgui.demo_open);
        }

        let mut encoder = self
            .state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        imgui.platform.prepare_render(ui, self.state.window());

        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    // load: wgpu::LoadOp::Clear(imgui.clear_color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        imgui
            .renderer
            .render(
                imgui.context.render(),
                &self.state.queue,
                &self.state.device,
                &mut rpass,
            )
            .expect("imgui render failed");
        drop(rpass);
        self.state.queue.submit(Some(encoder.finish()));
    }
}

impl winit::application::ApplicationHandler for App<'_> {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {}

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
                self.render(event_loop);
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

        let imgui = self.imgui.as_mut().unwrap();
        imgui.platform.handle_event::<()>(
            imgui.context.io_mut(),
            self.state.window(),
            &Event::WindowEvent { window_id, event },
        );
    }
}

pub async fn render() {
    let event_loop = EventLoop::new().unwrap();

    let window_attributes = winit::window::WindowAttributes::default()
        .with_title("code adventure")
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600));

    // todo: move "create window procedure" to resume
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
