# RustPixel 技术讲座系列

## 课程大纲

本系列讲座将围绕 RustPixel 开源项目，从 Rust 语言基础开始，逐步深入到项目中使用的各种高级技术。

---

# 第一讲：Rust 基础入门

## 1.1 Rust 语言简介

### 什么是 Rust？
- Mozilla 开发的系统级编程语言
- 特点：内存安全、并发安全、零成本抽象
- 适用场景：系统编程、游戏引擎、嵌入式、WebAssembly

### 为什么选择 Rust？
- **内存安全**：编译时防止悬垂指针、数据竞争
- **性能**：与 C/C++ 相当的运行效率
- **现代化**：优秀的包管理器 Cargo、友好的错误提示
- **跨平台**：一次编写，多平台编译

## 1.2 环境搭建

```bash
# 安装 Rustup（Rust 工具链管理器）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 验证安装
rustc --version
cargo --version

# 创建第一个项目
cargo new hello_rust
cd hello_rust
cargo run
```

## 1.3 基本语法

### 变量与数据类型

```rust
fn main() {
    // 不可变变量（默认）
    let x = 5;
    
    // 可变变量
    let mut y = 10;
    y = 15;
    
    // 类型标注
    let z: i32 = 20;
    
    // 常量
    const MAX_POINTS: u32 = 100_000;
    
    // 基本类型
    let integer: i32 = 42;           // 整数
    let float: f64 = 3.14;           // 浮点数
    let boolean: bool = true;        // 布尔
    let character: char = 'A';       // 字符
    let string: &str = "Hello";      // 字符串切片
}
```

### 所有权系统（Ownership）

Rust 最核心的特性，解决内存管理问题：

```rust
fn main() {
    // 所有权转移
    let s1 = String::from("hello");
    let s2 = s1;  // s1 的所有权转移给 s2
    // println!("{}", s1);  // 错误！s1 已失效
    
    // 克隆
    let s3 = s2.clone();
    println!("{} {}", s2, s3);  // 都可用
    
    // 借用（Borrowing）
    let s4 = String::from("world");
    let len = calculate_length(&s4);  // 不可变借用
    println!("'{}' length: {}", s4, len);  // s4 仍然有效
    
    // 可变借用
    let mut s5 = String::from("hello");
    change(&mut s5);
    println!("{}", s5);
}

fn calculate_length(s: &String) -> usize {
    s.len()
}

fn change(s: &mut String) {
    s.push_str(", world");
}
```

### 结构体与枚举

```rust
// 结构体
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    fn new(x: i32, y: i32) -> Self {
        Point { x, y }
    }
    
    fn distance(&self) -> f64 {
        ((self.x * self.x + self.y * self.y) as f64).sqrt()
    }
}

// 枚举
enum GameState {
    Menu,
    Playing,
    Paused,
    GameOver,
}

// 带数据的枚举
enum Event {
    KeyPress(char),
    MouseClick { x: i32, y: i32 },
    Quit,
}
```

### 模式匹配

```rust
fn handle_event(event: Event) {
    match event {
        Event::KeyPress(c) => println!("Key pressed: {}", c),
        Event::MouseClick { x, y } => println!("Clicked at ({}, {})", x, y),
        Event::Quit => println!("Quitting..."),
    }
}

// Option 类型
fn divide(a: f64, b: f64) -> Option<f64> {
    if b == 0.0 {
        None
    } else {
        Some(a / b)
    }
}

fn main() {
    match divide(10.0, 2.0) {
        Some(result) => println!("Result: {}", result),
        None => println!("Cannot divide by zero"),
    }
}
```

## 1.4 错误处理

```rust
use std::fs::File;
use std::io::{self, Read};

// Result 类型
fn read_file(path: &str) -> Result<String, io::Error> {
    let mut file = File::open(path)?;  // ? 操作符：错误传播
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

fn main() {
    match read_file("data.txt") {
        Ok(content) => println!("File content: {}", content),
        Err(e) => eprintln!("Error reading file: {}", e),
    }
}
```

---

## 1.5 深度扩展：所有权、借用与错误处理的工程化视角

- 所有权边界与开销模型：
  - **Copy 类型零成本**：`u32`, `bool`, 小型 `Copy` 结构在移动时按位拷贝，不触发析构。
  - **Move 仅转移控制权**：`String`, `Vec<T>` 移动转移堆指针所有权，不复制底层缓冲；`clone()` 才复制数据。
  - **借用优先**：在 API 设计中优先 `&T`/`&mut T`，避免不必要的 `clone()`；必要时由调用方选择克隆时机。

- 借用规则的实用化：
  - 非词法生命周期（NLL）允许借用在作用域中更早结束，减少“编译器不够聪明”的误判。
  - `RefCell`/`Mutex` 只是把借用检查推迟到运行时，滥用会掩盖架构问题，应把“可变性”收敛在边界层。

- 错误处理分层：
  - 底层库返回 `Result<T, E>`，错误类型用 `thiserror` 实现可读化，避免 `String`；
  - 应用边界（CLI、WASM）用 `anyhow::Result` 统一兜底并带上下文（`with_context`）。
  - 在 `no_std`/WASM 环境避免 panic，`panic = "abort"` 仅在极致体积优化 profile 使用。

- 性能与可预测性：
  - 热路径避免短生命周期的临时 `String` 分配，优先 `&str`/`Cow<'_, str>`。
  - 批量处理用迭代器链减少中间分配；`SmallVec`/栈上缓存优化小批量场景。

### 常见误区
- 将“可变借用”散落在核心逻辑中，导致可变共享链路过长，难以推理并发安全。
- 随手 `clone()` 造成隐藏的 O(n) 复制开销；应在 API 边界明确“拥有 vs 借用”。

### 讲师提示
- 用 `cargo +nightly rustc -Z self-profile` 或 `cargo flamegraph` 展示一次无意 `clone()` 的开销差异。
- 以 `event` 模块对比 `thread_local! + Rc<RefCell<..>>` 与 `Mutex + lazy_static` 的性能与语义差异。

### 思考题
- 若把 `Context.input_events: Vec<Event>` 改为 `SmallVec<[Event; 32]>` 有何利弊？
- 在 WASM 环境下如何优雅传播错误信息到 JS 端（不 panic）？

#### 参考答案
- SmallVec 利弊：
  - 优点：在事件数量≤32的典型帧中零堆分配；更好的缓存局部性；降低短帧 GC/allocator 压力。
  - 缺点：栈占用上升（按元素大小×32），溢出时仍会堆分配且多一次搬运；元素若较大（如携带大枚举）会放大栈帧；拷贝/移动成本可能上升。
  - 建议：确保 `Event` 紧凑（如用 `#[repr(u8)]` 的小枚举 + 轻量负载），测量实际帧峰值后再定容量。
- WASM 错误传播：
  - 导出接口使用 `Result<T, JsValue>`，在 Rust 端用 `map_err(|e| JsValue::from_str(&e.to_string()))`；
  - 启用 `console_error_panic_hook` 仅作为调试兜底；
  - 对外暴露稳定错误码/结构体（如 `{ code, message }`），前端统一处理并上报；避免 `panic!` 直接中止。

# 第二讲：Cargo 与项目结构

## 2.1 Cargo 基础

### Cargo.toml 配置

```toml
[package]
name = "rust_pixel"
version = "1.0.7"
edition = "2021"
authors = ["zipxing@hotmail.com"]

[dependencies]
serde = { version = "1.0", features = ["derive"] }
rand = "0.9.2"

[dev-dependencies]
criterion = "0.5"

[features]
default = ["log4rs", "crossterm"]
sdl = ["sdl2", "image"]
web = ["wasm-bindgen"]

[profile.release]
opt-level = 's'  # 优化体积
```

### Workspace 工作空间

RustPixel 使用 workspace 管理多个子项目：

```toml
[workspace]
members = [
    "apps/*",
    "tools/*",
]

exclude = [
    "tools/cargo-pixel",
]
```

## 2.2 项目结构设计

RustPixel 的目录结构：

```
rust_pixel/
├── Cargo.toml           # 主项目配置
├── src/                 # 核心引擎代码
│   ├── lib.rs          # 库入口
│   ├── game.rs         # 游戏循环
│   ├── event.rs        # 事件系统
│   ├── render/         # 渲染模块
│   │   ├── adapter/    # 适配器模式
│   │   ├── buffer.rs   # 缓冲区
│   │   └── sprite.rs   # 精灵
│   ├── algorithm/      # 算法库
│   └── ui/             # UI 框架
├── apps/               # 示例应用
│   ├── tetris/        # 俄罗斯方块
│   ├── snake/         # 贪吃蛇
│   └── poker/         # 扑克游戏
└── tools/             # 工具集
    └── cargo-pixel/   # 命令行工具
```

## 2.4 深度扩展：Workspace 与 Feature 的协同

- 分层构建：
  - 根 `rust_pixel` 提供引擎核心；`apps/*` 作为二进制展示/测试，`tools/*` 工程化。
  - `exclude` 大型子目录，缩短 crates.io 发布体积与编译时间。

- Feature 组合的工程影响：
  - `term/sdl/winit/wgpu/web/base` 彼此排斥或组合，决定依赖图（如 `sdl2`、`winit`、`wgpu`）。
  - `base` 降依赖路径用于 FFI/WASM 仅算法导出；保持统一 API 以便切换。

- 构建剖面与体积优化：
  - `opt-level='s'`/`z`、`lto`、`codegen-units=1`、`panic='abort'`；WASM 场景权衡体积 vs 启动时延。
  - 对比 `debug` `release` 的 inlining 与边界检查，演示帧时抖动差异。

### 常见误区
- 将可选后端放入默认 feature，导致未使用平台依赖被强制编译。
- app 与引擎交错依赖，造成循环或难以裁剪。

### 讲师提示
- 现场切换 `cargo pixel r snake t/s/wg/w` 展示 feature 选择带来的依赖与运行时差异。
- 展示 `cargo tree -e features` 与 `cargo bloat` 的依赖、体积可视化。

