# 任务清单：Sprite 架构统一重构

## 1. Phase 1: 核心重构（预计 1-2 天）

### 1.1 重命名和结构调整

- [ ] 1.1.1 重命名 `src/render/panel.rs` → `src/render/scene.rs`
  - 文件移动和重命名
  - 更新模块导出：`src/render/mod.rs`

- [ ] 1.1.2 重命名 `src/render/sprite/sprites.rs` → `src/render/sprite/layer.rs`
  - 文件移动和重命名
  - 更新模块导出：`src/render/sprite/mod.rs`

- [ ] 1.1.3 重命名类型：`Panel` → `Scene`
  - 在 `scene.rs` 中重命名结构体
  - 更新所有相关方法和注释

- [ ] 1.1.4 重命名类型：`Sprites` → `Layer`
  - 在 `layer.rs` 中重命名结构体
  - 更新所有相关方法和注释

- [ ] 1.1.5 更新 `src/lib.rs` 中的重导出
  ```rust
  pub use render::scene::Scene;
  pub use render::sprite::Layer;
  ```

### 1.2 修改 Scene 结构

- [ ] 1.2.1 修改 `Scene` 结构定义（保持 layers: Vec<Layer>）
  ```rust
  pub struct Scene {
      pub buffers: [Buffer; 2],
      pub current: usize,
      pub layer_tag_index: HashMap<String, usize>,  // 保留
      pub layers: Vec<Layer>,                        // 保留多层支持
      pub render_index: Vec<(usize, i32)>,
  }
  ```

- [ ] 1.2.2 修改默认层名称
  - layers[0]: "main" → "tui"（TUI 内容层）
  - layers[1]: "pixel" → "sprite"（图形精灵层）

- [ ] 1.2.3 修改 `Scene::new()` 构造函数
  ```rust
  pub fn new() -> Self {
      let (width, height) = (180, 80);
      let size = Rect::new(0, 0, width, height);

      let mut layers = vec![];
      let mut layer_tag_index = HashMap::new();

      // TUI 层 - 包含全屏 buffer sprite
      let mut tui_layer = Layer::new("tui");
      tui_layer.render_weight = 100;  // 在上层
      let tui_sprite = Sprite::new(0, 0, width, height);
      tui_layer.add(tui_sprite, "buffer");
      layers.push(tui_layer);
      layer_tag_index.insert("tui".to_string(), 0);

      // Sprite 层 - 图形精灵
      let sprite_layer = Layer::new("sprite");
      layers.push(sprite_layer);
      layer_tag_index.insert("sprite".to_string(), 1);

      Scene {
          buffers: [Buffer::empty(size), Buffer::empty(size)],
          current: 0,
          layer_tag_index,
          layers,
          render_index: vec![],
      }
  }
  ```

- [ ] 1.2.4 修改 `Scene::init()` 方法
  - 初始化 tui layer 中 buffer sprite 的大小

- [ ] 1.2.5 添加新的辅助方法
  ```rust
  // 获取 TUI buffer 的可变引用
  pub fn tui_buffer_mut(&mut self) -> &mut Buffer {
      &mut self.layers[0].get("buffer").content
  }

  // 添加 Sprite 到 sprite 层
  pub fn add_sprite(&mut self, sprite: Sprite, tag: &str) {
      self.layers[1].add(sprite, tag);
  }

  // 获取 Sprite
  pub fn get_sprite(&mut self, tag: &str) -> &mut Sprite {
      self.layers[1].get(tag)
  }
  ```

- [ ] 1.2.6 修改 `Scene::draw()` 方法（保持多层渲染逻辑）
  ```rust
  pub fn draw(&mut self, ctx: &mut Context) -> io::Result<()> {
      if ctx.stage > LOGO_FRAME {
          self.update_render_index();
          for idx in &self.render_index {
              if !self.layers[idx.0].is_hidden {
                  self.layers[idx.0].render_all_to_buffer(
                      &mut ctx.asset_manager,
                      &mut self.buffers[self.current]
                  );
              }
          }
      }

      let cb = &self.buffers[self.current];
      let pb = &self.buffers[1 - self.current];
      ctx.adapter.draw_all(cb, pb, &mut self.layers, ctx.stage)?;

      if ctx.stage > LOGO_FRAME {
          self.buffers[1 - self.current].reset();
          self.current = 1 - self.current;
      }

      Ok(())
  }
  ```

- [ ] 1.2.7 更新层管理方法（使用新名称）
  - `add_layer()` - 保留，添加自定义层
  - `get_layer()` - 保留
  - `set_layer_weight()` - 保留

- [ ] 1.2.8 保持 `update_render_index()` 方法
  - 按 render_weight 排序所有层

### 1.3 修改 Layer 结构

