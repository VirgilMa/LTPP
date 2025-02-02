struct Vector3 {
    x: f32,
    y: f32,
    z: f32,
}

impl Vector3 {
    fn add(&self, other: &Vector3) -> Vector3 {
        Vector3 {
            x: (self.x + other.x),
            y: (self.y + other.y),
            z: (self.z + other.z),
        }
    }
    fn sub(&self, other: &Vector3) -> Vector3 {
        Vector3 {
            x: (self.x - other.x),
            y: (self.y - other.y),
            z: (self.z - other.z),
        }
    }
    fn len_sqr(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }
    // fn len(&self) -> f32 {
    // (self.len_sqr()) ** 0.5
    // }
}

type Point3 = Vector3;

// 左手坐标系，x向右，y上，z前
struct PhysicalObject {
    pos: Point3, // m
    mess: f32,   // kg
}

struct Sphere {
    radius: f32,             // m
    pos: Point3,             // m
    mess: f32,               // kg
    velocity_vec: Vector3,   // m/s
    accelerate_vec: Vector3, // m/s^2
}

pub struct PhysicalWorld {
    objs: Vec<Sphere>,
    gravity: f32, // kg/m^2
}

impl PhysicalWorld {
    pub fn new(gravity: f32) -> PhysicalWorld {
        Self {
            gravity,
            objs: vec![],
        }
    }

    pub fn update(&mut self, delta_time: f32) {
        let obj_len = self.objs.len();


        for i in 0..obj_len {
            let s1 = &(self.objs[i]);
            for j in (i + 1)..obj_len {
                let s2 = &(self.objs[j]);
                let radius_sum = s1.radius + s2.radius;
                let radius_sum_sqr = radius_sum * radius_sum;
                let distance_sqr = s2.pos.sub(&s1.pos).len_sqr();

                if distance_sqr < radius_sum_sqr {
                    // 完全弹性碰撞，动量守恒，动能守恒，恢复系数e=1
                }
            }
        }
    }

    pub fn insertObject(&mut self, s: Sphere) {
        self.objs.push(s)
    }
}
