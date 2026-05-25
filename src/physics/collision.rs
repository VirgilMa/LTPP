use crate::common::Transform; // 明确导入Transform以避免歧义
use crate::physics::shape::{Cylinder, PhysicsBody, Plane, Shape};
use cgmath::*;

// 碰撞信息
#[derive(Clone, Copy)]
pub struct CollisionInfo {
    pub contact_point: Vector3<f32>, // 接触点
    pub penetration_depth: f32,      // 穿透深度
    pub normal: Vector3<f32>,        // 指向第一个物体的法向量
}

/// 检测两个圆柱体之间的碰撞
pub fn collide_cylinder_cylinder(cyl1: &Cylinder, cyl2: &Cylinder) -> Option<CollisionInfo> {
    // 将圆柱体轴向量标准化
    let axis1 = cyl1.axis.normalize();
    let axis2 = cyl2.axis.normalize();

    // 计算两圆柱体底面中心的向量
    let center_diff = cyl2.center - cyl1.center;

    // 计算两轴线之间的最短距离
    let cross_product = axis1.cross(axis2);
    let denom = cross_product.magnitude2();

    if denom < 1e-6 {
        // 两轴平行的情况
        let dist_from_axis = (center_diff - axis1 * center_diff.dot(axis1)).magnitude();
        let max_radius = cyl1.radius + cyl2.radius;

        if dist_from_axis <= max_radius {
            // 计算沿轴向的重叠
            let proj1 = center_diff.dot(axis1);
            let half_height1 = cyl1.height / 2.0;
            let half_height2 = cyl2.height / 2.0;

            // 检查高度方向是否有重叠
            if proj1.abs() <= half_height1 + half_height2 {
                // 计算接触点（取两轴线上最近点的中点）
                let contact_point = cyl1.center
                    + axis1 * proj1
                    + (axis1.cross(Vector3::unit_x())).normalize() * (max_radius - dist_from_axis)
                        / 2.0;

                // 法向量指向第二个圆柱体
                let normal = if dist_from_axis > 0.0 {
                    (cyl2.center - cyl1.center).normalize()
                } else {
                    axis1
                };

                return Some(CollisionInfo {
                    contact_point,
                    penetration_depth: max_radius - dist_from_axis,
                    normal,
                });
            }
        }
    } else {
        // 两轴不平行的情况 - 计算两直线间的最短距离
        let r12 = cyl2.center - cyl1.center;
        let a = axis1.magnitude2(); // |u|^2
        let b = axis1.dot(axis2); // u·v
        let c = axis2.magnitude2(); // |v|^2
        let d = axis1.dot(r12); // u·w
        let e = axis2.dot(r12); // v·w

        let denominator = a * c - b * b;
        if denominator < 1e-6 {
            // 接近平行的情况
            return collide_cylinder_cylinder_parallel(cyl1, cyl2);
        }

        let t_u = (b * e - c * d) / denominator;
        let t_v = (a * e - b * d) / denominator;

        let point_on_axis1 = cyl1.center + axis1 * t_u;
        let point_on_axis2 = cyl2.center + axis2 * t_v;
        let shortest_dist_vec = point_on_axis2 - point_on_axis1;
        let dist = shortest_dist_vec.magnitude();

        let max_radius = cyl1.radius + cyl2.radius;

        if dist <= max_radius {
            // 检查沿各自轴的范围是否重叠
            let half_h1 = cyl1.height / 2.0;
            let half_h2 = cyl2.height / 2.0;

            // 检查第一个圆柱体轴上的投影
            let proj1 = (point_on_axis1 - cyl1.center).dot(axis1);
            if proj1.abs() > half_h1 {
                // 最近点超出圆柱体范围，需要调整
                return None; // 简化处理，实际应找到边界点
            }

            // 检查第二个圆柱体轴上的投影
            let proj2 = (point_on_axis2 - cyl2.center).dot(axis2);
            if proj2.abs() > half_h2 {
                // 最近点超出圆柱体范围，需要调整
                return None; // 简化处理，实际应找到边界点
            }

            // 计算接触点（两最近点的中点）
            let contact_point = (point_on_axis1 + point_on_axis2) / 2.0;

            // 法向量从第一个圆柱体指向第二个
            let normal = if dist > 0.0 {
                shortest_dist_vec.normalize()
            } else {
                // 如果距离为0，使用轴向量之一
                axis1
            };

            return Some(CollisionInfo {
                contact_point,
                penetration_depth: max_radius - dist,
                normal,
            });
        }
    }

    None
}

