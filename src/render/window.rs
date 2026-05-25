use std::sync::Arc;

use super::state::State;
#[cfg(not(target_arch = "wasm32"))]
use cgmath;
#[cfg(not(target_arch = "wasm32"))]
use imgui::FontSource;
#[cfg(not(target_arch = "wasm32"))]
use imgui_wgpu::RendererConfig;
#[cfg(not(target_arch = "wasm32"))]
use imgui_winit_support::WinitPlatform;
use web_time::{Duration, Instant};
#[cfg(not(target_arch = "wasm32"))]
use wgpu::TextureView;
#[cfg(not(target_arch = "wasm32"))]
use winit::event::Event;
use winit::{
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
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
            // 注释掉FPS打印，保留计算逻辑
            // println!("fps: {}", self.count);
            self.count = 0;
            self.timestamp = Instant::now();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
struct ImguiState {
    context: imgui::Context,
    platform: WinitPlatform,
    renderer: imgui_wgpu::Renderer,
}

struct App<'a> {
    fps_counter: FpsCounter,
    state: State<'a>,
    #[cfg(not(target_arch = "wasm32"))]
    imgui: Option<ImguiState>,
    should_exit: bool,
    last_frame_time: Instant,
}

impl<'a> App<'a> {
    pub fn new(state: State<'a>) -> App<'a> {
        let fps_counter = FpsCounter::new();

        let app = Self {
            fps_counter,
            state,
            #[cfg(not(target_arch = "wasm32"))]
            imgui: None,
            should_exit: false,
            last_frame_time: Instant::now(),
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut app = app;
            app.setup_imgui();
            app
        }

        #[cfg(target_arch = "wasm32")]
        app
    }

    fn frame_interval(&self) -> Option<Duration> {
        (self.state.max_fps > 0.0).then(|| Duration::from_secs_f64(1.0 / self.state.max_fps))
    }

    fn next_frame_time(&self) -> Option<Instant> {
        self.frame_interval()
            .and_then(|interval| self.last_frame_time.checked_add(interval))
    }

    fn should_render_now(&self) -> bool {
        self.next_frame_time()
            .map_or(true, |next_frame_time| Instant::now() >= next_frame_time)
    }

    #[cfg(not(target_arch = "wasm32"))]
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

        let renderer = imgui_wgpu::Renderer::new(
            &mut context,
            &self.state.device,
            &self.state.queue,
            renderer_config,
        );

        self.imgui = Some(ImguiState {
            context,
            platform,
            renderer,
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

        #[cfg(not(target_arch = "wasm32"))]
        self.imgui_render(&view);

        frame.present();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn imgui_render(&mut self, view: &TextureView) {
        let imgui = self.imgui.as_mut().unwrap();
        imgui
            .platform
            .prepare_frame(imgui.context.io_mut(), self.state.window())
            .expect("imgui failed to prepare frame");

        let ui = imgui.context.frame();

        {
            // ui.show_demo_window(&mut imgui.demo_open);

            // 创建一个 ImGui 窗口并显示相机位置
            let window_pos = [0.0, 0.0];
            ui.window("Info")
                .size([400.0, 350.0], imgui::Condition::FirstUseEver)
                .position(window_pos, imgui::Condition::FirstUseEver)
                .build(|| {
                    // Camera section
                    ui.text("Camera");
                    ui.separator();
                    if ui.button("Reset Camera") {
                        // Reset camera to initial position and target
                        self.state.camera.eye = cgmath::Point3::new(0.0, 0.0, 15.0);
                        self.state.camera.target = cgmath::Point3::new(0.0, 0.0, 0.0);
                    }
                    let camera_pos = &self.state.camera.eye;
                    let camera_target = &self.state.camera.target;
                    ui.text(format!(
                        "Camera Position: ({:.2}, {:.2}, {:.2})",
                        camera_pos.x, camera_pos.y, camera_pos.z
                    ));
                    ui.text(format!(
                        "Camera Target: ({:.2}, {:.2}, {:.2})",
                        camera_target.x, camera_target.y, camera_target.z
                    ));

                    // Physics section
                    ui.separator();
                    ui.text("Physics");
                    ui.separator();

                    // 添加一个开关来控制物理模拟
                    if ui.button(if self.state.phy_tick_trigger {
                        "Disable Physics"
                    } else {
                        "Enable Physics"
                    }) {
                        self.state.phy_tick_trigger = !self.state.phy_tick_trigger;
                    }

                    if ui.button("Reset Physics") {
                        // Reset physics simulation
                        self.state.reset_physics();
                    }

                    // 物理模拟启用时，Step 按钮置灰
                    if self.state.phy_tick_trigger {
                        ui.text_disabled("Step Physics");
                    } else if ui.button("Step Physics") {
                        // 当点击 Step 按钮时，设置单步执行标志
                        self.state.phy_single_step = true;
                    }

                    ui.separator();
                    // 显示当前实际 FPS
                    ui.text(format!("Current FPS: {:.1}", self.state.current_fps));

                    // FPS 限制控制
                    ui.text("FPS Limit");
                    let mut fps_limit = self.state.max_fps as f32;
                    if ui
                        .slider_config("##FPS", 0.0, 240.0)
                        .display_format("%.0f")
                        .build(&mut fps_limit)
                    {
                        self.state.max_fps = fps_limit as f64;
                    }
                    if fps_limit == 0.0 {
                        ui.text("Unlimited FPS");
                    } else {
                        ui.text(format!("Max FPS: {:.0}", fps_limit));
                    }

                    ui.separator();
                    if ui.button("Exit Application") {
                        // 设置退出标志
                        self.should_exit = true;
                    }
                });
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

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.should_exit {
            event_loop.exit();
            return;
        }

        match self.next_frame_time() {
            Some(next_frame_time) if Instant::now() < next_frame_time => {
                event_loop.set_control_flow(ControlFlow::WaitUntil(next_frame_time));
            }
            _ => {
                event_loop.set_control_flow(ControlFlow::Poll);
                self.state.window().request_redraw();
            }
        }
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
                if !self.should_render_now() {
                    return;
                }

                self.last_frame_time = Instant::now();
                self.state.update();
                self.render(event_loop);
                self.fps_counter.count();
            }
            WindowEvent::CloseRequested => {
                println!("received {:?}. EXIT", event);
                event_loop.exit();
            }
            WindowEvent::Resized(physical_size) => {
                self.state.resize(physical_size);
                println!("WindowEvent::Resized {:?}", physical_size);
            }
            _ => {} // }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let imgui = self.imgui.as_mut().unwrap();
            imgui.platform.handle_event::<()>(
                imgui.context.io_mut(),
                self.state.window(),
                &Event::WindowEvent { window_id, event },
            );
        }
    }
}

enum RenderEvent {
    Initialized(State<'static>),
}

struct RenderApp {
    app: Option<App<'static>>,
    initializing: bool,
    proxy: EventLoopProxy<RenderEvent>,
}

impl RenderApp {
    fn new(proxy: EventLoopProxy<RenderEvent>) -> Self {
        Self {
            app: None,
            initializing: false,
            proxy,
        }
    }
}

pub async fn render() {
    let event_loop = EventLoop::<RenderEvent>::with_user_event().build().unwrap();
    let proxy = event_loop.create_proxy();
    let render_app = RenderApp::new(proxy);

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(render_app);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut render_app = render_app;
        event_loop.run_app(&mut render_app).unwrap();
    }
}

impl winit::application::ApplicationHandler<RenderEvent> for RenderApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        // Only initialize the app once when the application is resumed for the first time
        if self.app.is_none() && !self.initializing {
            let window_attributes = winit::window::WindowAttributes::default()
                .with_title("LTPP")
                .with_inner_size(winit::dpi::LogicalSize::new(1920, 1080));

            let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

            #[cfg(target_arch = "wasm32")]
            {
                // Winit prevents sizing with CSS, so we have to set
                // the size manually when on web.
                let _ = window.request_inner_size(winit::dpi::PhysicalSize::new(450, 400));

                use winit::platform::web::WindowExtWebSys;
                web_sys::window()
                    .and_then(|win| win.document())
                    .and_then(|doc| {
                        let dst = doc.get_element_by_id("wasm-example")?;
                        let canvas = web_sys::Element::from(window.canvas()?);
                        dst.append_child(&canvas).ok()?;
                        Some(())
                    })
                    .expect("Couldn't append canvas to document body.");
            }

            #[cfg(target_arch = "wasm32")]
            {
                self.initializing = true;
                let proxy = self.proxy.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let state = State::new(window.clone()).await;
                    let _ = proxy.send_event(RenderEvent::Initialized(state));
                });
            }

            #[cfg(not(target_arch = "wasm32"))]
            {
                let state = pollster::block_on(State::new(window.clone()));
                self.app = Some(App::new(state));
            }
        }

        if let Some(app) = self.app.as_mut() {
            app.resumed(event_loop);
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: RenderEvent) {
        match event {
            RenderEvent::Initialized(state) => {
                self.initializing = false;
                self.app = Some(App::new(state));
                if let Some(app) = self.app.as_mut() {
                    app.resumed(event_loop);
                    app.state.window().request_redraw();
                }
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(app) = self.app.as_mut() {
            app.about_to_wait(event_loop);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let Some(app) = self.app.as_mut() {
            app.window_event(event_loop, window_id, event);
        }
    }
}
