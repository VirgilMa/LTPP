use super::{model, texture};
use cfg_if::cfg_if;
use image::Rgba;
use std::io::{BufReader, Cursor};
use wgpu::util::DeviceExt;

#[cfg(target_arch = "wasm32")]
fn format_url(file_name: &str) -> reqwest::Url {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let mut origin = location.origin().unwrap();
    if !origin.ends_with("res") {
        origin = format!("{}/res", origin);
    }
    let base = reqwest::Url::parse(&format!("{}/", origin,)).unwrap();
    base.join(file_name).unwrap()
}

pub async fn load_string(file_name: &str) -> anyhow::Result<String> {
    cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            let url = format_url(file_name);
            let txt = reqwest::get(url)
                .await?
                .text()
                .await?;
        } else {
            let path = std::path::Path::new(env!("OUT_DIR"))
                .join("res")
                .join(file_name);
            let txt = std::fs::read_to_string(path)?;
        }
    }
    Ok(txt)
}

pub async fn load_binary(file_name: &str) -> anyhow::Result<Vec<u8>> {
    cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            let url = format_url(file_name);
            let data = reqwest::get(url)
                .await?
                .bytes()
                .await?
                .to_vec();
        } else {
            let path = std::path::Path::new(env!("OUT_DIR"))
                .join("res")
                .join(file_name);
            let data = std::fs::read(path)?;
        }
    }

    Ok(data)
}

pub async fn load_texture(
    file_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> anyhow::Result<texture::Texture> {
    let data = load_binary(file_name).await?;
    texture::Texture::from_bytes(device, queue, &data, file_name)
}

pub async fn load_model(
    file_name: &str,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) -> anyhow::Result<model::Model> {
    let obj_text = load_string(file_name).await?;
    let obj_cursor = Cursor::new(obj_text);
    let mut obj_reader = BufReader::new(obj_cursor);

    let (models, obj_materials) = tobj::load_obj_buf_async(
        &mut obj_reader,
        &tobj::LoadOptions {
            triangulate: true,
            single_index: true,
            ..Default::default()
        },
        |p| async move {
            let mat_text = load_string(&p).await.unwrap();
            tobj::load_mtl_buf(&mut BufReader::new(Cursor::new(mat_text)))
        },
    )
    .await?;

    let mut materials = Vec::new();
    for m in obj_materials? {
        let diffuse_texture = load_texture(&m.diffuse_texture, device, queue).await?;
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
                },
            ],
            label: None,
        });

        materials.push(model::Material {
            name: m.name,
            diffuse_texture,
            bind_group,
        })
    }

    let meshes = models
        .into_iter()
        .map(|m| {
            let vertices = (0..m.mesh.positions.len() / 3)
                .map(|i| model::ModelVertex {
                    position: [
                        m.mesh.positions[i * 3],
                        m.mesh.positions[i * 3 + 1],
                        m.mesh.positions[i * 3 + 2],
                    ],
                    tex_coords: [m.mesh.texcoords[i * 2], m.mesh.texcoords[i * 2 + 1]],
                    normal: [
                        m.mesh.normals[i * 3],
                        m.mesh.normals[i * 3 + 1],
                        m.mesh.normals[i * 3 + 2],
                    ],
                })
                .collect::<Vec<_>>();

            let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Vertex Buffer", file_name)),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Index Buffer", file_name)),
                contents: bytemuck::cast_slice(&m.mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

            model::Mesh {
                name: file_name.to_string(),
                vertex_buffer,
                index_buffer,
                num_elements: m.mesh.indices.len() as u32,
                material: m.mesh.material_id.unwrap_or(0),
            }
        })
        .collect::<Vec<_>>();

    Ok(model::Model { meshes, materials })
}

// 在你的资源加载模块（例如 resource.rs）中添加以下代码

// 生成球体模型的函数
pub async fn generate_sphere_model(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    radius: f32,
    sectors: u32,
    stacks: u32,
    color: Option<image::Rgba<u8>>,
) -> anyhow::Result<model::Model> {
    // 生成球体顶点和索引数据
    let (vertices, indices) = generate_sphere(radius, sectors, stacks);

    let default_color = if let Some(c) = color {
        c
    } else {
        Rgba([255, 255, 255, 255]) // 默认白色
    };

    let default_texture = texture::Texture::create_color_texture(
        device,
        queue,
        Some("default_sphere_texture"),
        800,
        600,
        default_color,
    )?;

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&default_texture.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&default_texture.sampler),
            },
        ],
        label: None,
    });

    let materials = vec![model::Material {
        name: "Sphere_Material".to_string(),
        diffuse_texture: default_texture,
        bind_group,
    }];

    // 创建顶点缓冲区
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Sphere Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    // 创建索引缓冲区
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Sphere Index Buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    // 创建网格
    let meshes = vec![model::Mesh {
        name: "Sphere".to_string(),
        vertex_buffer,
        index_buffer,
        num_elements: indices.len() as u32,
        material: 0, // 使用第一个材质
    }];

    Ok(model::Model { meshes, materials })
}