### 思考题
- 若将 `wgpu` 从默认依赖移除，对使用者体验与发布节奏有何影响？
- 如何把 `apps/template` 抽成独立 crate 以复用脚手架？

#### 参考答案
- 去除 `wgpu` 默认依赖：
  - 体验：首次安装更轻、更少系统依赖冲突；需要 wgpu 的用户需显式启用。
  - 发布：crates.io 体积更小、编译矩阵更快；CI 可分层（基础/图形两套）。
  - 风险：新手可能不清楚如何启用；需在 README 与 `cargo pixel` 中清晰提示。
- 抽出 `apps/template`：
  - 做成 `pixel-template` crate 或 git 模板，暴露 `cargo generate` 模板；
  - 将宏/构建脚本依赖保持最小化；
  - CLI `cargo pixel creat` 内部转调 `cargo generate` 或复制该 crate 的模板资源即可。

## 2.3 模块系统

### 模块定义

```rust
// src/lib.rs
pub mod game;
pub mod event;
pub mod render;
pub mod algorithm;

// 条件编译
#[cfg(not(feature = "base"))]
pub mod ui;

// src/render.rs 或 src/render/mod.rs
pub mod buffer;
pub mod sprite;
pub mod adapter;

// 重新导出
pub use buffer::Buffer;
pub use sprite::Sprites;
```

### 可见性控制

```rust
pub struct Game {
    pub context: Context,      // 公开字段
    model: Model,              // 私有字段
}

impl Game {
    pub fn new() -> Self { }   // 公开方法
    fn internal() { }          // 私有方法
}

pub(crate) fn helper() { }     // 在 crate 内可见
pub(super) fn parent() { }     // 在父模块可见
```

---

# 第三讲：Trait 与泛型

## 3.1 Trait 特征

### 基本 Trait 定义

```rust
// 定义 Model trait
pub trait Model {
    fn init(&mut self, ctx: &mut Context);
    fn update(&mut self, ctx: &mut Context, dt: f32);
    fn handle_input(&mut self, ctx: &mut Context, dt: f32);
    fn handle_timer(&mut self, ctx: &mut Context, dt: f32);
    
    // 默认实现
    fn handle_event(&mut self, ctx: &mut Context, dt: f32) {
        // 默认行为
    }
}

// 实现 trait
struct SnakeModel {
    score: u32,
}

impl Model for SnakeModel {
    fn init(&mut self, ctx: &mut Context) {
        self.score = 0;
    }
    
    fn update(&mut self, ctx: &mut Context, dt: f32) {
        // 游戏逻辑
    }
    
    fn handle_input(&mut self, ctx: &mut Context, dt: f32) {
        // 输入处理
    }
    
    fn handle_timer(&mut self, ctx: &mut Context, dt: f32) {
        // 定时器处理
    }
}
```

### 关联类型

```rust
pub trait Render {
    type Model: Model;  // 关联类型
    
    fn init(&mut self, ctx: &mut Context, m: &mut Self::Model);
    fn draw(&mut self, ctx: &mut Context, m: &mut Self::Model, dt: f32);
}

struct SnakeRender;

impl Render for SnakeRender {
    type Model = SnakeModel;  // 指定关联类型
    
    fn init(&mut self, ctx: &mut Context, m: &mut Self::Model) {
        // 初始化渲染
    }
    
    fn draw(&mut self, ctx: &mut Context, m: &mut Self::Model, dt: f32) {
        // 渲染逻辑
    }
}
```

## 3.2 泛型

### 泛型结构体

```rust
pub struct Game<M, R>
where
    M: Model,
    R: Render<Model = M>,
{
    pub context: Context,
    pub model: M,
    pub render: R,
}

impl<M, R> Game<M, R>
where
    M: Model,
    R: Render<Model = M>,
{
    pub fn new(m: M, r: R, name: &str) -> Self {
        Self {
            context: Context::new(name),
            model: m,
            render: r,
        }
    }
    
    pub fn run(&mut self) -> io::Result<()> {
        loop {
            self.model.update(&mut self.context, 0.016);
            self.render.draw(&mut self.context, &mut self.model, 0.016);
        }
    }
}
```

### Trait 约束

```rust
// 单个约束
fn process<T: Model>(model: &mut T) {
    // ...
}

// 多个约束
fn render<T: Model + Clone + Debug>(model: T) {
    // ...
}

// where 子句（更清晰）
fn complex<M, R>(model: M, render: R) -> Game<M, R>
where
    M: Model + Clone,
    R: Render<Model = M> + Send,
{
    // ...
}
```

## 3.3 常用 Trait

### Display 和 Debug

```rust
use std::fmt;

struct Point {
    x: i32,
    y: i32,
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

// 使用派生宏
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}
```

### Iterator Trait

```rust
struct Counter {
    count: u32,
}

impl Iterator for Counter {
    type Item = u32;
    
    fn next(&mut self) -> Option<Self::Item> {
        self.count += 1;
        if self.count < 6 {
            Some(self.count)
        } else {
            None
        }
    }
}

fn main() {
    let counter = Counter { count: 0 };
    for num in counter {
        println!("{}", num);  // 1, 2, 3, 4, 5
    }
}
```

## 3.4 深度扩展：关联类型 vs 泛型参数、对象安全与动态分发

- 关联类型的语义优势：
  - 读者负担更低：`Render<Model = SnakeModel>` 比 `Render<SnakeModel>` 更自描述；
  - 减少泛型参数在多层传递时的“模板爆炸”。

- 对象安全与 `dyn Trait`：
  - 带有泛型方法或 `Self: Sized` 约束的 trait 不对象安全；
  - 引擎中 `Adapter` 通过对象安全 API 提供统一后端抽象，背后用 `cfg` 选择具体实现。

- 静态分发 vs 动态分发权衡：
  - 热路径优先静态分发（泛型/内联）以获得零开销抽象；
  - 边界层（后端选择、插件系统）用动态分发降低二进制膨胀与编译时长。

### 常见误区
- 在所有层次都使用 `dyn` 导致不可内联与分支预测困难；
- 滥用泛型使编译时间与二进制体积快速膨胀。

### 讲师提示
- 用 `-Z emit-timing`/`cargo bloat` 展示动态分发和静态分发体积差异；
- 以 `Adapter` 为例，演示对象安全的最小必要约束设计。

### 思考题
- 若将 `Game<M, R>` 改为特征对象版本，哪些 API 需要调整以保持对象安全？

#### 参考答案
- 需要移除或改写以下不对象安全点：
  - 关联类型 `Render::Model` 的静态绑定会限制为具体类型；在 `dyn Render` 下需以对象安全方式暴露（如把与 `Model` 交互下沉到统一 trait 对象接口）。
  - 泛型方法/返回 Self 的方法需改为返回 trait 对象或使用构建器注入；
  - `Game` 中 `Render::update(&mut self, ctx, &mut M, dt)` 在对象化后应变为 `RenderDyn::update(&mut self, ctx: &mut Context, model: &mut dyn ModelDyn, dt)`；
  - 代价：失去静态派发与跨调用内联，需评估渲染帧预算是否允许。


---

# 第四讲：智能指针与生命周期

## 4.1 生命周期

### 生命周期标注

```rust
// 生命周期参数
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() {
        x
    } else {
        y
    }
}

// 结构体中的生命周期
struct Context<'a> {
    adapter: &'a mut dyn Adapter,
    buffer: &'a Buffer,
}

impl<'a> Context<'a> {
    fn new(adapter: &'a mut dyn Adapter) -> Self {
        // ...
    }
}
```

### 生命周期省略规则

```rust
// 编译器自动推断
fn first_word(s: &str) -> &str {
    // 等同于：fn first_word<'a>(s: &'a str) -> &'a str
    s.split_whitespace().next().unwrap_or("")
}
```

## 4.2 智能指针

### Box<T> - 堆分配

```rust
// 递归类型必须使用 Box
enum List {
    Cons(i32, Box<List>),
    Nil,
}

// 大对象避免栈溢出
let large_data = Box::new([0u8; 1000000]);

// trait 对象
let widget: Box<dyn Widget> = Box::new(Button::new());
```

### Rc<T> - 引用计数

```rust
use std::rc::Rc;

let a = Rc::new(5);
let b = Rc::clone(&a);  // 增加引用计数
let c = Rc::clone(&a);

println!("count: {}", Rc::strong_count(&a));  // 3
```

### RefCell<T> - 内部可变性

```rust
use std::cell::RefCell;
use std::rc::Rc;

// RustPixel 中的事件中心实现
thread_local! {
    static EVENT_CENTER: Rc<RefCell<HashMap<String, bool>>> = 
        Rc::new(RefCell::new(HashMap::new()));
}

pub fn event_register(name: &str) {
    EVENT_CENTER.with(|ec| {
        let mut map = ec.borrow_mut();  // 运行时借用检查
        map.insert(name.to_string(), false);
    });
}
```

### Arc<T> 和 Mutex<T> - 线程安全

```rust
use std::sync::{Arc, Mutex};
use std::thread;

let counter = Arc::new(Mutex::new(0));
let mut handles = vec![];

for _ in 0..10 {
    let counter = Arc::clone(&counter);
    let handle = thread::spawn(move || {
        let mut num = counter.lock().unwrap();
        *num += 1;
    });
    handles.push(handle);
}

for handle in handles {
    handle.join().unwrap();
}

println!("Result: {}", *counter.lock().unwrap());
```

## 4.3 深度扩展：生命周期推断、Pin/Unpin 与 self-referential 结构

- 生命周期推断与设计：
  - 面向 API 的生命周期参数最小化：仅在返回值与参数存在共享关系时暴露 `'a`；
  - 避免把生命周期向上传播至顶层类型，优先在局部边界消解。

- `Pin`/`Unpin` 背景：
  - 当对象地址不可变更（例如生成指向自身内部的引用或异步状态机）时使用 `Pin<&mut T>`；
  - 大多数自定义类型是 `Unpin`，除非涉及自引用或 FFI 回调将内部指针暴露出去。

- 自引用结构的替代方案：
  - 索引/句柄（`slab`）替代裸引用；
  - 将数据与索引分离，使用 `Arena`/对象池（本项目 `util::objpool`）。

