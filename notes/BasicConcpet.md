# WebGPU Basic Concepts

wgpu::Instance 全局入口点，所有webgpu对象都由它创建

wgpu::Surface 呈现图像的平台特定表面
    Instance创建Surface，Surface与Adapter和Device配合工作
    Surface用于配置和获取可渲染纹理

> 多个视角相机不需要多个surface，只需要多个相机uniforms，每个相机视角执行一次渲染pass

wgpu::Adapter代表一个物理GPU设备，是对系统GPU的抽象

wgpu::Device是逻辑GPU设备，是恶创建和管理GPU资源的主要对象

wgpu::Queue是命令提交队列，与device一起创建

> adapter和device都代表gpu，adapter是物理抽象层，device是逻辑接口层。adapter用于查询gpu的能力特性，不直接使用；
device可以用来在gpu傻姑娘创建缓冲区、纹理、管线等资源，是实际与gpu相互的接口对象

wgpu::BindGroupLayout 定义资源绑定结构的模板，描述了着色器如何访问一组资源。
@group(0) @binding(0) 
使用流程
1. 创建 BindGroupLayout (定义接口)
2. 创建 BindGroup (实际绑定资源)
3. 在渲染管线中使用 (设置绑定组布局)
4. 在渲染时设置 (设置实际绑定组)
不指定具体的数据结构，只描述数据类型、访问方式（哪个shader阶段访问）、绑定位置。
BindingType:
- Buffer
    - Uniform (read only, 64kb)
    - Storage (容量比Uniform Buffer大) (particle system, physics system)
- Sampler
- Texture (多维数组，特定的纹理格式，坐标采样)(特有功能：mipmap)
- Storage Texture

## swap chain

交换链是一个缓冲区队列的概念，通常包含2-3个纹理，用于解决渲染和显示之间的同步问题。避免画面撕裂 screen tearing

## frame 与 surface 与 窗口之间的关联

surface 创建时用了window参数

frame 从surface中创建

window使用了winit创建，event_loop.create_window

完整的调用链
```
winit::EventLoop::new() 
    ↓
event_loop.create_window(window_attributes)  ← 创建窗口
    ↓
Arc::new(window)  ← 包装为 Arc
    ↓
State::new(window.clone())  ← 传递给 State
    ↓
instance.create_surface(window.clone())  ← Surface 与窗口关联
    ↓
surface.get_current_texture()  ← 获取与窗口关联的 frame
    ↓
frame.present()  ← 提交到窗口显示
```

## 多个相机视图

关键接口`set_viewport`，可以划分frame上的部分区域给当前renderpass，从而实现一个frame上多个相机视角。

--

## render pass 和 pipeline

command encoder和render pass是每帧创建销毁的，但是pipeline跟buffer是复用的。

pipeline需要设定渲染配置，create_shader_module的时候回编译wgsl代码为gpu可执行机器码，可能出发额外的gpu驱动优化。平时游戏启动时的“编译着色器”过程就包括了项目中的create_shader_module和create_render_pipeline两个步骤。

render pass的具体工作
1. 分配渲染资源
2. 执行加载操作
3. 设置渲染状态
4. 建立渲染context