// 球体生成核心逻辑
fn generate_sphere(radius: f32, sectors: u32, stacks: u32) -> (Vec<model::ModelVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let pi = std::f32::consts::PI;

    // 生成顶点
    for i in 0..=stacks {
        let phi = pi * i as f32 / stacks as f32; // 纬度 0~π
        let y = radius * phi.cos();
        let xy = radius * phi.sin();

        for j in 0..=sectors {
            let theta = 2.0 * pi * j as f32 / sectors as f32; // 经度 0~2π
            let x = xy * theta.cos();
            let z = xy * theta.sin();

            // 纹理坐标
            let s = theta / (2.0 * pi);
            let t = phi / pi;

            // 法线计算（归一化位置）
            let normal = [x / radius, y / radius, z / radius];

            vertices.push(model::ModelVertex {
                position: [x, y, z],
                tex_coords: [s, t],
                normal,
            });
        }
    }

    // 生成索引
    for i in 0..stacks {
        for j in 0..sectors {
            let first = i * (sectors + 1) + j;
            let second = first + sectors + 1;

            indices.push(first);
            indices.push(first + 1);
            indices.push(second);

            indices.push(second);
            indices.push(first + 1);
            indices.push(second + 1);
        }
    }

    (vertices, indices)
}

// 生成圆柱体模型的函数
pub fn generate_cylinder_model(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    radius: f32,
    height: f32,
    sectors: u32,
    stacks: u32,
    color: Option<image::Rgba<u8>>,
) -> anyhow::Result<model::Model> {
    // 生成圆柱体顶点和索引数据
    let (vertices, indices) = generate_cylinder(radius, height, sectors, stacks);

    let default_color = if let Some(c) = color {
        c
    } else {
        Rgba([255, 255, 255, 255]) // 默认白色
    };

    let default_texture = texture::Texture::create_color_texture(
        device,
        queue,
        Some("default_cylinder_texture"),
        800,
        600,
        default_color,
    )?;

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&default_texture.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&default_texture.sampler),
            },
        ],
        label: None,
    });

    let materials = vec![model::Material {
        name: "Cylinder_Material".to_string(),
        diffuse_texture: default_texture,
        bind_group,
    }];

    // 创建顶点缓冲区
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Cylinder Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });

    // 创建索引缓冲区
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Cylinder Index Buffer"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    // 创建网格
    let meshes = vec![model::Mesh {
        name: "Cylinder".to_string(),
        vertex_buffer,
        index_buffer,
        num_elements: indices.len() as u32,
        material: 0, // 使用第一个材质
    }];

    Ok(model::Model { meshes, materials })
}

// 生成专门用于边缘渲染的圆柱体模型
pub fn generate_cylinder_edge_model(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
    radius: f32,
    height: f32,
    sectors: u32,
    stacks: u32,
    color: Option<image::Rgba<u8>>,
) -> anyhow::Result<model::Model> {
    // 生成圆柱体顶点和边缘索引数据
    let (vertices, _) = generate_cylinder(radius, height, sectors, stacks);
    let edge_indices = generate_cylinder_edge_indices(sectors, stacks);

    let default_color = if let Some(c) = color {
        c
    } else {
        Rgba([255, 255, 255, 255]) // 默认白色
    };

    let default_texture = texture::Texture::create_color_texture(
        device,
        queue,
        Some("default_cylinder_edge_texture"),
        800,
        600,
        default_color,
    )?;

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&default_texture.view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&default_texture.sampler),
            },
        ],
        label: None,
    });

    let materials = vec![model::Material {
        name: "Cylinder_Edge_Material".to_string(),
        diffuse_texture: default_texture,
        bind_group,
    }];

    // 创建顶点缓冲区
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Cylinder Edge Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
    });

    // 创建边缘索引缓冲区
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Cylinder Edge Index Buffer"),
        contents: bytemuck::cast_slice(&edge_indices),
        usage: wgpu::BufferUsages::INDEX,
    });

    // 创建网格
    let meshes = vec![model::Mesh {
        name: "Cylinder_Edge".to_string(),
        vertex_buffer,
        index_buffer,
        num_elements: edge_indices.len() as u32,
        material: 0, // 使用第一个材质
    }];

    Ok(model::Model { meshes, materials })
}