### 常见误区
- 用 `Rc<RefCell<..>>` 在多线程环境共享数据；
- 试图在安全 Rust 中直接构造自引用结构体，导致生命周期难以满足。

### 讲师提示
- 通过 `objpool` 展示句柄式访问的 ergonomics 与借用安全性；
- 用一个简短 async 状态机示例讲解 `Pin` 的必要性。

### 思考题
- 何时应该选择 `Arc<Mutex<T>>`，何时选择消息传递（`mpsc`/`crossbeam`）？

#### 参考答案
- 选择 `Arc<Mutex<T>>`：
  - 共享可变状态、写入频繁但数据量小；临界区短、锁竞争低；
  - 对一致性要求强（多个消费者需要看到同一份即时状态）。
- 选择消息传递：
  - 数据不可变或 copy 代价低；天然消除共享可变性、避免死锁；
  - 生产者/消费者解耦、天然 backpressure；
  - 在高并发、长链路流水线中更可观测与可扩展。


---

# 第五讲：宏与元编程

## 5.1 声明式宏

### macro_rules!

```rust
// RustPixel 中的 pixel_game 宏
#[macro_export]
macro_rules! pixel_game {
    ($name:ident) => {
        mod model;
        #[cfg(not(graphics_mode))]
        mod render_terminal;
        #[cfg(graphics_mode)]
        mod render_graphics;

        use rust_pixel::game::Game;
        
        pub struct [<$name Game>] {
            g: Game<[<$name Model>], [<$name Render>]>,
        }
        
        pub fn init_game() -> [<$name Game>] {
            let m = [<$name Model>]::new();
            let r = [<$name Render>]::new();
            let mut g = Game::new(m, r, stringify!([<$name:lower>]));
            g.init();
            [<$name Game>] { g }
        }
    };
}

// 使用
pixel_game!(Snake);  // 生成 SnakeGame, SnakeModel, SnakeRender
```

### 常用宏模式

```rust
// 重复模式
macro_rules! vec_of_strings {
    ($($x:expr),*) => {
        vec![$(String::from($x)),*]
    };
}

let v = vec_of_strings!["a", "b", "c"];

// 条件编译宏
macro_rules! only_terminal_mode {
    () => {
        #[cfg(graphics_mode)]
        {
            println!("Run in terminal only...");
            std::process::exit(0);
        }
    };
}
```

## 5.3 深度扩展：声明式宏可读性、过程宏边界与 hygienic 标识符

- 声明式宏的可维护性：
  - 将宏输入限制为“结构化片段”，减少自由文本拼接；
  - 使用 `macro_rules!` 的重复匹配（`$(...)*`）与命名捕获提升可读性；
  - 宏内部少做业务逻辑，主要负责样板代码展开。

- 过程宏应用边界：
  - 派生宏适合重复模式（序列化、日志注入、组件注册）；
  - 属性宏用于生成桥接代码（如 `wasm_bindgen` 导出、FFI 包装）；
  - 减少编译期复杂逻辑，保持增量编译速度。

- Hygiene（卫生性）与作用域：
  - 利用 `paste` 拼接标识符时，注意命名冲突与可见性；
  - 对外导出的宏需要 `#[macro_export]` 并文档化约束与副作用。

### 常见误区
- 在宏中隐藏大量副作用（注册全局状态、读写文件）使可测试性变差；
- 将过程宏当作“编译期脚本语言”，导致编译时间过长。

### 讲师提示
- 展示 `pixel_game!` 如何把同一游戏产出终端/桌面/WASM 入口，强调宏在“统一架构入口”上的价值。

### 思考题
- 若把 `pixel_game!` 拆为多宏（入口、WASM 导出、日志初始化），能否提升可维护性？

#### 参考答案
- 可以提升可维护性：
  - 各宏关注点单一，便于测试与文档化；调用方按需组合，减少不必要展开；
  - 缺点：调用点增多、易错；需要在宏之间定义清晰的数据契约（命名/路径/可见性）。
- 折中方案：
  - 提供一个“总宏”包装常见组合，同时导出子宏给高级用户；
  - 在子宏间共享 `paste` 约定，避免命名分叉。

## 5.2 过程宏

### 派生宏

```rust
// 使用 derive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameState {
    score: u32,
    level: u8,
}
```

### 属性宏

```rust
// wasm_bindgen 属性宏
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct SnakeGame {
    g: Game<SnakeModel, SnakeRender>,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl SnakeGame {
    pub fn new() -> Self {
        // ...
    }
    
    pub fn tick(&mut self, dt: f32) {
        self.g.on_tick(dt);
    }
}
```

## 5.3 paste 宏

RustPixel 使用 paste 进行标识符拼接：

```rust
use paste::paste;

paste! {
    // 拼接标识符
    pub struct [<$name Game>] { }
    
    impl [<$name Game>] {
        pub fn [<new_ $name:lower>]() -> Self { }
    }
}
```

---

# 第六讲：条件编译与特性门控

## 6.1 cfg 属性

### 平台特定代码

```rust
// 针对不同操作系统
#[cfg(target_os = "windows")]
fn platform_specific() {
    println!("Running on Windows");
}

#[cfg(target_os = "macos")]
fn platform_specific() {
    println!("Running on macOS");
}

#[cfg(target_os = "linux")]
fn platform_specific() {
    println!("Running on Linux");
}
```

### 架构特定代码

```rust
// WebAssembly 平台
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
use rodio::Audio;

// RustPixel 中的音频处理
#[cfg(not(any(target_os = "android", target_os = "ios", target_arch = "wasm32")))]
pub mod audio {
    use rodio::*;
    // 桌面平台音频实现
}
```

## 6.2 Features 特性门控

### Cargo.toml 中定义

```toml
[features]
default = ["log4rs", "crossterm", "rodio"]
term = ["log4rs", "crossterm", "rodio"]
sdl = ["log4rs", "rodio", "sdl2", "image"]
winit = ["log4rs", "rodio", "winit", "glutin"]
wgpu = ["log4rs", "rodio", "wgpu", "bytemuck"]
web = ["image"]
base = ["log4rs"]  # 最小依赖模式
```

### 代码中使用

```rust
// 基于 feature 条件编译
#[cfg(not(feature = "base"))]
pub mod render;

#[cfg(not(feature = "base"))]
pub mod ui;

#[cfg(feature = "sdl")]
pub mod sdl_adapter;

#[cfg(feature = "wgpu")]
pub mod wgpu_adapter;
```

## 6.3 自定义 cfg 别名

RustPixel 使用 `build.rs` 定义自定义配置：

```rust
// build.rs
use cfg_aliases::cfg_aliases;

fn main() {
    cfg_aliases! {
        // 渲染后端别名
        cross_backend: { feature = "crossterm" },
        sdl_backend: { feature = "sdl" },
        winit_backend: { feature = "winit" },
        wgpu_backend: { feature = "wgpu" },
        wasm: { target_arch = "wasm32" },
        
        // 渲染模式
        graphics_mode: { any(sdl_backend, winit_backend, wgpu_backend, wasm) },
    }
}

// 在代码中使用
#[cfg(graphics_mode)]
pub mod render_graphics;

#[cfg(not(graphics_mode))]
pub mod render_terminal;
```

## 6.4 深度扩展：多后端矩阵的维护与测试

- 后端组合矩阵：`term/sdl/winit/wgpu/web` × 平台（macOS/Linux/Windows/WASM）。
  - 以 `cfg_aliases` 收敛组合，避免到处堆砌 `#[cfg(any(...))]`。
  - 将最小可运行子集（如 `base`）作为 CI 常驻任务，完整矩阵用 nightly/周构建降低成本。

- 测试策略：
  - 单元测试：算法、`buffer`/`panel` 的纯逻辑；
  - 端到端：`cargo pixel r` 的 smoke test（跑一帧退出），WASM 用 headless 浏览器；
  - 特性组合测试：`--features term`/`wgpu` 等最小用例。

- 文档与示例：
  - `README` 给出常见组合与依赖；
  - `cargo pixel` 作为统一入口，减少“环境不齐”带来的讲座现场阻塞。

### 常见误区
- 在库层做运行时后端选择，导致体积与初始化复杂；Rust 更适合“编译期选择”。

### 讲师提示
- 展示切换 feature 时 `Adapter` 的替换与二进制大小变化，帮助受众建立“编译期多态”的直觉。

### 思考题
- 如果需要在运行时在 WGPU 与 OpenGL 间切换，如何设计“薄抽象 + 双二进制”来保持性能与可维护性？

#### 参考答案
- 运行时切换的成本：单二进制包含双后端实现，体积膨胀、初始化复杂、难以内联；
- 推荐方案：
  - 编译期多态 + 双二进制发布（`myapp-wgpu` / `myapp-glow`），用启动器脚本/CLI 参数选择；
  - 或通过插件化（`dlopen`/feature flag）加载所需后端，核心逻辑以 FFI 接口暴露，保持“薄抽象”。


---

# 第七讲：RustPixel 架构设计

## 7.1 整体架构

### MVC 模式变体

RustPixel 采用 Model-Render-Game 三层架构：

```
┌─────────────────────────────────────────┐
│              Game Loop                  │
│  ┌─────────────┐    ┌─────────────┐     │
│  │   Model     │◄──►│   Render    │     │
│  │  (Logic)    │    │ (Graphics)  │     │
│  └─────────────┘    └─────────────┘     │
│         │                   │            │
│         ▼                   ▼            │
│  ┌──────────────────────────────────┐    │
│  │         Context                  │    │
│  │  - Adapter (渲染后端)            │    │
│  │  - Event System (事件系统)       │    │
│  │  - Asset Manager (资源管理)      │    │
│  └──────────────────────────────────┘    │
└─────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────┐
│          Adapter Layer                  │
│  ┌──────┬──────┬──────┬──────┐          │
│  │Cross │ SDL  │Winit │ Web  │          │
│  │term  │      │      │      │          │
│  └──────┴──────┴──────┴──────┘          │
└─────────────────────────────────────────┘
```

### 核心组件

