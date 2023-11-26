use winit::{
    event::{Event, WindowEvent, KeyboardInput, VirtualKeyCode, StartCause},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use winit::event::ElementState;
use super::state::State;

pub async fn render() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let mut state = State::new(window).await;

    event_loop.run(move |event, _, control_flow| {
        // *control_flow = ControlFlow::Wait;

        match event {
            Event::MainEventsCleared => (),
            Event::RedrawEventsCleared => (),
            Event::RedrawRequested(window_id) => if window_id == state.window().id() {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    Err(e) => eprintln!("{:?}", e)
                }
            },
            Event::NewEvents(StartCause::Poll) => (),
            Event::WindowEvent {
                window_id,
                event
            } => if window_id == state.window().id() {
                // println!("[{:?}] receive event WindowEvent {:?}, {:?}", chrono::Local::now(), window_id, event);

                match event {
                    WindowEvent::KeyboardInput {
                        input: KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Q) | Some(VirtualKeyCode::Escape),
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
                    WindowEvent::CursorMoved { device_id, position, modifiers } => {
                        state.input(&event);

                        state.update();

                        match state.render() {
                            Ok(_) => {}
                            Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                            Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                            Err(e) => eprintln!("{:?}", e)
                        }
                    }
                    _ => {}
                }
            }
            _ => {
                // println!("[{:?}] receive event {:?}", chrono::Local::now(), event);
            }
        };
    });
}