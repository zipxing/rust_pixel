我想对底层资源纹理系统进行重构，主要目的如下：
- 目的：sdf在渲染时质量不高，还不如缩放比例较小的时候直接渲染
- 目的：目前静态8192大图直接加载进内存，浪费比较多，希望能够按需加载

实现要点：
- 采用mipmap技术，对于sprite符号，采用32x32，16x16两个级别，对于TUIChar，盲文，emoji，中文等TUI字符,采用64，32，16三级
- 采用Texture2DArray技术，加载多幅2048x2048的大图(Layer)，但仍然一次drawcall保持高性能
- 上述各种级别的各种符号，全部塞入TextureLayer，根据app使用的符号，动态组织和加载
- 各种概念命名是否需要统一：
  Cell（包含symbol字符串用于确定是什么符号，glyph用于定位纹理中的位置）
  Cell要不要改名为Tile，符合引擎tile优先的概念
  symbol改名tile_symbol_string
  glyph改名为tile_texture_info
- tools/symbols工具（最新rust版本在cargo-pixel下）,负责把各种级别的图片准备好，合理的拼接到2048Layers里，并生成对应的map json
- 主库里，根据当前渲染符号的实际需要（真实物理大小）判断应该使用哪个级别的资源，然后由shader在Texture2DArray中选择，渲染
- 这是一个比较大的重构，所以需要使用openspec创建一个changes，然后进行详细的设计，生成好各种文档，确认后再推进tasks！

  