// 圆柱体生成核心逻辑
fn generate_cylinder(radius: f32, height: f32, sectors: u32, stacks: u32) -> (Vec<model::ModelVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let pi = std::f32::consts::PI;

    // 生成圆柱侧面的顶点
    for i in 0..=stacks {
        let y = -height / 2.0 + (i as f32 * height / stacks as f32); // 从-bottom到+top

        for j in 0..=sectors {
            let theta = 2.0 * pi * j as f32 / sectors as f32; // 角度 0~2π
            let x = radius * theta.cos();
            let z = radius * theta.sin();

            // 纹理坐标
            let s = j as f32 / sectors as f32; // 水平纹理坐标
            let t = i as f32 / stacks as f32;  // 垂直纹理坐标

            // 法线计算（侧面法线指向外侧）
            let normal = [x / radius, 0.0, z / radius]; // 侧面法线在XZ平面

            vertices.push(model::ModelVertex {
                position: [x, y, z],
                tex_coords: [s, t],
                normal,
            });
        }
    }

    // 记录侧面顶点数量
    let _side_vertices_count = vertices.len();

    // 生成圆柱上下底面的顶点
    // 下底面中心
    let bottom_center_idx = vertices.len() as u32;
    vertices.push(model::ModelVertex {
        position: [0.0, -height / 2.0, 0.0],
        tex_coords: [0.5, 0.5],
        normal: [0.0, -1.0, 0.0], // 向下
    });

    // 上底面中心
    let top_center_idx = vertices.len() as u32;
    vertices.push(model::ModelVertex {
        position: [0.0, height / 2.0, 0.0],
        tex_coords: [0.5, 0.5],
        normal: [0.0, 1.0, 0.0], // 向上
    });

    // 添加底面和顶面的边缘顶点
    for j in 0..sectors {
        let theta = 2.0 * pi * j as f32 / sectors as f32;
        let x = radius * theta.cos();
        let z = radius * theta.sin();

        // 下底面边缘 (需要独立的顶点，因为法线不同)
        vertices.push(model::ModelVertex {
            position: [x, -height / 2.0, z],
            tex_coords: [0.5 + 0.5 * theta.cos(), 0.5 + 0.5 * theta.sin()],
            normal: [0.0, -1.0, 0.0], // 向下
        });

        // 上底面边缘 (需要独立的顶点，因为法线不同)
        vertices.push(model::ModelVertex {
            position: [x, height / 2.0, z],
            tex_coords: [0.5 + 0.5 * theta.cos(), 0.5 + 0.5 * theta.sin()],
            normal: [0.0, 1.0, 0.0], // 向上
        });
    }

    // 生成侧面索引
    for i in 0..stacks {
        for j in 0..sectors {
            let first = i * (sectors + 1) + j;
            let second = (i + 1) * (sectors + 1) + j;

            // 调整索引顺序以修正法线方向
            indices.push(first);
            indices.push(second);
            indices.push(first + 1);

            indices.push(second);
            indices.push(second + 1);
            indices.push(first + 1);
        }
    }

    // 生成底面索引
    let bottom_edge_start_idx = bottom_center_idx + 2; // 底面边缘顶点开始的索引
    for j in 0..sectors {
        let center = bottom_center_idx;
        let current = bottom_edge_start_idx + j * 2;   // 当前边缘顶点
        let next = if j == sectors - 1 {
            bottom_edge_start_idx // 最后一个连接到第一个
        } else {
            bottom_edge_start_idx + (j + 1) * 2 // 下一个边缘顶点
        };

        indices.push(center);
        indices.push(current);
        indices.push(next);
    }

    // 生成顶面索引
    let top_edge_start_idx = bottom_edge_start_idx + 1; // 顶面边缘顶点开始的索引
    for j in 0..sectors {
        let center = top_center_idx;
        let current = top_edge_start_idx + j * 2;   // 当前边缘顶点
        let next = if j == sectors - 1 {
            top_edge_start_idx // 最后一个连接到第一个
        } else {
            top_edge_start_idx + (j + 1) * 2 // 下一个边缘顶点
        };

        indices.push(center);
        indices.push(next);
        indices.push(current);
    }

    (vertices, indices)
}

// 生成圆柱体边缘渲染的索引
fn generate_cylinder_edge_indices(sectors: u32, stacks: u32) -> Vec<u32> {
    let mut indices = Vec::new();

    // 垂直线：连接相邻堆栈层的对应顶点
    for j in 0..=sectors {
        for i in 0..stacks {
            let current = i * (sectors + 1) + j;
            let next = (i + 1) * (sectors + 1) + j;

            indices.push(current);
            indices.push(next);
        }
    }

    // 底面边缘圆环
    let bottom_center_idx = (stacks + 1) * (sectors + 1); // 侧面顶点总数
    let bottom_edge_start_idx = bottom_center_idx + 2; // 底面中心和顶面中心之后

    for j in 0..sectors {
        let current = bottom_edge_start_idx + j * 2;
        let next = if j == sectors - 1 {
            bottom_edge_start_idx // 连接到第一个
        } else {
            bottom_edge_start_idx + (j + 1) * 2
        };

        indices.push(current);
        indices.push(next);
    }

    // 顶面边缘圆环
    let top_edge_start_idx = bottom_edge_start_idx + 1; // 顶面边缘顶点开始的索引

    for j in 0..sectors {
        let current = top_edge_start_idx + j * 2;
        let next = if j == sectors - 1 {
            top_edge_start_idx // 连接到第一个
        } else {
            top_edge_start_idx + (j + 1) * 2
        };

        indices.push(current);
        indices.push(next);
    }

    indices
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_cylinder() {
        // Test the generation logic without device-dependent operations
        let (vertices, indices) = generate_cylinder(1.0, 2.0, 8, 8);
        
        // Basic checks
        assert!(!vertices.is_empty());
        assert!(!indices.is_empty());
        assert_eq!(indices.len() % 3, 0); // Should be triangles
    }
}