```rust
// Game 结构
pub struct Game<M, R>
where
    M: Model,
    R: Render<Model = M>,
{
    pub context: Context,
    pub model: M,
    pub render: R,
}

// Model trait
pub trait Model {
    fn init(&mut self, ctx: &mut Context);
    fn update(&mut self, ctx: &mut Context, dt: f32);
    fn handle_event(&mut self, ctx: &mut Context, dt: f32);
    fn handle_timer(&mut self, ctx: &mut Context, dt: f32);
    fn handle_input(&mut self, ctx: &mut Context, dt: f32);
    fn handle_auto(&mut self, ctx: &mut Context, dt: f32);
}

// Render trait
pub trait Render {
    type Model: Model;
    
    fn init(&mut self, ctx: &mut Context, m: &mut Self::Model);
    fn update(&mut self, ctx: &mut Context, m: &mut Self::Model, dt: f32);
    fn draw(&mut self, ctx: &mut Context, m: &mut Self::Model, dt: f32);
}
```

## 7.2 适配器模式

### Adapter Trait

```rust
pub trait Adapter {
    // 初始化
    fn init(&mut self, w: u16, h: u16, rx: f32, ry: f32, title: String);
    
    // 重置
    fn reset(&mut self);
    
    // 事件轮询
    fn poll_event(&mut self, timeout: Duration, ev: &mut Vec<Event>) -> bool;
    
    // 渲染
    fn draw_all(
        &mut self,
        current_buffer: &Buffer,
        previous_buffer: &Buffer,
        pixel_sprites: &mut Vec<Sprites>,
        stage: u32,
    ) -> Result<(), String>;
    
    // 辅助方法
    fn hide_cursor(&mut self) -> Result<(), String>;
    fn show_cursor(&mut self) -> Result<(), String>;
    fn set_cursor(&mut self, x: u16, y: u16) -> Result<(), String>;
}
```

### 多种实现

```rust
// 终端适配器
#[cfg(cross_backend)]
pub struct CrosstermAdapter {
    base: AdapterBase,
    // crossterm 特定字段
}

// SDL 适配器
#[cfg(sdl_backend)]
pub struct SdlAdapter {
    base: AdapterBase,
    canvas: Canvas<Window>,
    // SDL 特定字段
}

// Web 适配器
#[cfg(wasm)]
pub struct WebAdapter {
    base: AdapterBase,
    gl: WebGl2RenderingContext,
    // WebGL 特定字段
}

// WGPU 适配器
#[cfg(wgpu_backend)]
pub struct WinitWgpuAdapter {
    base: AdapterBase,
    device: wgpu::Device,
    queue: wgpu::Queue,
    // WGPU 特定字段
}
```

## 7.3 事件系统

### 全局事件中心

```rust
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static EVENT_CENTER: Rc<RefCell<HashMap<String, HashMap<String, bool>>>> = 
        Rc::new(RefCell::new(HashMap::new()));
}

// 注册事件
pub fn event_register(event: &str, func: &str) {
    EVENT_CENTER.with(|ec| {
        let mut ec_ref = ec.borrow_mut();
        match ec_ref.get_mut(event) {
            Some(ht) => {
                ht.insert(func.to_string(), false);
            }
            None => {
                let mut h: HashMap<String, bool> = HashMap::new();
                h.insert(func.to_string(), false);
                ec_ref.insert(event.to_string(), h);
            }
        }
    });
}

// 触发事件
pub fn event_emit(event: &str) {
    EVENT_CENTER.with(|ec| {
        let mut ec_ref = ec.borrow_mut();
        if let Some(ht) = ec_ref.get_mut(event) {
            for value in ht.values_mut() {
                *value = true;
            }
        }
    });
}

// 检查事件
pub fn event_check(event: &str, func: &str) -> bool {
    EVENT_CENTER.with(|ec| {
        let mut ec_ref = ec.borrow_mut();
        if let Some(ht) = ec_ref.get_mut(event) { 
            if let Some(flag) = ht.get_mut(func) {
                if *flag {
                    *flag = false;
                    return true;
                }
            } 
        }
        false
    })
}
```

### 定时器系统

```rust
thread_local! {
    static GAME_TIMER: Rc<RefCell<Timers>> = 
        Rc::new(RefCell::new(Timers::new()));
}

pub fn timer_register(name: &str, time: f32, func: &str) {
    GAME_TIMER.with(|gt| {
        gt.borrow_mut().register(name, time, func);
    });
}

pub fn timer_fire<T: Serialize>(name: &str, value: T) {
    GAME_TIMER.with(|gt| {
        gt.borrow_mut().fire(name, value);
    });
}

// 在游戏循环中更新
pub fn timer_update() {
    GAME_TIMER.with(|gt| {
        gt.borrow_mut().update();
    });
}
```

---

# 第八讲：渲染系统深入

## 8.1 Buffer 系统

### Cell 结构

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    pub symbol: char,              // 字符
    pub fg: Color,                 // 前景色
    pub bg: Option<Color>,         // 背景色
    pub modifier: Modifier,        // 修饰符（粗体、斜体等）
}

impl Cell {
    pub fn new(symbol: char) -> Self {
        Self {
            symbol,
            fg: Color::Reset,
            bg: None,
            modifier: Modifier::empty(),
        }
    }
}
```

### Buffer 结构

```rust
pub struct Buffer {
    pub area: Rect,                    // 缓冲区区域
    pub content: Vec<Cell>,            // 单元格数组
}

impl Buffer {
    pub fn new(area: Rect) -> Self {
        let size = area.area() as usize;
        Self {
            area,
            content: vec![Cell::default(); size],
        }
    }
    
    pub fn get(&self, x: u16, y: u16) -> &Cell {
        let index = self.index_of(x, y);
        &self.content[index]
    }
    
    pub fn set(&mut self, x: u16, y: u16, cell: Cell) {
        let index = self.index_of(x, y);
        self.content[index] = cell;
    }
    
    fn index_of(&self, x: u16, y: u16) -> usize {
        ((y - self.area.y) * self.area.width + (x - self.area.x)) as usize
    }
    
    // Diff 渲染：只渲染改变的部分
    pub fn diff<'a>(&self, other: &'a Buffer) -> Vec<(u16, u16, &'a Cell)> {
        let mut updates = vec![];
        for y in 0..self.area.height {
            for x in 0..self.area.width {
                let current = self.get(x, y);
                let other_cell = other.get(x, y);
                if current != other_cell {
                    updates.push((x, y, other_cell));
                }
            }
        }
        updates
    }
}
```

## 8.2 Panel 绘图 API

### 基本绘图

```rust
impl Panel {
    // 绘制字符
    pub fn put(&mut self, x: i32, y: i32, c: char, s: &Style) {
        if x >= 0 && y >= 0 && x < self.width as i32 && y < self.height as i32 {
            let cell = Cell {
                symbol: c,
                fg: s.fg,
                bg: s.bg,
                modifier: s.modifier,
            };
            self.buffer.set(x as u16, y as u16, cell);
        }
    }
    
    // 绘制字符串
    pub fn print(&mut self, x: i32, y: i32, text: &str, s: &Style) {
        let mut cx = x;
        for c in text.chars() {
            self.put(cx, y, c, s);
            cx += 1;
        }
    }
    
    // 绘制矩形
    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, s: &Style) {
        for dy in 0..h {
            for dx in 0..w {
                self.put(x + dx, y + dy, ' ', s);
            }
        }
    }
    
    // 绘制边框
    pub fn border(&mut self, x: i32, y: i32, w: i32, h: i32, s: &Style) {
        // 角
        self.put(x, y, '┌', s);
        self.put(x + w - 1, y, '┐', s);
        self.put(x, y + h - 1, '└', s);
        self.put(x + w - 1, y + h - 1, '┘', s);
        
        // 边
        for dx in 1..w-1 {
            self.put(x + dx, y, '─', s);
            self.put(x + dx, y + h - 1, '─', s);
        }
        for dy in 1..h-1 {
            self.put(x, y + dy, '│', s);
            self.put(x + w - 1, y + dy, '│', s);
        }
    }
}
```

## 8.3 Sprite 系统

### Sprite 结构

```rust
pub struct Sprites {
    pub x: f32,              // 精确位置（支持亚像素）
    pub y: f32,
    pub w: u16,              // 宽度
    pub h: u16,              // 高度
    pub visible: bool,       // 可见性
    pub buffer: Buffer,      // 内容缓冲
}

impl Sprites {
    pub fn new(x: f32, y: f32, w: u16, h: u16) -> Self {
        Self {
            x,
            y,
            w,
            h,
            visible: true,
            buffer: Buffer::new(Rect::new(0, 0, w, h)),
        }
    }
    
    // 移动
    pub fn move_to(&mut self, x: f32, y: f32) {
        self.x = x;
        self.y = y;
    }
    
    // 绘制到主缓冲区
    pub fn render_to(&self, target: &mut Buffer) {
        if !self.visible {
            return;
        }
        
        let start_x = self.x as i32;
        let start_y = self.y as i32;
        
        for y in 0..self.h {
            for x in 0..self.w {
                let tx = start_x + x as i32;
                let ty = start_y + y as i32;
                if tx >= 0 && ty >= 0 {
                    let cell = self.buffer.get(x, y);
                    target.set(tx as u16, ty as u16, *cell);
                }
            }
        }
    }
}
```

## 8.4 图形模式渲染管线

### RenderCell 结构

```rust
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RenderCell {
    pub position: [f32; 2],      // 屏幕位置
    pub tex_coords: [f32; 2],    // 纹理坐标
    pub color: [f32; 4],         // 前景色 RGBA
    pub bg_color: [f32; 4],      // 背景色 RGBA
    pub tex_index: u32,          // 纹理索引
    pub symbol_index: u32,       // 符号索引
}
```

### 渲染流程

```rust
#[cfg(graphics_mode)]
fn draw_all_graph(
    &mut self,
    current_buffer: &Buffer,
    previous_buffer: &Buffer,
    pixel_sprites: &mut Vec<Sprites>,
    stage: u32,
) {
    // 步骤1：生成渲染单元数组
    let rbuf = generate_render_buffer(
        current_buffer,
        previous_buffer,
        pixel_sprites,
        stage,
        self.get_base(),
    );

    // 步骤2：渲染到纹理
    if self.get_base().gr.rflag {
        self.draw_render_buffer_to_texture(&rbuf, 2, false);
        self.draw_render_textures_to_screen();
    } else {
        self.get_base().gr.rbuf = rbuf;
    }
}
```

---

# 第九讲：并发与异步

## 9.1 线程基础

### 创建线程

```rust
use std::thread;
use std::time::Duration;

