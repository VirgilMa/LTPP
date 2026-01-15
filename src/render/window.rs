use std::sync::Arc;
use std::time::{Duration, Instant};

use super::state::State;
use cgmath;
use imgui::FontSource;
use imgui_wgpu::RendererConfig;
use imgui_winit_support::WinitPlatform;
use wgpu::TextureView;
use winit::{
    event::{Event, WindowEvent},
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
            // 注释掉FPS打印，保留计算逻辑
            // println!("fps: {}", self.count);
            self.count = 0;
            self.timestamp = Instant::now();
        }
    }
}

struct ImguiState {
    context: imgui::Context,
    platform: WinitPlatform,
    renderer: imgui_wgpu::Renderer,
}

struct App<'a> {
    fps_counter: FpsCounter,
    state: State<'a>,
    imgui: Option<ImguiState>,
    should_exit: bool,
    last_frame_time: std::time::Instant,
}

impl<'a> App<'a> {
    pub fn new(state: State<'a>) -> App<'a> {
        let fps_counter = FpsCounter::new();

        let mut app = Self {
            fps_counter,
            state,
            imgui: None,
            should_exit: false,
            last_frame_time: std::time::Instant::now(),
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

        // 检查是否需要等待下一帧以限制 FPS
        if self.state.max_fps > 0.0 {
            let elapsed = self.last_frame_time.elapsed();
            let target_frame_time = std::time::Duration::from_secs_f64(1.0 / self.state.max_fps);

            if elapsed < target_frame_time {
                // 等待直到达到目标帧时间
                std::thread::sleep(target_frame_time - elapsed);
            }
        }

        // 更新上次帧时间
        self.last_frame_time = std::time::Instant::now();

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

        let imgui = self.imgui.as_mut().unwrap();
        imgui.platform.handle_event::<()>(
            imgui.context.io_mut(),
            self.state.window(),
            &Event::WindowEvent { window_id, event },
        );
    }
}

struct RenderApp<'a>(Option<App<'a>>);

impl RenderApp<'_> {
    fn new() -> Self {
        Self(None)
    }
}

pub async fn render() {
    let event_loop = EventLoop::new().unwrap();
    let mut render_app = RenderApp::new();
    event_loop.run_app(&mut render_app).unwrap();
}

impl winit::application::ApplicationHandler for RenderApp<'_> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        // Only initialize the app once when the application is resumed for the first time
        if self.0.is_none() {
            let window_attributes = winit::window::WindowAttributes::default()
                .with_title("LTPP")
                .with_inner_size(winit::dpi::LogicalSize::new(1920, 1080));

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

            let state = pollster::block_on(State::new(window.clone()));
            self.0 = Some(App::new(state));
        }

        if let Some(app) = self.0.as_mut() {
            app.resumed(event_loop);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(app) = self.0.as_mut() {
            app.about_to_wait(event_loop);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if let Some(app) = self.0.as_mut() {
            app.window_event(event_loop, window_id, event);
        }
    }
}
