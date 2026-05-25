// main cycle is here

use std::sync::Arc;

use crate::get_current_time;
use crate::render::model::ModelVertex;
use cgmath::{InnerSpace, Rotation3, Vector3, Zero};
use web_time::Instant;
use wgpu::util::DeviceExt;
use wgpu::TextureView;
use winit::event::WindowEvent;
use winit::window::Window;

use super::camera::{Camera, CameraController, CameraUniform};
use super::{lib::*, resource, texture};

use super::model::Vertex;

const SPACE_BETWEEN: f32 = 3.0;

// 定义模型实例结构
pub struct ModelInstance {
    pub model: super::model::Model,
    pub instances: Vec<Instance>,
    pub instance_buffer: wgpu::Buffer,
}

impl ModelInstance {
    pub fn new(
        model: super::model::Model,
        instances: Vec<Instance>,
        device: &wgpu::Device,
    ) -> Self {
        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        Self {
            model,
            instances,
            instance_buffer,
        }
    }

    pub fn update_instance_buffer(&mut self, queue: &wgpu::Queue) {
        let instance_data = self
            .instances
            .iter()
            .map(Instance::to_raw)
            .collect::<Vec<_>>();

        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&instance_data),
        );
    }
}

pub struct State<'a> {
    pub surface: wgpu::Surface<'a>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    window: Arc<Window>,
    clear_color: wgpu::Color,

    mesh_pipeline: wgpu::RenderPipeline,

    edge_pipeline: wgpu::RenderPipeline,

    edge_bind_group: wgpu::BindGroup,

    pub camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_controller: CameraController,

    depth_texture: super::texture::Texture,

    // 存储所有模型实例的集合
    model_instances: Vec<ModelInstance>,

    last_update_time: i64,

    pub phy_tick_trigger: bool,
    pub phy_single_step: bool,

    // FPS 限制相关
    pub max_fps: f64,

    // 物理模拟时间步长（与渲染帧率解耦）
    pub physics_time_step: f64, // 固定物理时间步长，如 1/60 秒
    pub accumulated_time: f64,  // 累积时间，用于物理更新

    // FPS 计算相关
    pub frame_count: u32,
    pub last_fps_update: Instant,
    pub current_fps: f64,
}

fn initial_surface_size(window: &Window) -> winit::dpi::PhysicalSize<u32> {
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowExtWebSys;

        let mut size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            if let Some(web_window) = web_sys::window() {
                let scale_factor = web_window.device_pixel_ratio();
                let width = web_window
                    .inner_width()
                    .ok()
                    .and_then(|value| value.as_f64())
                    .unwrap_or(800.0);
                let height = web_window
                    .inner_height()
                    .ok()
                    .and_then(|value| value.as_f64())
                    .unwrap_or(600.0);

                size = winit::dpi::PhysicalSize::new(
                    (width * scale_factor).max(1.0) as u32,
                    (height * scale_factor).max(1.0) as u32,
                );
                let _ = window.request_inner_size(size);

                if let Some(canvas) = window.canvas() {
                    canvas.set_width(size.width);
                    canvas.set_height(size.height);
                }
            }
        }

        size
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        window.inner_size()
    }
}