fn main() {
    let handle = thread::spawn(|| {
        for i in 1..10 {
            println!("spawned thread: {}", i);
            thread::sleep(Duration::from_millis(1));
        }
    });
    
    for i in 1..5 {
        println!("main thread: {}", i);
        thread::sleep(Duration::from_millis(1));
    }
    
    handle.join().unwrap();
}
```

### 线程间数据共享

```rust
use std::sync::{Arc, Mutex};

struct GameState {
    score: i32,
    level: i32,
}

fn main() {
    let state = Arc::new(Mutex::new(GameState {
        score: 0,
        level: 1,
    }));
    
    let state_clone = Arc::clone(&state);
    let handle = thread::spawn(move || {
        let mut s = state_clone.lock().unwrap();
        s.score += 100;
    });
    
    handle.join().unwrap();
    
    let s = state.lock().unwrap();
    println!("Score: {}", s.score);
}
```

## 9.2 Channel 消息传递

```rust
use std::sync::mpsc;

enum GameEvent {
    KeyPress(char),
    MouseClick(i32, i32),
    Quit,
}

fn main() {
    let (tx, rx) = mpsc::channel();
    
    // 发送方线程
    thread::spawn(move || {
        tx.send(GameEvent::KeyPress('a')).unwrap();
        tx.send(GameEvent::MouseClick(100, 200)).unwrap();
        tx.send(GameEvent::Quit).unwrap();
    });
    
    // 接收方
    for event in rx {
        match event {
            GameEvent::KeyPress(c) => println!("Key: {}", c),
            GameEvent::MouseClick(x, y) => println!("Click: {}, {}", x, y),
            GameEvent::Quit => break,
        }
    }
}
```

## 9.3 异步编程

### async/await

```rust
use std::future::Future;

// 异步函数
async fn load_asset(path: &str) -> Result<Vec<u8>, std::io::Error> {
    // 模拟异步 I/O
    tokio::fs::read(path).await
}

async fn init_game() {
    // 并发加载多个资源
    let (texture, sound, level) = tokio::join!(
        load_asset("texture.png"),
        load_asset("sound.mp3"),
        load_asset("level1.dat"),
    );
    
    println!("All assets loaded!");
}

#[tokio::main]
async fn main() {
    init_game().await;
}
```

### RustPixel 中的资源加载

```rust
// Web 平台异步加载
#[cfg(target_arch = "wasm32")]
pub async fn load_assets_async(urls: Vec<String>) -> HashMap<String, Vec<u8>> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, Response};
    
    let mut assets = HashMap::new();
    
    for url in urls {
        let request = Request::new_with_str(&url).unwrap();
        let window = web_sys::window().unwrap();
        let resp_value = JsFuture::from(window.fetch_with_request(&request))
            .await
            .unwrap();
        
        let resp: Response = resp_value.dyn_into().unwrap();
        let array_buffer = JsFuture::from(resp.array_buffer().unwrap())
            .await
            .unwrap();
        
        let data = js_sys::Uint8Array::new(&array_buffer).to_vec();
        assets.insert(url.clone(), data);
    }
    
    assets
}
```

---

# 第十讲：FFI 与 C 语言互操作

## 10.1 FFI 基础

### 导出 Rust 函数给 C

```rust
// Cargo.toml
[lib]
crate-type = ["cdylib", "rlib"]

// lib.rs
#[no_mangle]
pub extern "C" fn rust_add(a: i32, b: i32) -> i32 {
    a + b
}

#[repr(C)]
pub struct Point {
    x: f64,
    y: f64,
}

#[no_mangle]
pub extern "C" fn point_distance(p: *const Point) -> f64 {
    unsafe {
        let point = &*p;
        (point.x * point.x + point.y * point.y).sqrt()
    }
}
```

### C 头文件生成

使用 cbindgen 自动生成 C 头文件：

```toml
# cbindgen.toml
language = "C"
cpp_compat = true

[export]
include = ["Point"]
```

```bash
cbindgen --config cbindgen.toml --crate poker_ffi --output poker_ffi.h
```

生成的头文件：

```c
#ifndef POKER_FFI_H
#define POKER_FFI_H

#include <stdint.h>

typedef struct Point {
    double x;
    double y;
} Point;

int32_t rust_add(int32_t a, int32_t b);
double point_distance(const Point* p);

#endif
```

## 10.2 RustPixel 中的 FFI 实现

### 扑克算法 FFI

```rust
// apps/poker/ffi/src/lib.rs

#[repr(C)]
pub struct HandResult {
    rank: u8,        // 牌型等级
    score: u32,      // 得分
}

#[no_mangle]
pub extern "C" fn poker_evaluate_hand(
    cards: *const u8,
    count: usize
) -> HandResult {
    let cards_slice = unsafe {
        std::slice::from_raw_parts(cards, count)
    };
    
    // 调用 Rust 核心算法
    let result = evaluate_poker_hand(cards_slice);
    
    HandResult {
        rank: result.rank,
        score: result.score,
    }
}

#[no_mangle]
pub extern "C" fn poker_compare_hands(
    hand1: *const u8,
    count1: usize,
    hand2: *const u8,
    count2: usize,
) -> i32 {
    unsafe {
        let h1 = std::slice::from_raw_parts(hand1, count1);
        let h2 = std::slice::from_raw_parts(hand2, count2);
        
        compare_hands(h1, h2)
    }
}

#[no_mangle]
pub extern "C" fn poker_free_string(s: *mut std::os::raw::c_char) {
    unsafe {
        if !s.is_null() {
            let _ = std::ffi::CString::from_raw(s);
        }
    }
}
```

### C++ 调用示例

```cpp
// test.cc
#include "poker_ffi.h"
#include <iostream>

int main() {
    // 测试加法
    int result = rust_add(10, 20);
    std::cout << "10 + 20 = " << result << std::endl;
    
    // 测试扑克牌
    uint8_t hand[] = {1, 2, 3, 4, 5};  // 示例牌
    HandResult hr = poker_evaluate_hand(hand, 5);
    std::cout << "Hand rank: " << (int)hr.rank << std::endl;
    std::cout << "Hand score: " << hr.score << std::endl;
    
    return 0;
}
```

### Python 调用示例

```python
# testffi.py
import ctypes

# 加载动态库
lib = ctypes.CDLL("./target/release/libpoker_ffi.dylib")

# 定义函数签名
lib.rust_add.argtypes = [ctypes.c_int32, ctypes.c_int32]
lib.rust_add.restype = ctypes.c_int32

# 定义结构体
class HandResult(ctypes.Structure):
    _fields_ = [
        ("rank", ctypes.c_uint8),
        ("score", ctypes.c_uint32),
    ]

lib.poker_evaluate_hand.argtypes = [ctypes.POINTER(ctypes.c_uint8), ctypes.c_size_t]
lib.poker_evaluate_hand.restype = HandResult

# 调用
result = lib.rust_add(10, 20)
print(f"10 + 20 = {result}")

# 扑克牌评估
hand = (ctypes.c_uint8 * 5)(1, 2, 3, 4, 5)
hr = lib.poker_evaluate_hand(hand, 5)
print(f"Hand rank: {hr.rank}, score: {hr.score}")
```

---

# 第十一讲：WebAssembly 部署

## 11.1 WASM 基础

### wasm-bindgen

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Game {
    score: u32,
}

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Game {
        Game { score: 0 }
    }
    
    pub fn tick(&mut self, dt: f32) {
        // 游戏逻辑
    }
    
    pub fn get_score(&self) -> u32 {
        self.score
    }
    
    pub fn handle_key(&mut self, key: &str) {
        // 处理按键
    }
}

// 导出给 JS 调用的函数
#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}
```

### Cargo 配置

```toml
[package]
name = "snake_wasm"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = [
    "Document",
    "Element",
    "HtmlCanvasElement",
    "WebGlRenderingContext",
    "WebGl2RenderingContext",
    "KeyboardEvent",
    "MouseEvent",
] }
console_error_panic_hook = "0.1"

[profile.release]
opt-level = "s"  # 优化体积
lto = true       # 链接时优化
```

## 11.2 JS 交互

### JavaScript 端

```javascript
// index.js
import init, { SnakeGame } from './pkg/snake.js';

async function run() {
    // 初始化 WASM
    await init();
    
    // 创建游戏实例
    const game = SnakeGame.new();
    
    // 游戏循环
    let lastTime = 0;
    function gameLoop(timestamp) {
        const dt = (timestamp - lastTime) / 1000.0;
        lastTime = timestamp;
        
        // 更新游戏
        game.tick(dt);
        
        requestAnimationFrame(gameLoop);
    }
    
    requestAnimationFrame(gameLoop);
    
    // 键盘事件
    document.addEventListener('keydown', (e) => {
        game.key_event(0, e);
    });
    
    // 鼠标事件
    canvas.addEventListener('mousedown', (e) => {
        game.key_event(1, e);
    });
}

run();
```

### HTML 模板

```html
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>Snake Game</title>
    <style>
        body {
            margin: 0;
            padding: 0;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            background: #000;
        }
        canvas {
            border: 2px solid #fff;
        }
    </style>
</head>
<body>
    <canvas id="game-canvas" width="800" height="600"></canvas>
    <script type="module" src="./index.js"></script>
</body>
</html>
```

## 11.3 RustPixel WASM 部署

### 构建脚本

```makefile
# apps/snake/wasm/Makefile

build:
	wasm-pack build --target web --release

serve: build
	python3 -m http.server 8080

clean:
	rm -rf pkg target
```

### pixel_game 宏 WASM 支持