- [ ] 1.3.1 修改 `Layer` 结构定义
  ```rust
  pub struct Layer {
      pub name: String,
      // 去除 is_pixel: bool 字段
      pub is_hidden: bool,
      pub sprites: Vec<Sprite>,
      pub tag_index: HashMap<String, usize>,
      pub render_index: Vec<(usize, i32)>,
      pub render_weight: i32,
  }
  ```

- [ ] 1.3.2 简化构造函数
  ```rust
  pub fn new(name: &str) -> Self {
      Layer {
          name: name.to_string(),
          is_hidden: false,
          sprites: vec![],
          tag_index: HashMap::new(),
          render_index: vec![],
          render_weight: 0,
      }
  }
  ```

- [ ] 1.3.3 去除 `new_pixel()` 方法
  - 不再需要区分 pixel 和 normal

- [ ] 1.3.4 更新 `render_all_to_buffer()` 方法
  ```rust
  pub fn render_all_to_buffer(&mut self, am: &mut AssetManager, buf: &mut Buffer) {
      self.update_render_index();
      for idx in &self.render_index {
          if !self.sprites[idx.0].is_hidden() {
              // 所有 sprite 都是 pixel sprite
              self.sprites[idx.0].check_asset_request(am);
          }
      }
  }
  ```

### 1.4 修改 Sprite 实现

- [ ] 1.4.1 标记或移除字符渲染 API
  ```rust
  #[deprecated(note = "使用 Widget 系统或直接操作 Stage.tui_sprite.content")]
  pub fn set_color_str(&mut self, ...) { ... }

  #[deprecated(note = "使用 Widget 系统或直接操作 Stage.tui_sprite.content")]
  pub fn set_default_str(&mut self, ...) { ... }
  ```

- [ ] 1.4.2 在文本模式下，图形 API 退化为空操作
  ```rust
  #[cfg(not(graphics_mode))]
  pub fn set_angle(&mut self, _angle: f64) {
      // 文本模式忽略旋转
  }

  #[cfg(not(graphics_mode))]
  pub fn set_alpha(&mut self, _alpha: u8) {
      // 文本模式忽略透明度
  }
  ```

- [ ] 1.4.3 保留 `set_graph_sym()` 方法
  - 在图形模式：正常工作
  - 在文本模式：映射到字符渲染（内部实现）

### 1.5 更新 Adapter 接口

- [ ] 1.5.1 修改 `Adapter` trait 的 `draw_all()` 签名
  ```rust
  fn draw_all(
      &mut self,
      current_buffer: &Buffer,
      previous_buffer: &Buffer,
      layers: &mut Vec<Layer>,     // 保持多层支持
      stage: u32,
  ) -> Result<(), Box<dyn Error>>;
  ```

- [ ] 1.5.2 更新 `cross_adapter.rs` 实现（文本模式）
  - 渲染所有层到终端

- [ ] 1.5.3 更新 `sdl_adapter.rs` 实现
  - 渲染所有层（tui 层 + sprite 层）

- [ ] 1.5.4 更新 `winit_glow_adapter.rs` 实现
  - 同 sdl_adapter

- [ ] 1.5.5 更新 `winit_wgpu_adapter.rs` 实现
  - 同 sdl_adapter

- [ ] 1.5.6 更新 `web_adapter.rs` 实现
  - 同 sdl_adapter

- [ ] 1.5.7 修改 `draw_all_graph()` 辅助方法
  ```rust
  fn draw_all_graph(
      &mut self,
      current_buffer: &Buffer,
      previous_buffer: &Buffer,
      layers: &mut Vec<Layer>,  // 多层
      stage: u32,
  ) {
      // 生成 render buffer
      let rbuf = generate_render_buffer(
          current_buffer,
          previous_buffer,
          layers,
          stage,
          self.get_base(),
      );

      // 渲染
      if self.get_base().gr.rflag {
          self.draw_render_buffer_to_texture(&rbuf, 2, false);
          self.draw_render_textures_to_screen();
      }
  }
  ```

- [ ] 1.5.8 修改 `generate_render_buffer()` 函数签名
  ```rust
  pub fn generate_render_buffer(
      current_buffer: &Buffer,
      previous_buffer: &Buffer,
      layers: &Vec<Layer>,  // 多层
      stage: u32,
      base: &RenderBase,
  ) -> Vec<RenderCell> {
      // ...
  }
  ```

### 1.6 更新导出和文档

- [ ] 1.6.1 更新 `src/render/mod.rs`
  ```rust
  pub mod scene;
  pub use scene::Scene;

  pub mod sprite;
  pub use sprite::{Sprite, Layer};
  ```

