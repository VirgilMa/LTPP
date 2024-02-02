use std::time::{Duration, Instant};

use super::state::State;
use winit::event::ElementState;
use winit::{
    event::{Event, KeyboardInput, StartCause, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
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

pub async fn render() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut state = State::new(window).await;
    let mut fps_counter = FpsCounter::new();

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::MainEventsCleared => {
                state.window().request_redraw();
            }
            Event::RedrawEventsCleared => (),
            Event::RedrawRequested(window_id) => {
                if window_id == state.window().id() {
                    state.update();
                    match state.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                        Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                        Err(e) => eprintln!("{:?}", e),
                    };
                    fps_counter.count();
                }
            }
            Event::NewEvents(StartCause::Poll) => (),
            Event::WindowEvent { window_id, event } => {
                if window_id == state.window().id() {
                    // println!("[{:?}] receive event WindowEvent {:?}, {:?}", chrono::Local::now(), window_id, event);

                    state.input(&event);

                    match event {
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => {
                            println!("received {:?}. EXIT", event);
                            *control_flow = ControlFlow::Exit
                        }
                        WindowEvent::Resized(physical_size) => {
                            state.resize(physical_size);
                            // println!("WindowEvent::Resized {:?}", physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            state.resize(*new_inner_size);
                            // println!("WindowEvent::ScaleFactorChanged {:?}", new_inner_size);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        };
    });
}
