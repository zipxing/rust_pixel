# 渲染系统规范增量

## ADDED Requirements

### Requirement: 动态字体光栅化
系统应当(SHALL)支持运行时动态生成字符纹理,使用 TTF/OTF 字体文件。

#### Scenario: 首次渲染 CJK 字符
- **WHEN** 图形模式下首次渲染一个中文字符 "中"
- **THEN** 系统使用 fontdue 库光栅化字形生成位图
- **THEN** 将位图上传到 GPU 纹理并缓存纹理 ID
- **THEN** 渲染时间应在 0.1ms 以内

#### Scenario: 渲染已缓存字符
- **WHEN** 渲染之前已经缓存过的字符
- **THEN** 系统直接使用缓存的纹理 ID
- **THEN** 不产生额外的光栅化开销

#### Scenario: LRU 缓存满时驱逐
- **WHEN** 字形缓存达到上限(如 1000 个字符)
- **WHEN** 需要渲染新字符
- **THEN** 系统驱逐最久未使用的字形纹理
- **THEN** 为新字符生成纹理

### Requirement: 混合渲染模式
系统应当(SHALL)同时支持静态纹理图集和动态字体渲染,根据字符类型自动选择。

#### Scenario: ASCII 字符使用静态图集
- **WHEN** 渲染 ASCII 字符 'A'
- **THEN** 系统使用预制的静态纹理图集
- **THEN** 不进行动态光栅化

#### Scenario: Emoji 使用静态图集
- **WHEN** 渲染 Emoji 字符 "😀"
- **THEN** 系统使用预制的 Emoji 纹理图集
- **THEN** 不进行动态光栅化

#### Scenario: CJK 字符使用动态渲染
- **WHEN** 渲染中文字符 "你好世界"
- **THEN** 系统对每个字符使用动态字体光栅化
- **THEN** 结果被缓存供后续使用

### Requirement: 字体资源管理
系统应当(SHALL)提供字体加载、卸载和切换的接口。

#### Scenario: 加载 TTF 字体文件
- **WHEN** 应用调用 `AssetManager::load_font("my_font.ttf")`
- **THEN** 系统解析 TTF 文件并创建 fontdue::Font 对象
- **THEN** 返回字体句柄供后续使用

#### Scenario: 切换默认字体
- **WHEN** 应用设置新的默认字体
- **THEN** 系统清空当前字形缓存
- **THEN** 后续文本渲染使用新字体

#### Scenario: 多字体支持
- **WHEN** 应用需要同时使用多种字体(如代码字体和UI字体)
- **THEN** 系统允许为不同的 Panel 或 Sprite 指定不同字体
- **THEN** 每个字体维护独立的字形缓存

### Requirement: 性能优化
系统应当(SHALL)实现性能优化策略以减少运行时开销。

#### Scenario: 启动时预加载常用字符
- **WHEN** 系统初始化时
- **THEN** 预加载 ASCII、常用标点和高频中文字符(如前 500 个)
- **THEN** 这些字符在首次使用时无需光栅化

#### Scenario: 批量纹理上传
- **WHEN** 同一帧内需要渲染多个新字符
- **THEN** 系统批量生成字形位图
- **THEN** 一次性上传到 GPU 以减少绘制调用

### Requirement: Emoji 扩展接口
系统应当(SHALL)预留接口供未来支持动态彩色 Emoji 渲染。

#### Scenario: 静态 Emoji 作为当前实现
- **WHEN** 当前版本渲染 Emoji
- **THEN** 使用静态纹理图集
- **THEN** 质量和性能满足基本需求

#### Scenario: 未来升级到动态 Emoji
- **WHEN** 未来版本需要支持高质量彩色 Emoji
- **THEN** `GlyphRenderer` 的接口设计允许添加彩色字形支持
- **THEN** 可以无缝升级到 COLR/SBIX 或基于 swash 的方案
- **THEN** 现有应用代码无需修改
