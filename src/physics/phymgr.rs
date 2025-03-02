type Vector3 = cgmath::Vector3<f32>;
type Vector4 = cgmath::Vector4<f32>;
type Quaternion = cgmath::Quaternion<f32>;
type Matrix4 = cgmath::Matrix4<f32>;
type Decomposed = cgmath::Decomposed<Vector4, Quaternion>;

struct PhyMgr {
    dynamic_objs: Vec<Instance>,
    static_objs: Vec<Instance>,
    gravity: Vector3,
    air_friction: f32, // acceleration
}

struct Instance {
    transform: Decomposed,
    shape: Shape,
    mass: f32,
    velocity: Vector3,
    accel: Vector3,
}

// todo
// struct Transform2 {
//     position: Vector4,
//     rotation: Quaternion,
//     scale: Vector4,
// }

#[derive(Debug)]
enum Shape {
    Sphere { radius: f32 },
}

impl PhyMgr {
    pub fn create_instance(
        &mut self,
        transform: Decomposed,
        shape: Shape,
        mass: f32,
        is_static: bool,
    ) {
        let instance = Instance {
            transform,
            shape,
            mass,
            velocity: Vector3::new(0.0, 0.0, 0.0),
            accel: Vector3::new(0.0, 0.0, 0.0),
        };

        let insert_arr = if is_static {
            &mut self.static_objs
        } else {
            &mut self.dynamic_objs
        };

        insert_arr.push(instance);
    }

    pub fn update(&mut self, delta_time: f32) {
        let dynamic_objs_len = self.dynamic_objs.len();
        let static_objs_len = self.static_objs.len();

        // calculate accelerate
        for i in 0..dynamic_objs_len {
            let dyo = &mut self.dynamic_objs[i];

            // calculate accelerate
            dyo.accel = self.gravity;

            // update velocity
            dyo.velocity += dyo.accel * delta_time;

            // move step
            dyo.transform.disp += (dyo.velocity * delta_time).extend(0.0);
        }

        // check collision, cal the collision normal, squeeze out and reverse velocity by the collision
        for i in 0..dynamic_objs_len {
            for j in 0..dynamic_objs_len{

                
            }
            for j in 0..static_objs_len {

                
            }
        }

        // update all dynamics
    }

    // todo: braod phase and narrow phase
}