```rust
#[cfg(wasm)]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl SnakeGame {
    pub fn new() -> Self {
        // 设置 panic hook
        console_error_panic_hook::set_once();
        init_game()
    }

    pub fn tick(&mut self, dt: f32) {
        self.g.on_tick(dt);
    }

    pub fn key_event(&mut self, t: u8, e: web_sys::Event) {
        let abase = &self
            .g
            .context
            .adapter
            .as_any()
            .downcast_ref::<WebAdapter>()
            .unwrap()
            .base;
        
        if let Some(pe) = input_events_from_web(
            t, e, 
            abase.gr.pixel_h, 
            abase.gr.ratio_x, 
            abase.gr.ratio_y
        ) {
            self.g.context.input_events.push(pe);
        }
    }
}
```

---

# 第十二讲：OpenGL 与着色器编程

## 12.1 OpenGL 基础

### 顶点着色器

```glsl
// vertex shader
#version 300 es
precision mediump float;

layout(location = 0) in vec2 position;
layout(location = 1) in vec2 tex_coords;
layout(location = 2) in vec4 color;
layout(location = 3) in vec4 bg_color;
layout(location = 4) in float tex_index;
layout(location = 5) in float symbol_index;

out vec2 v_tex_coords;
out vec4 v_color;
out vec4 v_bg_color;
flat out int v_tex_index;
flat out int v_symbol_index;

uniform mat4 u_projection;

void main() {
    gl_Position = u_projection * vec4(position, 0.0, 1.0);
    v_tex_coords = tex_coords;
    v_color = color;
    v_bg_color = bg_color;
    v_tex_index = int(tex_index);
    v_symbol_index = int(symbol_index);
}
```

### 片段着色器

```glsl
// fragment shader
#version 300 es
precision mediump float;

in vec2 v_tex_coords;
in vec4 v_color;
in vec4 v_bg_color;
flat in int v_tex_index;
flat in int v_symbol_index;

out vec4 FragColor;

uniform sampler2D u_texture;

void main() {
    // 计算符号在纹理图集中的位置
    int block_x = v_symbol_index % 128;
    int block_y = v_symbol_index / 128;
    
    vec2 block_uv = vec2(
        float(block_x) / 128.0,
        float(block_y) / 128.0
    );
    
    // 采样纹理
    vec2 final_uv = block_uv + v_tex_coords / 128.0;
    vec4 tex_color = texture(u_texture, final_uv);
    
    // 混合前景色和背景色
    vec4 final_color;
    if (tex_color.r > 0.5) {
        final_color = v_color;
    } else {
        final_color = v_bg_color;
    }
    
    FragColor = final_color;
}
```

## 12.2 Glow 使用

### 初始化 OpenGL 上下文

```rust
use glow::*;

unsafe fn init_gl(gl: &glow::Context) {
    // 编译着色器
    let vertex_shader = gl.create_shader(VERTEX_SHADER).unwrap();
    gl.shader_source(vertex_shader, VERTEX_SHADER_SOURCE);
    gl.compile_shader(vertex_shader);
    
    let fragment_shader = gl.create_shader(FRAGMENT_SHADER).unwrap();
    gl.shader_source(fragment_shader, FRAGMENT_SHADER_SOURCE);
    gl.compile_shader(fragment_shader);
    
    // 链接程序
    let program = gl.create_program().unwrap();
    gl.attach_shader(program, vertex_shader);
    gl.attach_shader(program, fragment_shader);
    gl.link_program(program);
    
    // 创建 VAO 和 VBO
    let vao = gl.create_vertex_array().unwrap();
    gl.bind_vertex_array(Some(vao));
    
    let vbo = gl.create_buffer().unwrap();
    gl.bind_buffer(ARRAY_BUFFER, Some(vbo));
    
    // 设置顶点属性
    gl.enable_vertex_attrib_array(0);
    gl.vertex_attrib_pointer_f32(0, 2, FLOAT, false, 48, 0);
    
    gl.enable_vertex_attrib_array(1);
    gl.vertex_attrib_pointer_f32(1, 2, FLOAT, false, 48, 8);
    
    // ... 更多属性设置
}
```

### 渲染循环

```rust
unsafe fn render(gl: &glow::Context, render_data: &[RenderCell]) {
    // 清屏
    gl.clear_color(0.0, 0.0, 0.0, 1.0);
    gl.clear(COLOR_BUFFER_BIT);
    
    // 使用程序
    gl.use_program(Some(program));
    
    // 上传数据
    gl.buffer_data_u8_slice(
        ARRAY_BUFFER,
        bytemuck::cast_slice(render_data),
        DYNAMIC_DRAW,
    );
    
    // 绘制
    gl.draw_arrays(TRIANGLES, 0, render_data.len() as i32);
}
```

## 12.3 RustPixel 渲染管线

### 实例化渲染

```rust
// 为每个字符创建一个实例
for cell in buffer.content.iter() {
    let render_cell = RenderCell {
        position: [x as f32, y as f32],
        tex_coords: [0.0, 0.0],
        color: cell.fg.to_rgba(),
        bg_color: cell.bg.unwrap_or_default().to_rgba(),
        tex_index: 0,
        symbol_index: cell.symbol as u32,
    };
    
    render_buffer.push(render_cell);
}

// 一次性渲染所有实例
unsafe {
    gl.draw_arrays_instanced(
        TRIANGLE_STRIP,
        0,
        4,
        render_buffer.len() as i32,
    );
}
```

---

# 第十三讲：WGPU 现代图形 API

## 13.1 WGPU 基础

### 初始化

```rust
use wgpu::*;

async fn init_wgpu(window: &winit::window::Window) -> (Device, Queue, Surface, SurfaceConfiguration) {
    // 创建实例
    let instance = Instance::new(InstanceDescriptor {
        backends: Backends::all(),
        ..Default::default()
    });
    
    // 创建 surface
    let surface = unsafe { instance.create_surface(&window) }.unwrap();
    
    // 请求适配器
    let adapter = instance
        .request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .unwrap();
    
    // 请求设备和队列
    let (device, queue) = adapter
        .request_device(
            &DeviceDescriptor {
                label: Some("RustPixel Device"),
                features: Features::empty(),
                limits: Limits::default(),
            },
            None,
        )
        .await
        .unwrap();
    
    // 配置 surface
    let size = window.inner_size();
    let config = SurfaceConfiguration {
        usage: TextureUsages::RENDER_ATTACHMENT,
        format: surface.get_capabilities(&adapter).formats[0],
        width: size.width,
        height: size.height,
        present_mode: PresentMode::Fifo,
        alpha_mode: CompositeAlphaMode::Auto,
        view_formats: vec![],
    };
    surface.configure(&device, &config);
    
    (device, queue, surface, config)
}
```

## 13.2 渲染管线

### 创建管线

```rust
fn create_render_pipeline(
    device: &Device,
    surface_format: TextureFormat,
) -> RenderPipeline {
    // 加载着色器
    let shader = device.create_shader_module(ShaderModuleDescriptor {
        label: Some("Shader"),
        source: ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
    });
    
    // 定义顶点布局
    let vertex_layout = VertexBufferLayout {
        array_stride: std::mem::size_of::<RenderCell>() as BufferAddress,
        step_mode: VertexStepMode::Vertex,
        attributes: &[
            VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: VertexFormat::Float32x2,
            },
            VertexAttribute {
                offset: 8,
                shader_location: 1,
                format: VertexFormat::Float32x2,
            },
            // ... 更多属性
        ],
    };
    
    // 创建管线
    device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[vertex_layout],
        },
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(ColorTargetState {
                format: surface_format,
                blend: Some(BlendState::ALPHA_BLENDING),
                write_mask: ColorWrites::ALL,
            })],
        }),
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: MultisampleState::default(),
        multiview: None,
    })
}
```

### WGSL 着色器

```wgsl
// shader.wgsl
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) bg_color: vec4<f32>,
    @location(4) tex_index: u32,
    @location(5) symbol_index: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) bg_color: vec4<f32>,
    @location(3) @interpolate(flat) symbol_index: u32,
}

@group(0) @binding(0)
var<uniform> projection: mat4x4<f32>;

@group(0) @binding(1)
var t_diffuse: texture_2d<f32>;

@group(0) @binding(2)
var s_diffuse: sampler;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = projection * vec4<f32>(in.position, 0.0, 1.0);
    out.tex_coords = in.tex_coords;
    out.color = in.color;
    out.bg_color = in.bg_color;
    out.symbol_index = in.symbol_index;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let block_x = in.symbol_index % 128u;
    let block_y = in.symbol_index / 128u;
    
    let block_uv = vec2<f32>(
        f32(block_x) / 128.0,
        f32(block_y) / 128.0
    );
    
    let final_uv = block_uv + in.tex_coords / 128.0;
    let tex_color = textureSample(t_diffuse, s_diffuse, final_uv);
    
    var final_color: vec4<f32>;
    if (tex_color.r > 0.5) {
        final_color = in.color;
    } else {
        final_color = in.bg_color;
    }
    
    return final_color;
}
```

## 13.3 渲染循环

```rust
fn render(&mut self) -> Result<(), SurfaceError> {
    let output = self.surface.get_current_texture()?;
    let view = output.texture.create_view(&TextureViewDescriptor::default());
    
    let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("Render Encoder"),
    });
    
    {
        let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..self.num_vertices, 0..1);
    }
    
    self.queue.submit(std::iter::once(encoder.finish()));
    output.present();
    
    Ok(())
}
```

---

# 第十四讲：UI 框架设计

## 14.1 Widget 系统

### Widget Trait

```rust
pub trait Widget {
    fn render(&self, buffer: &mut Buffer, ctx: &Context) -> UIResult<()>;
    fn handle_event(&mut self, event: &UIEvent, ctx: &mut Context) -> UIResult<bool>;
    fn update(&mut self, dt: f32, ctx: &mut Context) -> UIResult<()>;
    
    fn get_bounds(&self) -> Rect;
    fn set_bounds(&mut self, bounds: Rect);
    fn is_focused(&self) -> bool;
    fn set_focused(&mut self, focused: bool);
    fn is_visible(&self) -> bool;
    fn set_visible(&mut self, visible: bool);
}
```