/// 辅助函数：处理平行轴情况下的圆柱体碰撞
fn collide_cylinder_cylinder_parallel(cyl1: &Cylinder, cyl2: &Cylinder) -> Option<CollisionInfo> {
    let axis1 = cyl1.axis.normalize();
    let center_diff = cyl2.center - cyl1.center;

    // 计算两轴线之间的距离
    let dist_from_axis = (center_diff - axis1 * center_diff.dot(axis1)).magnitude();
    let max_radius = cyl1.radius + cyl2.radius;

    if dist_from_axis <= max_radius {
        // 计算沿轴向的重叠
        let proj = center_diff.dot(axis1);
        let half_height1 = cyl1.height / 2.0;
        let half_height2 = cyl2.height / 2.0;

        // 检查高度方向是否有重叠
        let h1_start = -half_height1;
        let h1_end = half_height1;
        let h2_start = proj - half_height2;
        let h2_end = proj + half_height2;

        let overlap_start = h1_start.max(h2_start);
        let overlap_end = h1_end.min(h2_end);

        if overlap_start <= overlap_end {
            // 有重叠
            let contact_point = cyl1.center
                + axis1 * (overlap_start + overlap_end) / 2.0
                + (axis1.cross(Vector3::unit_x())).normalize() * (max_radius - dist_from_axis)
                    / 2.0;

            let normal = if dist_from_axis > 0.0 {
                (cyl2.center - cyl1.center - axis1 * proj).normalize()
            } else {
                // 如果轴线重合，任选垂直方向
                axis1.cross(Vector3::unit_x()).normalize()
            };

            return Some(CollisionInfo {
                contact_point,
                penetration_depth: max_radius - dist_from_axis,
                normal,
            });
        }
    }

    None
}

/// 检测圆柱体与平面之间的碰撞
pub fn collide_cylinder_plane(cylinder: &Cylinder, plane: &Plane) -> Option<CollisionInfo> {
    // 标准化圆柱体轴向量
    let axis_normalized = cylinder.axis.normalize();

    // 计算圆柱体中心点到平面的距离
    let dist_to_center = cylinder.center.dot(plane.normal) - plane.distance;

    // 计算圆柱体轴线在平面法线方向上的投影
    let axis_projection = axis_normalized.dot(plane.normal).abs();
    let half_height_proj = (cylinder.height / 2.0) * axis_projection;

    // 计算最大可能距离（考虑半径）
    let max_dist = half_height_proj + cylinder.radius;

    // 检查是否碰撞
    if dist_to_center.abs() <= max_dist {
        // 确定碰撞类型
        if half_height_proj >= dist_to_center.abs() {
            // 侧面碰撞：圆柱体侧面与平面相交
            // 计算圆柱体轴线到平面的最近点
            let t = (plane.distance - cylinder.center.dot(plane.normal))
                / axis_normalized.dot(plane.normal);
            let closest_point_on_axis = cylinder.center + axis_normalized * t;

            // 计算圆柱体表面上距离平面最近的点
            let radial_direction =
                (plane.normal - axis_normalized * plane.normal.dot(axis_normalized)).normalize();
            let surface_point = closest_point_on_axis + radial_direction * cylinder.radius;

            // 计算穿透深度
            let dist_to_surface = surface_point.dot(plane.normal) - plane.distance;
            let penetration_depth = cylinder.radius - dist_to_surface.abs();

            let contact_point = surface_point - plane.normal * dist_to_surface;
            let normal = if dist_to_surface > 0.0 {
                plane.normal
            } else {
                -plane.normal
            };

            return Some(CollisionInfo {
                contact_point,
                penetration_depth: penetration_depth.abs(),
                normal,
            });
        } else {
            // 顶部或底部碰撞
            // 计算圆柱体顶部和底部中心到平面的距离
            let top_center = cylinder.center + axis_normalized * (cylinder.height / 2.0);
            let bottom_center = cylinder.center - axis_normalized * (cylinder.height / 2.0);

            let dist_to_top = top_center.dot(plane.normal) - plane.distance;
            let dist_to_bottom = bottom_center.dot(plane.normal) - plane.distance;

            // 检查顶部是否碰撞
            if dist_to_top.abs() <= cylinder.radius {
                // 顶部碰撞
                let contact_point = top_center - plane.normal * dist_to_top;

                let normal = if dist_to_top > 0.0 {
                    plane.normal
                } else {
                    -plane.normal
                };

                return Some(CollisionInfo {
                    contact_point,
                    penetration_depth: cylinder.radius - dist_to_top.abs(),
                    normal,
                });
            }
            // 检查底部是否碰撞
            else if dist_to_bottom.abs() <= cylinder.radius {
                // 底部碰撞
                let contact_point = bottom_center - plane.normal * dist_to_bottom;

                let normal = if dist_to_bottom > 0.0 {
                    plane.normal
                } else {
                    -plane.normal
                };

                return Some(CollisionInfo {
                    contact_point,
                    penetration_depth: cylinder.radius - dist_to_bottom.abs(),
                    normal,
                });
            }
        }
    }

    None
}

