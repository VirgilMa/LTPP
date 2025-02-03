use std::f32::consts::PI;

use cgmath::{Quaternion, Rotation, Rotation3};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

#[rustfmt::skip]
const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

pub struct Camera {
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        // 1.
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        // 2.
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);

        // 3.
        OPENGL_TO_WGPU_MATRIX * proj * view
    }
}

// We need this for Rust to store our data correctly for the shaders
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    // We can't use cgmath with bytemuck directly, so we'll have
    // to convert the Matrix4 into a 4x4 f32 array
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}

#[derive(Debug)]
pub struct CameraController {
    speed: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    is_up_pressed: bool,
    is_down_pressed: bool,
    scroll_val: f32,

    is_mouse_right_pressed: bool,
    mouse_speed: f32,
    old_position: Option<PhysicalPosition<f64>>,
    new_position: Option<PhysicalPosition<f64>>,
    window_size: PhysicalSize<u32>,
}

impl CameraController {
    pub fn new(speed: f32, mouse_speed: f32, window_size: &PhysicalSize<u32>) -> Self {
        Self {
            speed,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            is_up_pressed: false,
            is_down_pressed: false,
            is_mouse_right_pressed: false,
            scroll_val: 0.0,
            old_position: None,
            new_position: None,
            mouse_speed,
            window_size: *window_size,
        }
    }

    pub fn process_events(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Right,
                ..
            } => {
                self.is_mouse_right_pressed = *state == ElementState::Pressed;
                true
            }
            WindowEvent::CursorMoved { position, .. } => {
                if self.is_mouse_right_pressed {
                    self.new_position = Some(*position);
                    // println!("mouse position {:?}", position)
                }
                true
            }
            // todo: imgui use wheel
            // WindowEvent::MouseWheel { delta, phase, .. } => {
            //     if *phase == TouchPhase::Moved {
            //         match delta {
            //             MouseScrollDelta::LineDelta(_, vert) => {
            //                 self.scroll_val += vert;
            //             }
            //             _ => {}
            //         }
            //     }
                // println!("mouse wheel: {:?} phase<{:?}>", delta, phase);
            //     true
            // }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state,
                        physical_key,
                        ..
                    },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match physical_key {
                    PhysicalKey::Code(KeyCode::KeyW) | PhysicalKey::Code(KeyCode::ArrowUp) => {
                        self.is_forward_pressed = is_pressed;
                        true
                    }
                    PhysicalKey::Code(KeyCode::KeyA) | PhysicalKey::Code(KeyCode::ArrowLeft) => {
                        self.is_left_pressed = is_pressed;
                        true
                    }
                    PhysicalKey::Code(KeyCode::KeyS) | PhysicalKey::Code(KeyCode::ArrowDown) => {
                        self.is_backward_pressed = is_pressed;
                        true
                    }
                    PhysicalKey::Code(KeyCode::KeyD) | PhysicalKey::Code(KeyCode::ArrowRight) => {
                        self.is_right_pressed = is_pressed;
                        true
                    }
                    PhysicalKey::Code(KeyCode::KeyQ) => {
                        self.is_up_pressed = is_pressed;
                        true
                    }
                    PhysicalKey::Code(KeyCode::KeyE) => {
                        self.is_down_pressed = is_pressed;
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    pub fn update_camera(&mut self, camera: &mut Camera) {
        use cgmath::InnerSpace;
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        // let forward_mag = forward.magnitude();
        let up_norm = camera.up.normalize();

        // Prevents glitching when the camera gets too close to the
        // center of the scene.
        if self.scroll_val != 0.0 {
            camera.eye += forward_norm * self.speed * self.scroll_val;
            camera.target += forward_norm * self.speed * self.scroll_val;
            self.scroll_val = 0.0;
        }
        if self.is_forward_pressed {
            camera.eye += forward_norm * self.speed;
            camera.target += forward_norm * self.speed;
        }
        if self.is_backward_pressed {
            camera.eye -= forward_norm * self.speed;
            camera.target -= forward_norm * self.speed;
        }

        let right = forward_norm.cross(camera.up);

        // Redo radius calc in case the forward/backward is pressed.
        // let forward = camera.target - camera.eye;
        // let forward_mag = forward.magnitude();

        if self.is_right_pressed {
            // Rescale the distance between the target and the eye so
            // that it doesn't change. The eye, therefore, still
            // lies on the circle made by the target and eye.
            let right_move = right * self.speed;
            camera.eye += right_move;
            camera.target += right_move;
            // camera.eye = camera.target - (forward + right * self.speed).normalize() * forward_mag;
        }
        if self.is_left_pressed {
            // camera.eye = camera.target - (forward - right * self.speed).normalize() * forward_mag;
            let right_move = -right * self.speed;
            camera.eye += right_move;
            camera.target += right_move;
        }

        if self.is_up_pressed {
            let up_move = up_norm * self.speed;
            camera.eye += up_move;
            camera.target += up_move;
        }
        if self.is_down_pressed {
            let up_move = -up_norm * self.speed;
            camera.eye += up_move;
            camera.target += up_move;
        }

        // Mouse movement. Width height stands for 360 degrees
        if let Some(new_position_in) = self.new_position {
            if let Some(old_position_in) = self.old_position {
                let move_x = (new_position_in.x - old_position_in.x) as f32
                    / self.window_size.width as f32
                    * PI
                    * 2.0
                    * self.mouse_speed;
                let move_y = (new_position_in.y - old_position_in.y) as f32
                    / self.window_size.height as f32
                    * PI
                    * 2.0
                    * self.mouse_speed;

                let move_x_rad = cgmath::Rad(move_x);
                let move_y_rad = cgmath::Rad(move_y);
                let dir_vec = (camera.target - camera.eye).normalize();

                let rot_quat = Quaternion::from_angle_y(-move_x_rad)
                    * Quaternion::from_axis_angle(right, -move_y_rad);
                let new_dir_vec = rot_quat.rotate_vector(dir_vec);
                // println!(
                //     "rotate dir. old<{:?}> right<{:?}> move_x<{:?}>, move_y<{:?}>, new<{:?}>",
                //     dir_vec, right, move_x, move_y, new_dir_vec
                // );

                camera.target = camera.eye + new_dir_vec;
            }
        }
        self.old_position = self.new_position;
        self.new_position = None;

        // println!("update camera");
    }
}
