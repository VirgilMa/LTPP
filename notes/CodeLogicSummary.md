# LTPP 代码逻辑精简文档（面向快速阅读）

## 1) 项目目标（一句话）
这是一个 **Rust + wgpu** 的长期物理仿真练习项目：当前已具备基础渲染与交互 UI，正在逐步补齐刚体物理与碰撞响应。

## 2) 入口与运行链路
1. `src/bin/ltpp_run.rs`：二进制入口，直接调用 `ltpp::run()`。
2. `src/lib.rs`：
   - 初始化日志（native 用 `log4rs`，wasm 用 `console_log`）。
   - 调用 `render::window::render()` 启动主循环。
3. `src/render/window.rs`：
   - 创建 winit event loop。
   - 驱动每帧更新、渲染、imgui 交互。
4. `src/render/state.rs`：
   - GPU 资源（surface/device/queue/pipeline/buffers）初始化。
   - 场景更新（相机、物理步进）与具体 draw 调用。

## 3) 模块职责（按目录）

### `src/render/*`（渲染与交互主干）
- `window.rs`：应用层主循环 + ImGui 面板。
  - 可开关物理模拟、单步执行、重置相机、重置物理、FPS 限制。
- `state.rs`：渲染状态机，持有 camera、pipeline、model instances、物理运行参数。
  - 采用 **固定物理步长**（`physics_time_step`）+ 累积时间，和渲染帧率解耦。
- `camera.rs`：摄像机参数、uniform、输入控制。
- `model.rs` / `resource.rs` / `texture.rs`：模型与纹理加载、GPU 资源管理。
- `shaders/*.wgsl`：mesh 与 edge shader。

### `src/physics/*`（物理域）
- `shape.rs`：物理形状与刚体定义。
  - 当前 shape 主要是 `Cylinder` 与 `Plane`。
  - `PhysicsBody` 包含质量、逆质量、速度、角速度、摩擦、恢复系数等。
- `collision.rs`：碰撞检测函数。
  - 已实现：`cylinder-cylinder`、`cylinder-plane` 的几何检测，并返回接触点/法线/穿透深度。
- `phymgr.rs`：早期/实验性的物理管理草稿。
  - 有重力、速度积分、简单球体碰撞框架，但与 `shape.rs + collision.rs` 新体系并不完全一致。

### `src/common.rs`（通用数学）
- `Transform`（位移/旋转/缩放）及矩阵转换与组合。
- 常量 `PHYSICS_TIMESTEP = 1/60`。

## 4) 运行时数据流（简化）
1. 输入事件（鼠标/键盘） -> camera controller / imgui。
2. 每帧计算 `dt`；渲染频率可变。
3. 物理系统按固定步长迭代 0~N 次（避免渲染帧率波动影响仿真稳定性）。
4. 物理结果写回实例变换。
5. `state.scene_render()` 提交 draw call。
6. imgui overlay 叠加显示运行信息（FPS/按钮开关）。

## 5) 目前“已完成能力”
- wgpu 渲染管线跑通（模型、纹理、深度、camera uniform）。
- 基础相机控制 + ImGui 调参面板。
- 物理开关/单步执行框架。
- 圆柱-圆柱、圆柱-平面碰撞检测函数（几何层）。

## 6) 当前主要技术债（最影响后续迭代）
1. **物理管理存在两套思路并行**：`phymgr.rs` 草稿与 `shape/collision` 新结构未统一。
2. **碰撞检测到碰撞响应链路未闭合**：检测有了，但冲量解算/角速度更新/位置修正仍需完善。
3. **文档与代码状态不完全同步**：README 有重复项与粒度不一致的 TODO。

## 7) 推荐的最小推进路径（可直接按顺序开发）
1. 统一物理核心入口（建议以 `PhysicsBody + collision.rs` 为准，逐步废弃 `phymgr.rs` 草稿）。
2. 完成“圆柱 vs 平面”稳定落地场景：重力 -> 碰撞检测 -> 冲量 -> 静止收敛。
3. 增加调试可视化（接触点、法线、AABB/网格）。
4. 最后再扩展积分器（Implicit Euler / RK4 / Verlet）并做可切换实验。

## 8) 快速定位文件（读代码建议顺序）
1. `src/lib.rs`
2. `src/render/window.rs`
3. `src/render/state.rs`
4. `src/physics/shape.rs`
5. `src/physics/collision.rs`
6. `src/common.rs`