- [ ] 1.6.2 更新 `src/lib.rs`
  ```rust
  pub use render::{Scene, Sprite, Layer};
  ```

- [ ] 1.6.3 添加类型别名（可选，用于兼容）
  ```rust
  #[deprecated(note = "使用 Scene 代替")]
  pub type Panel = Scene;

  #[deprecated(note = "使用 Layer 代替")]
  pub type Sprites = Layer;
  ```

## 2. Phase 2: 应用迁移（预计 2-3 天）

### 2.1 迁移 ui_demo（示例应用）

- [ ] 2.1.1 更新导入
  ```rust
  use rust_pixel::render::scene::Scene;
  ```

- [ ] 2.1.2 修改 `UiDemoRender` 结构
  ```rust
  pub struct UiDemoRender {
      pub scene: Scene,  // panel → scene
  }
  ```

- [ ] 2.1.3 修改 `new()` 方法
  ```rust
  pub fn new() -> Self {
      Self {
          scene: Scene::new(),
      }
  }
  ```

- [ ] 2.1.4 修改 `init()` 方法
  ```rust
  fn init(&mut self, ctx: &mut Context, model: &mut UiDemoModel) {
      ctx.adapter.get_base().gr.set_use_tui_height(true);
      ctx.adapter.init(...);
      self.scene.init(ctx);
  }
  ```

- [ ] 2.1.5 修改 `draw()` 方法
  ```rust
  fn draw(&mut self, ctx: &mut Context, model: &mut UiDemoModel, _dt: f32) {
      self.scene.tui_buffer_mut().reset();
      model.ui_app.render_into(self.scene.tui_buffer_mut())?;
      self.scene.draw(ctx)?;
  }
  ```

- [ ] 2.1.6 测试文本模式和图形模式

### 2.2 迁移 snake

- [ ] 2.2.1 更新导入和结构定义（同 ui_demo）

- [ ] 2.2.2 迁移边框渲染
  ```rust
  // 旧代码
  let mut l = Sprite::new(0, 0, SNAKEW + 2, SNAKEH + 2);
  l.set_color_str(20, 0, "SNAKE [RustPixel]", ...);
  t.add_sprite(l, "SNAKE-BORDER");

  // 新代码
  self.scene.tui_buffer_mut().set_string(
      20, 0, "SNAKE [RustPixel]",
      Style::default().fg(Color::Indexed(222))
  );
  ```

- [ ] 2.2.3 保持游戏画面 Sprite（添加到 sprite 层）
  ```rust
  // 使用新 API
  #[cfg(graphics_mode)]
  self.scene.add_sprite(
      Sprite::new(1, 1, SNAKEW as u16, SNAKEH as u16),
      "SNAKE"
  );
  ```

- [ ] 2.2.4 迁移消息渲染
  ```rust
  // 旧代码
  let ml = self.panel.get_sprite("SNAKE-MSG");
  ml.set_color_str(...);

  // 新代码
  self.scene.tui_buffer_mut().set_string(...);
  ```

- [ ] 2.2.5 测试文本模式和图形模式

### 2.3 迁移 tetris

- [ ] 2.3.1 同 snake 的步骤
- [ ] 2.3.2 测试文本模式和图形模式

### 2.4 迁移 tower

- [ ] 2.4.1 更新导入和结构定义
- [ ] 2.4.2 迁移 UI 渲染到 tui_sprite
- [ ] 2.4.3 保持塔防精灵（已经是 pixel sprite）
- [ ] 2.4.4 测试图形模式

### 2.5 迁移 poker

- [ ] 2.5.1 同上
- [ ] 2.5.2 测试文本模式和图形模式

### 2.6 迁移其他应用

- [ ] 2.6.1 tetris_duel
- [ ] 2.6.2 terminal
- [ ] 2.6.3 game2048
- [ ] 2.6.4 ascii_bird
- [ ] 2.6.5 other apps...

## 3. Phase 3: 测试和文档（预计 1 天）

### 3.1 单元测试

- [ ] 3.1.1 添加 `Scene` 单元测试
  ```rust
  #[test]
  fn test_scene_creation() { ... }

  #[test]
  fn test_tui_buffer() { ... }

  #[test]
  fn test_layer_management() { ... }
  ```

- [ ] 3.1.2 添加 `Layer` 单元测试
  ```rust
  #[test]
  fn test_layer_add_remove() { ... }

  #[test]
  fn test_render_index_update() { ... }
  ```

- [ ] 3.1.3 运行所有单元测试
  ```bash
  cargo test
  ```

### 3.2 集成测试

- [ ] 3.2.1 运行所有应用（文本模式）
  ```bash
  cargo pixel r ui_demo t
  cargo pixel r snake t
  cargo pixel r tetris t
  # ... 其他
  ```

