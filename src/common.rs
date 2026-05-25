use cgmath::*;

/// 通用变换结构体，包含位置、旋转和缩放
#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub translation: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub scale: Vector3<f32>,
}

impl Transform {
    /// 创建默认变换（位于原点，无旋转，单位缩放）
    pub fn identity() -> Self {
        Self {
            translation: Vector3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0), // 单位四元数 w=1, x=y=z=0
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }

    /// 创建指定位置的变换
    pub fn translation(x: f32, y: f32, z: f32) -> Self {
        Self {
            translation: Vector3::new(x, y, z),
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }

    /// 创建指定旋转的变换
    pub fn rotation(quat: Quaternion<f32>) -> Self {
        Self {
            translation: Vector3::new(0.0, 0.0, 0.0),
            rotation: quat.normalize(),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }

    /// 创建指定缩放的变换
    pub fn scale(x: f32, y: f32, z: f32) -> Self {
        Self {
            translation: Vector3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::new(1.0, 0.0, 0.0, 0.0),
            scale: Vector3::new(x, y, z),
        }
    }

    /// 转换为模型矩阵
    pub fn to_matrix(&self) -> Matrix4<f32> {
        // 按照 T * R * S 的顺序应用变换
        Matrix4::from_translation(self.translation)
            * Matrix4::from(self.rotation)
            * Matrix4::from_nonuniform_scale(self.scale.x, self.scale.y, self.scale.z)
    }

    /// 应用另一个变换
    pub fn concat(&self, other: &Transform) -> Self {
        let new_matrix = self.to_matrix() * other.to_matrix();

        // 从变换矩阵提取平移
        let translation = Vector3::new(new_matrix[3][0], new_matrix[3][1], new_matrix[3][2]);

        // 从变换矩阵提取旋转（仅当没有剪切时有效）
        let scale_x = new_matrix[0].truncate().magnitude();
        let scale_y = new_matrix[1].truncate().magnitude();
        let scale_z = new_matrix[2].truncate().magnitude();

        let scale = Vector3::new(scale_x, scale_y, scale_z);

        // 标准化旋转矩阵部分
        let rot_matrix = Matrix3::new(
            new_matrix[0][0] / scale.x,
            new_matrix[0][1] / scale.x,
            new_matrix[0][2] / scale.x,
            new_matrix[1][0] / scale.y,
            new_matrix[1][1] / scale.y,
            new_matrix[1][2] / scale.y,
            new_matrix[2][0] / scale.z,
            new_matrix[2][1] / scale.z,
            new_matrix[2][2] / scale.z,
        );

        let rotation = Quaternion::from(rot_matrix);

        Self {
            translation,
            rotation,
            scale,
        }
    }

    /// 变换一个点
    pub fn transform_point(&self, point: Vector3<f32>) -> Vector3<f32> {
        let transformed = self.to_matrix() * point.extend(1.0);
        Vector3::new(transformed.x, transformed.y, transformed.z)
    }

    /// 变换一个向量（忽略平移）
    pub fn transform_vector(&self, vector: Vector3<f32>) -> Vector3<f32> {
        let rotation_matrix = Matrix3::from(self.rotation);
        let scaled_vector = Vector3::new(
            vector.x * self.scale.x,
            vector.y * self.scale.y,
            vector.z * self.scale.z,
        );
        rotation_matrix * scaled_vector
    }
}

/// 3x3 矩阵的便捷别名
pub type Mat3 = Matrix3<f32>;

/// 4x4 矩阵的便捷别名
pub type Mat4 = Matrix4<f32>;

/// 用于物理模拟的时间步长常量
pub const PHYSICS_TIMESTEP: f32 = 1.0 / 60.0; // 60 FPS

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_identity() {
        let t = Transform::identity();
        assert_eq!(t.translation, Vector3::new(0.0, 0.0, 0.0));
        assert_eq!(t.rotation.v, Vector3::new(0.0, 0.0, 0.0)); // 确保是单位四元数
        assert_eq!(t.scale, Vector3::new(1.0, 1.0, 1.0));
    }

    #[test]
    fn test_transform_to_matrix() {
        let t = Transform::translation(1.0, 2.0, 3.0);
        let matrix = t.to_matrix();

        assert_eq!(matrix[3][0], 1.0);
        assert_eq!(matrix[3][1], 2.0);
        assert_eq!(matrix[3][2], 3.0);
    }

    #[test]
    fn test_transform_concat() {
        let t1 = Transform::translation(1.0, 0.0, 0.0);
        let t2 = Transform::translation(0.0, 1.0, 0.0);
        let result = t1.concat(&t2);

        assert!((result.translation.x - 1.0).abs() < 1e-5);
        assert!((result.translation.y - 1.0).abs() < 1e-5);
        assert!((result.translation.z - 0.0).abs() < 1e-5);
    }
}