### 基础组件实现

```rust
pub struct Button {
    base: BaseWidget,
    text: String,
    on_click: Option<Box<dyn FnMut()>>,
    style: ButtonStyle,
}

impl Button {
    pub fn new(text: &str) -> Self {
        Self {
            base: BaseWidget::new(),
            text: text.to_string(),
            on_click: None,
            style: ButtonStyle::Normal,
        }
    }
    
    pub fn on_click<F>(mut self, f: F) -> Self
    where
        F: FnMut() + 'static,
    {
        self.on_click = Some(Box::new(f));
        self
    }
}

impl Widget for Button {
    fn render(&self, buffer: &mut Buffer, ctx: &Context) -> UIResult<()> {
        let bounds = self.get_bounds();
        let style = ctx.theme.get_style("button", self.base.state);
        
        // 绘制背景
        for y in bounds.y..(bounds.y + bounds.height) {
            for x in bounds.x..(bounds.x + bounds.width) {
                buffer.set(x, y, Cell::new(' ').with_style(style));
            }
        }
        
        // 绘制文本
        let text_x = bounds.x + (bounds.width - self.text.len() as u16) / 2;
        let text_y = bounds.y + bounds.height / 2;
        
        for (i, c) in self.text.chars().enumerate() {
            buffer.set(text_x + i as u16, text_y, Cell::new(c).with_style(style));
        }
        
        Ok(())
    }
    
    fn handle_event(&mut self, event: &UIEvent, ctx: &mut Context) -> UIResult<bool> {
        match event {
            UIEvent::Input(InputEvent::Mouse(MouseEvent::Press(_, x, y))) => {
                if self.get_bounds().contains(*x, *y) {
                    if let Some(ref mut callback) = self.on_click {
                        callback();
                        return Ok(true);
                    }
                }
            }
            _ => {}
        }
        Ok(false)
    }
    
    fn update(&mut self, dt: f32, ctx: &mut Context) -> UIResult<()> {
        Ok(())
    }
    
    // ... 实现其他方法
}
```

## 14.2 布局系统

### 线性布局

```rust
pub struct LinearLayout {
    direction: Direction,
    spacing: u16,
    padding: Padding,
}

pub enum Direction {
    Horizontal,
    Vertical,
}

impl LinearLayout {
    pub fn vertical() -> Self {
        Self {
            direction: Direction::Vertical,
            spacing: 0,
            padding: Padding::default(),
        }
    }
    
    pub fn with_spacing(mut self, spacing: u16) -> Self {
        self.spacing = spacing;
        self
    }
    
    pub fn layout(&self, children: &mut [Box<dyn Widget>], bounds: Rect) {
        let mut current_y = bounds.y + self.padding.top;
        
        for child in children {
            let child_height = child.get_bounds().height;
            let child_bounds = Rect::new(
                bounds.x + self.padding.left,
                current_y,
                bounds.width - self.padding.left - self.padding.right,
                child_height,
            );
            
            child.set_bounds(child_bounds);
            current_y += child_height + self.spacing;
        }
    }
}
```

### 网格布局

```rust
pub struct GridLayout {
    rows: u16,
    cols: u16,
    gap: u16,
}

impl GridLayout {
    pub fn new(rows: u16, cols: u16) -> Self {
        Self { rows, cols, gap: 0 }
    }
    
    pub fn layout(&self, children: &mut [Box<dyn Widget>], bounds: Rect) {
        let cell_width = (bounds.width - (self.cols - 1) * self.gap) / self.cols;
        let cell_height = (bounds.height - (self.rows - 1) * self.gap) / self.rows;
        
        for (i, child) in children.iter_mut().enumerate() {
            let row = i as u16 / self.cols;
            let col = i as u16 % self.cols;
            
            let x = bounds.x + col * (cell_width + self.gap);
            let y = bounds.y + row * (cell_height + self.gap);
            
            child.set_bounds(Rect::new(x, y, cell_width, cell_height));
        }
    }
}
```

## 14.3 事件处理

### 事件分发

```rust
pub struct EventDispatcher {
    focused_widget: Option<usize>,
}

impl EventDispatcher {
    pub fn dispatch(
        &mut self,
        event: &UIEvent,
        widgets: &mut [Box<dyn Widget>],
        ctx: &mut Context,
    ) -> UIResult<bool> {
        // 先尝试给焦点组件发送事件
        if let Some(index) = self.focused_widget {
            if widgets[index].handle_event(event, ctx)? {
                return Ok(true);
            }
        }
        
        // 从后向前遍历（绘制顺序的反向）
        for (i, widget) in widgets.iter_mut().enumerate().rev() {
            if !widget.is_visible() {
                continue;
            }
            
            if widget.handle_event(event, ctx)? {
                self.focused_widget = Some(i);
                return Ok(true);
            }
        }
        
        Ok(false)
    }
}
```

---

# 第十五讲：性能优化技巧

## 15.1 性能分析工具

### Criterion 基准测试

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("fib 20", |b| b.iter(|| fibonacci(black_box(20))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
```

### Flamegraph 火焰图

```bash
cargo install flamegraph
cargo flamegraph --bin snake
```

## 15.2 优化技巧

### 1. 使用 `&str` 而非 `String`

```rust
// 慢
fn process(s: String) { }

// 快
fn process(s: &str) { }
```

### 2. 避免不必要的克隆

```rust
// 慢
let v2 = v1.clone();
process(v2);

// 快
process(&v1);
```

### 3. 使用迭代器链

```rust
// 慢
let mut result = vec![];
for x in data {
    if x > 0 {
        result.push(x * 2);
    }
}

// 快
let result: Vec<_> = data
    .iter()
    .filter(|&&x| x > 0)
    .map(|&x| x * 2)
    .collect();
```

### 4. 缓存计算结果

```rust
struct GameState {
    cells: Vec<Cell>,
    // 缓存已计算的结果
    cached_score: Option<u32>,
}

impl GameState {
    fn get_score(&mut self) -> u32 {
        if let Some(score) = self.cached_score {
            return score;
        }
        
        let score = self.calculate_score();
        self.cached_score = Some(score);
        score
    }
    
    fn update(&mut self) {
        // 状态变化时清除缓存
        self.cached_score = None;
    }
}
```

### 5. 对象池

RustPixel 中的对象池实现：

```rust
pub struct ObjPool<T> {
    objects: Vec<T>,
    available: Vec<usize>,
}

impl<T> ObjPool<T> {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            available: Vec::new(),
        }
    }
    
    pub fn get(&mut self) -> Option<&mut T>
    where
        T: Default,
    {
        if let Some(index) = self.available.pop() {
            Some(&mut self.objects[index])
        } else {
            self.objects.push(T::default());
            Some(self.objects.last_mut().unwrap())
        }
    }
    
    pub fn release(&mut self, index: usize) {
        self.available.push(index);
    }
}
```

### 6. 使用 SmallVec

```rust
use smallvec::SmallVec;

// 小数组避免堆分配
let mut vec: SmallVec<[i32; 8]> = SmallVec::new();
vec.push(1);
vec.push(2);
```

## 15.3 编译优化

### Cargo.toml 配置

```toml
[profile.release]
opt-level = 3          # 最高优化级别
lto = "fat"            # 链接时优化
codegen-units = 1      # 单个编译单元（更好的优化）
strip = true           # 移除符号表
panic = "abort"        # panic 时直接中止

# 体积优化
[profile.release-small]
inherits = "release"
opt-level = "z"        # 优化体积
lto = "fat"
codegen-units = 1
strip = true
```

### 条件编译优化

```rust
// 调试模式下的额外检查
#[cfg(debug_assertions)]
fn validate_state(&self) {
    assert!(self.x >= 0);
    assert!(self.y >= 0);
}

#[cfg(not(debug_assertions))]
#[inline(always)]
fn validate_state(&self) { }
```

---

# 第十六讲：实战项目解析

## 16.1 Snake 游戏

### 游戏状态

```rust
pub struct SnakeModel {
    snake: VecDeque<Point>,
    direction: Direction,
    food: Point,
    score: u32,
    game_over: bool,
}

enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Model for SnakeModel {
    fn init(&mut self, ctx: &mut Context) {
        self.snake = VecDeque::new();
        self.snake.push_back(Point::new(10, 10));
        self.spawn_food();
    }
    
    fn handle_input(&mut self, ctx: &mut Context, dt: f32) {
        for event in &ctx.input_events {
            match event {
                Event::Key(KeyCode::Up) => self.direction = Direction::Up,
                Event::Key(KeyCode::Down) => self.direction = Direction::Down,
                Event::Key(KeyCode::Left) => self.direction = Direction::Left,
                Event::Key(KeyCode::Right) => self.direction = Direction::Right,
                _ => {}
            }
        }
    }
    
    fn handle_timer(&mut self, ctx: &mut Context, dt: f32) {
        if event_check("move", "snake") {
            self.move_snake();
            self.check_collision();
        }
    }
}
```

## 16.2 Tetris 游戏

### 方块系统

```rust
pub struct Tetromino {
    shape: [[bool; 4]; 4],
    color: Color,
    x: i32,
    y: i32,
}

impl Tetromino {
    pub fn new(kind: TetrominoKind) -> Self {
        let (shape, color) = match kind {
            TetrominoKind::I => (
                [
                    [false, false, false, false],
                    [true, true, true, true],
                    [false, false, false, false],
                    [false, false, false, false],
                ],
                Color::Cyan,
            ),
            // ... 其他形状
        };
        
        Self { shape, color, x: 0, y: 0 }
    }
    
    pub fn rotate(&mut self) {
        let mut new_shape = [[false; 4]; 4];
        for i in 0..4 {
            for j in 0..4 {
                new_shape[j][3 - i] = self.shape[i][j];
            }
        }
        self.shape = new_shape;
    }
}