- [ ] 3.2.2 运行所有应用（图形模式）
  ```bash
  cargo pixel r ui_demo s
  cargo pixel r snake s
  cargo pixel r tetris s
  # ... 其他
  ```

- [ ] 3.2.3 验证渲染输出一致性
  - 截图对比
  - 交互测试

### 3.3 性能测试

- [ ] 3.3.1 测量 FPS（文本模式）
  ```bash
  cargo pixel r ui_demo t -r
  # 记录 FPS
  ```

- [ ] 3.3.2 测量 FPS（图形模式）
  ```bash
  cargo pixel r ui_demo s -r
  # 记录 FPS
  ```

- [ ] 3.3.3 对比新旧版本性能
  - 确保无明显降低

### 3.4 更新文档

- [ ] 3.4.1 更新 `CLAUDE.md`
  ```markdown
  ## Architecture

  ### Core Design Pattern: Model-Render-Scene

  Scene (场景容器)
  ├── layers[0]: "tui" (TUI 内容层)
  │   └── sprites[0]: "buffer" (全屏 buffer)
  └── layers[1]: "sprite" (图形精灵层)
      └── sprites[...] (游戏精灵)
  ```

- [ ] 3.4.2 更新 `README.md`
  - 示例代码使用 Scene
  - 更新架构图

- [ ] 3.4.3 更新 `doc/` 技术文档
  - 渲染系统说明
  - API 参考

- [ ] 3.4.4 创建迁移指南 `doc/migration/panel-to-scene.md`
  ```markdown
  # Panel → Scene 迁移指南

  ## 背景
  ...

  ## 快速替换
  1. Panel → Scene
  2. Sprites → Layer
  3. panel.add_sprite() → scene.add_sprite()
  4. panel.add_pixel_sprite() → scene.add_sprite()
  5. Normal Sprite 渲染 → scene.tui_buffer_mut().set_string()

  ## 详细示例
  ...
  ```

### 3.5 代码清理

- [ ] 3.5.1 移除 deprecated API（可选）
  - 或保留一段时间，打印警告

- [ ] 3.5.2 运行 clippy
  ```bash
  cargo clippy --all-features
  ```

- [ ] 3.5.3 格式化代码
  ```bash
  cargo fmt --all
  ```

- [ ] 3.5.4 更新 Cargo.toml 版本号
  ```toml
  version = "1.1.0"  # 或 2.0.0 如果是 breaking change
  ```

## 4. 发布准备

### 4.1 Git 提交

- [ ] 4.1.1 创建功能分支
  ```bash
  git checkout -b refactor/sprite-architecture
  ```

- [ ] 4.1.2 分阶段提交
  ```bash
  git commit -m "refactor: rename Panel to Scene, Sprites to Layer"
  git commit -m "refactor: rename layers 'main' to 'tui', 'pixel' to 'sprite'"
  git commit -m "refactor: remove is_pixel flag, unify Layer"
  git commit -m "refactor: migrate ui_demo to new architecture"
  git commit -m "refactor: migrate snake and tetris to new architecture"
  git commit -m "refactor: migrate remaining apps"
  git commit -m "docs: update documentation for new architecture"
  ```

- [ ] 4.1.3 合并到主分支
  ```bash
  git checkout main
  git merge refactor/sprite-architecture
  ```

### 4.2 发布

- [ ] 4.2.1 创建 tag
  ```bash
  git tag -a v2.0.0 -m "Release v2.0.0: Scene/Layer Architecture"
  git push origin v2.0.0
  ```

- [ ] 4.2.2 发布到 crates.io
  ```bash
  cargo publish
  ```

- [ ] 4.2.3 创建 GitHub Release
  - 发布说明
  - 迁移指南链接

## 进度追踪

- Phase 1: ⬜ 0/47 (0%)
- Phase 2: ⬜ 0/17 (0%)
- Phase 3: ⬜ 0/24 (0%)
- Phase 4: ⬜ 0/7 (0%)

**总进度：⬜ 0/95 (0%)**

---

## 注意事项

1. **分阶段提交**：每个大的改动单独提交，便于回滚
2. **保持测试通过**：每个阶段完成后运行测试
3. **文档同步更新**：代码和文档同步更新
4. **性能监控**：每个阶段测试性能，发现问题及时优化
5. **向后兼容**：考虑保留 deprecated API 一段时间

## 里程碑

- ✅ Milestone 1: 核心重构完成（Phase 1）
- ✅ Milestone 2: 示例应用迁移完成（ui_demo, snake, tetris）
- ✅ Milestone 3: 所有应用迁移完成（Phase 2）
- ✅ Milestone 4: 测试和文档完成（Phase 3）
- ✅ Milestone 5: 发布新版本（Phase 4）