impl State<'_> {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = initial_surface_size(&window);

        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            desired_maximum_frame_latency: 1,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        // let outline_bind_group_layout =
        //     device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        //         entries: &[wgpu::BindGroupLayoutEntry {
        //             binding: 0,
        //             visibility: wgpu::ShaderStages::FRAGMENT,
        //             ty: wgpu::BindingType::Buffer {
        //                 ty: wgpu::BufferBindingType::Uniform,
        //                 has_dynamic_offset: false,
        //                 min_binding_size: None,
        //             },
        //             count: None,
        //         }],
        //         label: Some("outline_bind_group_layout"),
        //     });

        // // Create outline parameters buffer and bind group
        // let outline_params = super::model::OutlineParams {
        //     outline_color: [0.0, 0.0, 0.0, 1.0], // Black outline
        //     outline_width: 0.02, // Outline width
        //     enable_outline: 1.0, // Enable outline
        // };

        // let outline_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //     label: Some("Outline Params Buffer"),
        //     contents: bytemuck::cast_slice(&[outline_params]),
        //     usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        // });

        // let outline_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        //     layout: &outline_bind_group_layout,
        //     entries: &[wgpu::BindGroupEntry {
        //         binding: 0,
        //         resource: wgpu::BindingResource::Buffer(outline_params_buffer.as_entire_buffer_binding()),
        //     }],
        //     label: Some("outline_bind_group"),
        // });

        let clear_color = wgpu::Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        };

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/mesh_shader.wgsl").into()),
        });

        let camera = Camera {
            eye: (0.0, 0.0, 15.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: config.width as f32 / config.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let mesh_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout, &camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let mesh_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Mesh Pipeline"),
            layout: Some(&mesh_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"), // 1.
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[ModelVertex::desc(), InstanceRaw::desc()], // 2.
            },
            fragment: Some(wgpu::FragmentState {
                // 3.
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    // 4.
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // 2.
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less, // 1.
                stencil: wgpu::StencilState::default(),     // 2.
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,                         // 2.
                mask: !0,                         // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
            multiview: None, // 5.
            cache: None,
        });

        let edge_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Edge Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/edge_shader.wgsl").into()),
        });

        // Create bind group layout for edge parameters
        let edge_params_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("edge_params_bind_group_layout"),
            });

        let edge_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Edge Pipeline Layout"),
            bind_group_layouts: &[
                &texture_bind_group_layout,
                &camera_bind_group_layout,
                &edge_params_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        // Define edge parameters struct
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct EdgeParams {
            thickness: f32,
            _padding: [f32; 3], // Padding to align to 16 bytes
        }

        let edge_params = EdgeParams {
            thickness: 5.0, // Make the edges much thicker
            _padding: [0.0; 3],
        };

        let edge_params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Edge Params Buffer"),
            contents: bytemuck::cast_slice(&[edge_params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let edge_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &edge_params_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: edge_params_buffer.as_entire_binding(),
            }],
            label: Some("edge_bind_group"),
        });

        let edge_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Edge Pipeline"),
            layout: Some(&edge_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &edge_shader,
                entry_point: Some("vs_main"), // 1.
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[ModelVertex::desc(), InstanceRaw::desc()], // 2.
            },
            fragment: Some(wgpu::FragmentState {
                // 3.
                module: &edge_shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    // 4.
                    format: config.format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::One,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList, // 1.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // 2.
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less, // 1.
                stencil: wgpu::StencilState::default(),     // 2.
                bias: wgpu::DepthBiasState::default(),
            }),
            multiview: None, // 5.
            cache: None,
            multisample: wgpu::MultisampleState {
                count: 1,                         // 2.
                mask: !0,                         // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
        });

        let camera_controller = CameraController::new(0.2, 1.0, &size);

        // instances
        let mut obj_instances = (0..NUM_INSTANCE_PER_ROW)
            .flat_map(|z| {
                (0..NUM_INSTANCE_PER_ROW).map(move |x| {
                    let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCE_PER_ROW as f32 / 2.0);
                    let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCE_PER_ROW as f32 / 2.0);
                    let position = cgmath::Vector3 { x, y: 0.0, z };
                    let rotation = if position.is_zero() {
                        cgmath::Quaternion::from_axis_angle(
                            cgmath::Vector3::unit_z(),
                            cgmath::Deg(0.0),
                        )
                    } else {
                        cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
                    };

                    Instance {
                        position,
                        rotation,
                        last_position: position,
                    }
                })
            })
            .collect::<Vec<_>>();

        // rotate vector to show the value of depth buffer
        // println!("instance vec orig: {:?}", obj_instances);
        let instances_len = obj_instances.len();
        for i in 0..(instances_len / 2) {
            obj_instances.swap(i, instances_len - 1 - i);
        }
        // println!("instance vec after: {:?}", obj_instances);

        let instance_data = obj_instances
            .iter()
            .map(Instance::to_raw)
            .collect::<Vec<_>>();
        // let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        //     label: Some("Instance Buffer"),
        //     contents: bytemuck::cast_slice(&instance_data),
        //     usage: wgpu::BufferUsages::VERTEX,
        // });

        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        // let obj_model =
        //     resource::load_model("cube.obj", &device, &queue, &texture_bind_group_layout)
        //         .await
        //         .unwrap();

        // 创建圆柱体实例
        let mut cylinder_instances: Vec<Instance> = vec![];
        for i in 0..10 {
            cylinder_instances.insert(
                0,
                Instance {
                    position: Vector3 {
                        x: i as f32,
                        y: 5.0f32,
                        z: 0.0f32,
                    },
                    last_position: Vector3 {
                        x: i as f32,
                        y: 5.0f32,
                        z: 0.0f32,
                    },
                    rotation: cgmath::Quaternion::zero(),
                },
            );
        }

        // 创建圆柱体模型（用于填充渲染）
        let cylinder_model = resource::generate_cylinder_model(
            &device,
            &queue,
            &texture_bind_group_layout,
            0.5,
            1.0,
            32,
            32,
            None, // Use default color
        )
        .unwrap();

        // 创建圆柱体边缘模型（用于边缘渲染）
        let cylinder_edge_model = resource::generate_cylinder_edge_model(
            &device,
            &queue,
            &texture_bind_group_layout,
            0.5,
            1.0,
            32,
            32,
            None, // Use default color
        )
        .unwrap();

        // 将边缘模型的网格和材质合并到主模型中
        let combined_model = {
            let mut meshes = cylinder_model.meshes;
            // 调整边缘网格的材质索引，因为我们要合并材质
            let mut edge_meshes = cylinder_edge_model.meshes;
            for mesh in &mut edge_meshes {
                // 边缘网格使用边缘模型的材质，其索引需要调整
                mesh.material += cylinder_model.materials.len(); // 边缘材质索引从填充材质之后开始
            }
            meshes.extend(edge_meshes);

            let mut materials = cylinder_model.materials;
            materials.extend(cylinder_edge_model.materials);

            super::model::Model { meshes, materials }
        };

        // 创建模型实例集合
        let mut model_instances = Vec::new();

        // 添加圆柱体模型实例（包含填充和边缘网格）
        let cylinder_model_instance =
            ModelInstance::new(combined_model, cylinder_instances, &device);
        model_instances.push(cylinder_model_instance);

        let last_update_time = get_current_time();

        Self {
            window: window.clone(),
            surface,
            device,
            queue,
            config,
            size,
            clear_color,
            mesh_pipeline,
            edge_pipeline,
            edge_bind_group,
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_controller,
            depth_texture,
            model_instances,
            last_update_time,
            phy_tick_trigger: false,
            phy_single_step: false,
            max_fps: 60.0,                   // 默认60 FPS
            physics_time_step: 1.0 / 60.0,   // 固定物理时间步长为 1/60 秒
            accumulated_time: 0.0,           // 初始累积时间为 0
            frame_count: 0,                  // 初始帧计数为 0
            last_fps_update: Instant::now(), // FPS 更新时间
            current_fps: 0.0,                // 初始 FPS 为 0
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);

            self.depth_texture =
                texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        self.camera_controller.process_events(event)
    }

    pub fn phy_update(&mut self, delta_time: i64) {
        let delta_time_s = (delta_time as f32) / 1000.0;
        let dt2_div2 = delta_time_s * delta_time_s / 2.0;
        let acc = Vector3 {
            x: 0.0,
            y: -9.8,
            z: 0.0,
        };

        // 更新所有模型实例的物理状态
        for model_instance in &mut self.model_instances {
            for instance in &mut model_instance.instances {
                let llast_position = instance.last_position;
                let last_position = instance.position;
                instance.last_position = last_position;
                instance.position = last_position + last_position - llast_position + acc * dt2_div2;
                // println!(
                //     "update sphere instance's position: {:?}, delta_time: {}",
                //     instance.position, delta_time
                // )
            }
        }

        self.phy_update_write_instance_buffer();
    }

    fn phy_update_write_instance_buffer(&mut self) {
        // 更新所有模型实例的缓冲区
        for model_instance in &mut self.model_instances {
            model_instance.update_instance_buffer(&self.queue);
        }
    }

    pub fn update(&mut self) {
        let now = get_current_time();
        let delta_time = now - self.last_update_time;

        // 更新 FPS 计算
        self.frame_count += 1;
        let elapsed_since_fps_update = self.last_fps_update.elapsed().as_secs_f64();
        if elapsed_since_fps_update >= 1.0 {
            self.current_fps = self.frame_count as f64 / elapsed_since_fps_update;
            self.frame_count = 0;
            self.last_fps_update = Instant::now();
        }

        self.camera_controller.update_camera(&mut self.camera);
        self.camera_uniform.update_view_proj(&self.camera);

        // 物理更新逻辑：持续运行、单步执行或不执行
        // 使用固定时间步长，与渲染帧率解耦
        if self.phy_tick_trigger || self.phy_single_step {
            // 将时间转换为秒
            let delta_time_seconds = delta_time as f64 / 1000.0; // 假设 get_current_time 返回毫秒

            // 累积时间
            self.accumulated_time += delta_time_seconds;

            // 使用固定时间步长进行物理更新
            while self.accumulated_time >= self.physics_time_step {
                // 执行固定时间步长的物理更新
                self.phy_update((self.physics_time_step * 1000.0) as i64); // 转换回毫秒

                // 减去一个时间步长
                self.accumulated_time -= self.physics_time_step;
            }

            // 如果是单步执行，则执行后重置标志
            if self.phy_single_step {
                self.phy_single_step = false;
            }
        }

        // 模型的移动更新都可以通过类似的write_buffer来实现
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );

        self.last_update_time = now
    }

    pub fn reset_physics(&mut self) {
        // 重置所有模型实例
        for model_instance in &mut self.model_instances {
            for (i, instance) in model_instance.instances.iter_mut().enumerate() {
                instance.position = Vector3 {
                    x: i as f32,
                    y: 5.0f32,
                    z: 0.0f32,
                };
                instance.last_position = instance.position;
                instance.rotation = cgmath::Quaternion::zero();
            }
        }

        // 更新 GPU 实例缓冲区以反映重置的位置
        self.phy_update_write_instance_buffer();
    }

    pub fn scene_render(&mut self, view: &TextureView) -> Result<(), wgpu::SurfaceError> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.clear_color), // how to handle the color in the previous frame
                        store: wgpu::StoreOp::Store, // store the rendered result to the Texture
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.mesh_pipeline);
            // render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));

            use super::model::DrawModel;
            // let mesh = &self.obj_model.meshes[0];
            // let material = &self.obj_model.materials[mesh.material];
            // render_pass.draw_mesh_instanced(
            //     mesh,
            //     material,
            //     0..self.obj_instances.len() as u32,
            //     &self.camera_bind_group,
            // );

            // 渲染所有模型实例
            for model_instance in &self.model_instances {
                // 渲染填充模型（第一个网格）
                render_pass.set_pipeline(&self.mesh_pipeline);
                render_pass.set_vertex_buffer(1, model_instance.instance_buffer.slice(..));

                if !model_instance.model.meshes.is_empty() {
                    let fill_mesh = &model_instance.model.meshes[0];
                    let material = &model_instance.model.materials[fill_mesh.material];

                    render_pass.draw_mesh_instanced(
                        fill_mesh,
                        material,
                        0..model_instance.instances.len() as u32,
                        &self.camera_bind_group,
                    );
                }

                // 渲染边缘（第二个网格，如果存在）
                if let Some(edge_mesh) = model_instance.model.meshes.get(1) {
                    render_pass.set_pipeline(&self.edge_pipeline);
                    render_pass.set_vertex_buffer(1, model_instance.instance_buffer.slice(..));
                    render_pass.set_bind_group(2, &self.edge_bind_group, &[]); // Set edge parameters bind group

                    let edge_material = &model_instance.model.materials[edge_mesh.material];
                    render_pass.draw_mesh_instanced(
                        edge_mesh,
                        edge_material,
                        0..model_instance.instances.len() as u32,
                        &self.camera_bind_group,
                    );
                }
            }
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        Ok(())
    }
}