pub struct TetrisModel {
    current: Tetromino,
    board: [[Option<Color>; 10]; 20],
    score: u32,
}
```

## 16.3 Poker 算法

### 牌型评估

```rust
#[derive(Debug, PartialOrd, Ord, PartialEq, Eq)]
pub enum HandRank {
    HighCard = 0,
    OnePair = 1,
    TwoPair = 2,
    ThreeOfAKind = 3,
    Straight = 4,
    Flush = 5,
    FullHouse = 6,
    FourOfAKind = 7,
    StraightFlush = 8,
    RoyalFlush = 9,
}

pub fn evaluate_hand(cards: &[Card]) -> (HandRank, Vec<u8>) {
    let mut ranks = [0u8; 15];
    let mut suits = [0u8; 4];
    
    // 统计
    for card in cards {
        ranks[card.rank as usize] += 1;
        suits[card.suit as usize] += 1;
    }
    
    // 判断牌型
    let is_flush = suits.iter().any(|&count| count >= 5);
    let is_straight = check_straight(&ranks);
    
    if is_straight && is_flush {
        return (HandRank::StraightFlush, vec![]);
    }
    
    // ... 其他判断
}
```

---

# 总结与展望

## 学到的技术栈

通过 RustPixel 项目，我们学习了：

1. **Rust 基础**：所有权、借用、生命周期、trait、泛型
2. **系统架构**：MVC 模式、适配器模式、事件驱动
3. **构建工具**：Cargo、workspace、条件编译、特性门控
4. **图形编程**：OpenGL、WGPU、着色器、实例化渲染
5. **跨平台开发**：Terminal、SDL、Web、多后端支持
6. **FFI**：C 语言互操作、cbindgen、动态库
7. **WebAssembly**：wasm-bindgen、浏览器部署
8. **并发编程**：线程、Arc、Mutex、async/await
9. **性能优化**：基准测试、火焰图、对象池、缓存
10. **实战项目**：游戏引擎、UI 框架、算法库

## 进阶学习方向

1. **深入 Unsafe Rust**：FFI、内存操作、性能关键代码
2. **宏系统进阶**：过程宏、派生宏、领域特定语言
3. **嵌入式开发**：no_std、裸机编程
4. **网络编程**：tokio、async-std、协议实现
5. **数据库**：diesel、sqlx、ORM 设计
6. **分布式系统**：共识算法、分布式存储
7. **编译器**：LLVM、JIT、DSL 设计

## 资源推荐

### 官方资源
- [The Rust Programming Language Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [The Cargo Book](https://doc.rust-lang.org/cargo/)

### 进阶书籍
- Programming Rust (O'Reilly)
- Rust for Rustaceans (No Starch Press)
- Hands-On Rust (Pragmatic Bookshelf)

### 项目实践
- RustPixel: https://github.com/zipxing/rust_pixel
- 参与开源项目贡献
- 构建自己的游戏/工具

---

感谢学习！欢迎继续探索 Rust 生态系统！


---

# 第十七讲：资源与资产系统（AssetManager）

## 17.1 设计目标与职责

- **统一管理**: 通过 `AssetManager` 管理图片与序列帧等资源的加载、解析与复用。
- **跨平台**: 桌面/终端读取文件；Web 端通过 JS 异步加载后回调。
- **一次解析，多次复用**: 原始数据只解析一次，解析结果缓存为 `Buffer` 序列，供 `Sprite`/渲染直接使用。

## 17.2 核心结构

```rust
// 资产类型与状态
#[derive(PartialEq, Clone, Copy)]
pub enum AssetType { ImgPix, ImgEsc, ImgSsf }

#[derive(PartialEq, Clone, Copy)]
pub enum AssetState { Loading, Parsing, Ready }

// 资产基类，缓存原始数据与解析后的帧缓存
pub struct AssetBase {
    pub location: String,
    pub asset_type: AssetType,
    pub raw_data: Vec<u8>,
    pub parsed_buffers: Vec<Buffer>,
    pub frame_count: usize,
    pub state: AssetState,
}

// 资产 trait，负责设置数据、解析与保存
pub trait Asset {
    fn new(ab: AssetBase) -> Self where Self: Sized;
    fn get_base(&mut self) -> &mut AssetBase;
    fn set_data(&mut self, data: &[u8]);
    fn parse(&mut self);
    fn save(&mut self, buf: &Buffer);
}

// 资产管理器：去重、索引、统一 set_data/parse 流程
pub struct AssetManager {
    pub assets: Vec<Box<dyn Asset>>,
    pub assets_index: HashMap<String, usize>,
}
```

要点：
- **桌面/终端**: 直接读取文件，随后 `set_data -> parse -> Ready`。
- **Web (WASM)**: 通过 `js_load_asset(url)` 异步拉取，拉取完成后回调 `set_data` 并 `parse`。
- `PixAsset`/`EscAsset`/`SeqFrameAsset` 分别对应 `.pix`、`.txt(ESC)`、`.ssf` 序列帧。

## 17.3 使用范式

```rust
// 加载并获取资源
ctx.asset_manager.load(AssetType::ImgPix, "assets/logo.pix");
if let Some(ast) = ctx.asset_manager.get("assets/logo.pix") {
    // 将某一帧拷贝到 sprite 内容
    ast.set_sprite(&mut sprite, 0, 0, 0);
}
```

实践建议：
- 资源路径统一从 `Context.project_path` 解析，便于独立项目与工作区切换。
- `.ssf` 序列帧适合做过场动画、火焰、粒子等低门槛特效。


---

# 第十八讲：Context 与渲染后端选择（高 DPI 与坐标）

## 18.1 `Context` 的作用

- 运行时上下文：`game_name`、`project_path`、`stage/state`、`rand`、`asset_manager`、`input_events`、`adapter`。
- 初始化时根据 feature/平台选择具体后端：`CrosstermAdapter`、`SdlAdapter`、`WinitGlowAdapter`、`WinitWgpuAdapter`、`WebAdapter`。

```rust
pub struct Context {
    pub game_name: String,
    pub project_path: String,
    pub stage: u32,
    pub state: u8,
    pub rand: Rand,
    pub asset_manager: AssetManager,
    pub input_events: Vec<Event>,
    pub adapter: Box<dyn Adapter>,
}
```

## 18.2 高 DPI 尺寸与单元尺寸

- 图形模式下，后端维护 `Graph`，包含 `ratio_x/ratio_y`、`pixel_w/pixel_h` 等。
- `Context::cell_width/cell_height` 提供逻辑单元（字符/符号）对应的像素尺寸，便于精确布局与特效计算。

实战技巧：
- 布局与碰撞尽量基于“格子（cell）坐标”，需要像素级特效再乘以 `ratio`；
- 窗口缩放只影响 `ratio` 与像素画布，逻辑尺寸保持稳定，避免 UI 抖动。


---

# 第十九讲：构建脚本与 cfg_aliases（多后端统一开关）

## 19.1 统一别名

`build_support.rs` 定义了跨 crate 复用的 `cfg_aliases`：

```rust
cfg_aliases! {
    wasm: { target_arch = "wasm32" },
    mobile: { any(target_os = "android", target_os = "ios") },
    graphics_backend: { any(feature = "sdl", feature = "winit", feature = "wgpu") },
    sdl_backend: { all(feature = "sdl", not(wasm)) },
    winit_backend: { all(feature = "winit", not(wasm), not(feature = "wgpu")) },
    wgpu_backend: { all(feature = "wgpu", not(wasm)) },
    graphics_mode: { any(graphics_backend, wasm) },
    cross_backend: { not(graphics_mode) },
    audio_support: { not(any(mobile, wasm)) },
}
```

## 19.2 使用方式

- 主工程与各 `apps/*` 的 `build.rs` 统一 `include!("build_support.rs")` 并调用 `setup_rust_pixel_cfg_aliases()`。
- 代码中用 `#[cfg(sdl_backend)]`/`#[cfg(graphics_mode)]` 精准开关模块，避免复杂的条件表达式散落全工程。

收益：
- 清晰的“渲染模式 vs 终端模式”边界；
- 后端切换只需变更 feature 组合，减少分支编译歧义。


---

# 第二十讲：cargo-pixel CLI 工具（工程化一键脚手架）

## 20.1 功能概览

- `run/build`：按后端特性运行/编译 `apps/*` 或独立工程。
- `creat`：基于 `apps/template` 创建应用或独立工程（自动替换标识符、复制模板、写入配置）。
- 工具链：`asset`（图集打包）、`edit`（字符/像素编辑器）、`petii`（图片转 PETSCII）、`ssf`（序列帧播放）、`symbol`（符号提取）、`ttf`（字体转图集）。

## 20.2 常用命令

```bash
# 运行示例
cargo pixel r snake t        # 终端
cargo pixel r tetris s       # SDL/OpenGL
cargo pixel r petview wg -r  # WGPU
cargo pixel r tower w -r     # WASM（配本地静态服务）

# 创建项目
cargo pixel c mygame                 # 在 apps/ 下创建
cargo pixel c myapp ..               # 创建独立工程 ../myapp

# 资源与工具
cargo pixel asset ./sprites ./out    # 打包纹理图集
cargo pixel e t . assets/logo.txt    # 终端编辑器
cargo pixel p assets/a.png 40 25     # 图片转 PETSCII
cargo pixel sf t . assets/x.ssf      # 播放 ssf 序列帧
cargo pixel sy assets/c64.png 8      # 提取 8x8 符号
cargo pixel tf font.ttf atlas.png 8  # TTF 转字符图集
```

## 20.3 内部实现要点

- `prepare_env` 自动在合适位置克隆/定位 `rust_pixel` 仓库，并将路径写入配置（跨平台）。
- `command.rs` 用 `clap` 构建统一的命令解析器与别名（r/sf/sy/tt 等）。
- `build_run.rs` 将 `run/build` 解析为 cargo 命令或 wasm-pack 调用；Web 模式自动准备 `web-templates` 与 `pkg` 并启本地服务。
- `creat.rs` 复制模板并批量替换 `Template/TEMPLATE/template` 标识符，支持独立工程注入 `build_support.rs`。

实践建议：
- 面向讲座演示，优先用 `cargo pixel` 统一入口，降低环境差异；
- 需要发布 Web Demo 时，`cargo pixel r <app> w -r -p 8080` 一条龙生成并本地预览。