/// 检测两个物理体之间的碰撞
pub fn collide_bodies(body1: &PhysicsBody, body2: &PhysicsBody) -> Option<CollisionInfo> {
    match (&body1.shape, &body2.shape) {
        (Shape::Cylinder(cyl1), Shape::Cylinder(cyl2)) => {
            // 应用变换后检测碰撞
            let transformed_cyl1 = transform_cylinder(cyl1, &body1.transform);
            let transformed_cyl2 = transform_cylinder(cyl2, &body2.transform);
            collide_cylinder_cylinder(&transformed_cyl1, &transformed_cyl2)
        }
        (Shape::Cylinder(cylinder), Shape::Plane(plane)) => collide_cylinder_plane(cylinder, plane),
        (Shape::Plane(plane), Shape::Cylinder(cylinder)) => collide_cylinder_plane(cylinder, plane),
        _ => {
            // 暂时只支持圆柱体之间的碰撞
            None
        }
    }
}

/// 应用变换到圆柱体
fn transform_cylinder(cylinder: &Cylinder, transform: &Transform) -> Cylinder {
    let new_center = transform.transform_point(cylinder.center);
    let new_axis = transform.transform_vector(cylinder.axis);

    Cylinder {
        center: new_center,
        axis: new_axis.normalize(),
        radius: cylinder.radius * transform.scale.x, // 简化处理，假设各向同性缩放
        height: cylinder.height * transform.scale.y,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cylinder_cylinder_collision() {
        let cyl1 = Cylinder {
            center: Vector3::new(0.0, 0.0, 0.0),
            axis: Vector3::new(0.0, 1.0, 0.0),
            radius: 1.0,
            height: 2.0,
        };

        let cyl2 = Cylinder {
            center: Vector3::new(1.5, 0.0, 0.0),
            axis: Vector3::new(0.0, 1.0, 0.0),
            radius: 1.0,
            height: 2.0,
        };

        // 两个圆柱体重叠，应该检测到碰撞
        let collision = collide_cylinder_cylinder(&cyl1, &cyl2);
        assert!(collision.is_some());

        if let Some(info) = collision {
            assert!(info.penetration_depth > 0.0);
        }
    }

    #[test]
    fn test_cylinder_plane_collision() {
        let cylinder = Cylinder {
            center: Vector3::new(0.0, 0.0, 0.0), // 将圆柱体放置在原点
            axis: Vector3::new(0.0, 1.0, 0.0),   // Y轴方向
            radius: 1.0,
            height: 2.0, // 高度为2，所以从Y=-1到Y=1
        };

        let plane = Plane {
            normal: Vector3::new(0.0, 1.0, 0.0), // Y轴正方向的平面
            distance: 0.5,                       // 位于Y=0.5的平面
        };

        // 圆柱体跨越平面（从Y=-1到Y=1），平面在Y=0.5，所以应该碰撞
        let collision = collide_cylinder_plane(&cylinder, &plane);
        assert!(
            collision.is_some(),
            "Expected collision between cylinder and plane"
        );
    }
}
