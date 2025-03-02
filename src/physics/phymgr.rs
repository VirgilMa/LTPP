// use cgmath;
// use cgmath::{Quaternion, Vector3, Vector4};
type Vector3 = cgmath::Vector3<f32>;
type Vector4 = cgmath::Vector4<f32>;
type Quaternion = cgmath::Quaternion<f32>;
type Matrix4 = cgmath::Matrix4<f32>;
type Decomposed = cgmath::Decomposed<Vector4, Quaternion>;

struct PhyMgr {
    dynamic_objs: Vec<Instance>,
    static_objs: Vec<Instance>,
    gravity: f32,
    air_friction: f32, // acceleration
}

struct Instance {
    transform: Decomposed,
    shape: Shape,
    mass: f32,
    velocity: Vector3,
}

// todo
struct Transform2 {
    position: Vector4,
    rotation: Quaternion,
    scale: Vector4,
}


enum Shape {
    Sphere { radius: f32 },
}

impl PhyMgr {
    pub fn update(&mut self,delta_time: f32) {

        // check collide
        for (instance in self.dynamic_objs) {

        }

        // update all dynamics
    }

    // todo: braod phase and narrow phase
}
