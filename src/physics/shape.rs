use crate::common::Transform; // 明确导入Transform以避免歧义
use cgmath::*;

#[derive(Clone, Copy)]
pub enum PhysicsState {
    Static,    // 静态物体（不会移动）
    Dynamic,   // 动态物体（受力影响）
    Kinematic, // 运动学物体（手动控制位置）
}

// 基础形状定义
#[derive(Clone, Copy)]
pub struct Cylinder {
    pub center: Vector3<f32>, // 底面中心点
    pub axis: Vector3<f32>,   // 高度方向（单位向量）
    pub radius: f32,
    pub height: f32,
}

#[derive(Clone, Copy)]
pub struct Plane {
    pub normal: Vector3<f32>, // 单位法向量
    pub distance: f32,        // 到原点的距离（沿法线方向）
}

// 形状枚举
#[derive(Clone)]
pub enum Shape {
    Cylinder(Cylinder),
    Plane(Plane),
    // 后续可添加其他形状
}

// 物理体
#[derive(Clone)]
pub struct PhysicsBody {
    pub shape: Shape,
    pub transform: Transform,
    pub velocity: Vector3<f32>,
    pub angular_velocity: Vector3<f32>,
    pub mass: f32,     // dynmaic 质量必须大于0，static质量为无限大
    pub inv_mass: f32, // 质量倒数（0 表示无穷大，即静态）
    pub inertia_tensor: Matrix3<f32>,
    pub state: PhysicsState,
    pub friction: f32,
    pub restitution: f32, // 弹性系数
}

impl PhysicsBody {
    pub fn new_dynamic(shape: Shape, transform: Transform, mass: f32) -> Self {
        assert!(mass > 0.0);

        let inv_mass = if mass > 0.0 { 1.0 / mass } else { 0.0 };

        Self {
            shape,
            transform,
            velocity: Vector3::new(0.0, 0.0, 0.0),
            angular_velocity: Vector3::new(0.0, 0.0, 0.0),
            mass,
            inv_mass,
            inertia_tensor: Matrix3::from_diagonal(Vector3::new(1.0, 1.0, 1.0)), // 单位矩阵
            state: PhysicsState::Dynamic,
            friction: 0.5,
            restitution: 0.2,
        }
    }

    pub fn new_static(shape: Shape, transform: Transform) -> Self {
        Self {
            shape,
            transform,
            velocity: Vector3::new(0.0, 0.0, 0.0),
            angular_velocity: Vector3::new(0.0, 0.0, 0.0),
            mass: f32::INFINITY,
            inv_mass: 0.0,
            inertia_tensor: Matrix3::zero(), // 零矩阵表示无限大惯性
            state: PhysicsState::Static,
            friction: 0.5,
            restitution: 0.2,
        }
    }

    // 获取物体的位置（从变换中获取）
    pub fn position(&self) -> Vector3<f32> {
        self.transform.translation
    }

    // 设置物体的位置
    pub fn set_position(&mut self, pos: Vector3<f32>) {
        self.transform.translation = pos;
    }
}
