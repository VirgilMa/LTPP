use super::{
    model::{self, Material},
    texture,
};
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
) -> anyhow::Result<model::Model> {
    // 生成球体顶点和索引数据
    let (vertices, indices) = generate_sphere(radius, sectors, stacks);

    let default_color = Rgba([255, 255, 255, 255]);
    let default_texture = texture::Texture::pure_color_texture(
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
            let first = (i * (sectors + 1) + j) as u32;
            let second = first + sectors + 1;

            indices.push(first);
            indices.push(second);
            indices.push(first + 1);

            indices.push(second);
            indices.push(second + 1);
            indices.push(first + 1);
        }
    }

    (vertices, indices)
}
