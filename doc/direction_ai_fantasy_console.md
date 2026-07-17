# 方向探索：AI × 终端幻想机（AI Fantasy Console）

> 决策 / 设计记录 · 2026-07-15 · 状态：**方向假设已验证地基，待接引擎**
>
> 一句话：把 RustPixel 定位成 **"一句话 → 可玩卡带"** 的终端幻想机——用户用自然语言描述，
> LLM 生成受约束的 tile 卡带，引擎在 终端/窗口/Web 上带 shader 渲染。本文档沉淀这个方向
> 是怎么推导出来的、验证到什么程度、以及美术管线和下一步。

---

## 0. TL;DR（先给结论）

- **方向**：AI × 幻想机。用户输入一句话 → LLM 生成一张跑在字符网格上的小游戏"卡带" → 引擎渲染。
  传播机制 = **分享 prompt**（Midjourney 式），卡带作为冻结产物 + web 链接分享。
- **护城河不是"AI 会写代码"**（任何 LLM 都会），而是
  **[极小受约束运行时] × [生成-运行-自修复闭环] × [RustPixel 独有的 GPU shader/tile 视觉层] × [一套代码跑终端/窗口/Web]**。
- **卡带语言 = Lua，不是 pixel_basic**。自研 BASIC 方言两头得罪：对 LLM 是训练分布外（错误率高），
  对人是零生态（改不动、搜不到）。Lua 主流 + 可沙箱 + PICO-8/TIC-80 已验证。
- **地基已用 spike 验证**（见 §4）：一句话 → 可运行、会动、响应输入的卡带，**在前沿模型上 10/10**，
  自修复循环 4/4 收敛。**但这是"天花板"，不是部署数字**——小模型 + Lua-vs-BASIC 的真实 A/B 还没做。
- **美术**：对 tile 引擎，美术单元是"网格上的 tile"，**不要一上来接 image2/扩散模型**。
  正确顺序：Tier 0 用真 tile（无 AI）→ Tier 1 从 **2487 张现成 `.pix`** 检索 → Tier 2 受约束生成 →
  Tier 3 才用 image2 当"素材源"喂给 `tools/petii` 量化成 tile。

---

## 1. 为什么是这个方向（推导过程）

问题起点：RustPixel 摊得太开（17 个 app、3 条并行支柱都半程），缺一个能被更多人接受的拳头方向。
逐个评估竞争格局后**否掉**了几条：

| 候选方向 | 直接对手 | 结论 |
|---|---|---|
| 通用 2D 游戏引擎 | Bevy / macroquad / ggez | ❌ 硬碰硬打不过 |
| 终端风 Markdown 演示（MDPT） | presenterm / slides / marp | ❌ 赛道无热度，逆着热情走 |
| 终端幻想机（当游戏平台） | PICO-8 / TIC-80 | ⚠️ 终端输入/表现力太差（无 keyup、无逐像素），PICO/TIC 刻意避开终端是对的 |
| 纯 shader 视觉奇观 + 图形博客 | demoscene / shadertoy | ✅ 对得上作者热情，但受众小 |
| **AI × 幻想机** | 无干净对手 | ✅ **最强**：踩 AI 风口 + 复用闲置的脚本层 + 有真实技术护城河 |

**关键个人背景**：作者做 RustPixel 的初衷是**学 Rust + 打磨 shader/渲染**，不是做游戏或产品。
所以方向选择要满足"作者愿意长期做" + "有真空地带"。AI × 幻想机同时满足，且能把 shader 热情接回来
（生成的卡带自带 GPU shader 视觉层，是别人给不了的差异化）。

**为什么"AI × 幻想机"成立而不是空想**：幻想机的"自我设限"（小 API、固定网格、有限调色板）——
对人是激发创意，**对 LLM 是大幅降低出错率**。API 越小，模型越不可能写错。这是非显然的协同点，
也是整个方向的技术支点。

---

## 2. 产品形态与传播回路

- **输入**：一句话（"贪吃蛇，但苹果被吃掉时会爆炸"）。
- **输出**：一张 Lua 卡带（`init/update/draw` + tile 绘制），能在终端/窗口/Web 跑。
- **传播回路 = 分享 prompt**：
  - prompt 是最好的社交货币——小、好笑、能"再加一句"改写（one-up）。类比 Midjourney 的 prompt 文化。
  - 分享的是**卡带（冻结产物）+ 一键网页可玩链接**；prompt 挂旁边当谈资 + "改一个词再生成"的种子。
  - 乐趣不在游戏多好，而在"我这句话能变出什么"的惊喜（像抽卡）——平庸反而无所谓。
- **RustPixel 的视觉签名**：生成的卡带带 GPU shader 转场/后处理，信息流里一眼不同于干巴巴的像素 GIF。

---

## 3. 语言决策：Lua，不是 pixel_basic

这是"该不该继续投 pixel_basic"这个老问题的答案。

- **对 LLM**："更强的语言"是陷阱。要的是**主流 + 受限**：模型熟（Lua/Python/JS 训练数据多）+ 极小 API。
  完整 Python + 一堆库反而错误方式更多、更难沙箱、输出更长。
- **对人**：主编辑面应是**prompt（自然语言）**，非专业用户根本不看代码。想抠代码的 tinkerer 也需要
  "可读 + 能 google"——自研 BASIC 方言零生态，搜不到帮助。
- **pixel_basic 唯一优点**（天然沙箱）被**嵌入式 mlua（砍掉 io/os）**同样满足。
- **结论**：Lua。PICO-8/TIC-80 都选它、LLM 写得好、能干净沙箱、对小白也够读。
  pixel_basic 大概率是错的运行时（抛开感情：作者真正下功夫的是 shader，不是这个解释器）。

---

## 4. 可行性 Spike：`spike/cartgen/`

**目的**：验证整个方向的 load-bearing 假设——"一句话 → 真能跑、会动、响应输入的卡带"是否可靠，
以及失败时的报错能否支撑自修复。

**隔离性**：`spike/cartgen/` 是**独立 crate**（自带 `[workspace]`），**不影响主 `rust_pixel` build**，未 commit。

### 4.1 做了什么
- **headless Lua 卡带运行器**（`src/main.rs`，~330 行，mlua + vendored Lua 5.4）：
  - 极小 tile API：`cls / plot / text / key / rnd` + 全局 `W,H`，画到 48×24 字符网格。
  - **确定性无头运行**（固定 RNG 种子 + 150 帧脚本化输入），沙箱化（剥离 `os/io/require/文件`），
    指令预算防死循环。
  - 输出**客观 JSON 判定**，区分"能跑"和"是游戏"：`parse_ok / runtime_ok / error+行号 /
    max_filled_cells / distinct_frames（动画）/ responded_to_input（对比无输入控制组）/ RUNNABLE`。
- **契约文档** `CART_API.md`（喂给生成模型的规范）。
- 一键复现 `run_all.sh`。

### 4.2 结果

**生成（10 个 prompt，易→难）**：贪吃蛇(爆炸苹果)、打墙 pong、太空射击、接星星、迷宫逃鬼、
青蛙过河、打砖块、俄罗斯方块、躲落石、跑酷跳跃。

> **10 / 10 首次生成即 RUNNABLE**，全部响应输入、全部动画，快照确认是真游戏。

**自修复循环（4 个真实翻车场景）**：

| 故障 | 运行器报错（带行号） | 修好轮数 |
|---|---|---|
| PICO-8 肌肉记忆 (`btn`/`pset`) | `nil value (global 'btn')` @行4 | 1 |
| 未初始化表字段 | `nil value (field 'player')` @行3 | 2 |
| 按键名没加引号 | `bad argument… nil to String, in 'key'` | 1 |
| `while true` 死循环 | `instruction budget exceeded` | 1 |

> 4/4 在 ≤2 轮内修好。第 2 个尤其说明问题：修完不报错了，但运行器靠 `distinct_frames=1`
> 判出"能跑但不是游戏"——**这种语义反馈是编译器给不了的**，正是产品要的闭环。

### 4.3 证明了什么 / 没证明什么
- ✅ **小 API 让 AI 生成变可靠**（测出来了，不只是论证）。
- ✅ **引擎能当验证器**：无头 + 确定性 + "能跑 vs 是游戏"判定。
- ✅ **自修复循环真实且廉价**，报错行号精确。
- ⚠️ **10/10 是前沿模型天花板，不是部署数字**（是"我"盲生成的）。
- ⚠️ **Lua vs pixel_basic 没实测**：前沿模型对两者都行，A/B 只有在**小模型**上才暴露方言惩罚——
  那才是要跑的实验（本 spike 的运行器就是现成 harness）。
- ⚠️ **RUNNABLE ≠ 好玩**，好玩需人/LLM-judge 评。

---

## 5. 可玩 Demo（Arcade）：`spike/cartgen/arcade.html`

把 10 个卡带**忠实移植成 JS**，用浏览器 canvas 做成可玩街机页面（每张卡带展示"生成它的那句话"）。
已发布为 artifact，浏览器可键盘直接玩（有真 keydown/keyup——终端给不了游戏的东西）。

**诚实的分层（重要）**：目前**没有任何一层用到 RustPixel 引擎**——
- Lua 卡带 = 纯逻辑；
- `cartgen` 运行器 = 无头假网格（`Vec<char>`）；
- `arcade.html` = JS 手工移植 + canvas。

Spike 刻意只验证"一句话 → 能跑的游戏逻辑"，**接引擎是下一步**。arcade 只为"看得见"。

---

## 6. 美术方向（关键澄清）

**对 tile 引擎，美术单元是"网格上的 tile"，不是自由图片。** 由此：

- **扩散模型（image2）输出掉不进字符网格**：必须量化/调色成 cell——这正是已有的 `tools/petii`
  （图片→PETSCII）。且扩散模型做不好真正的逐像素栅格画。**不要一上来接 image2。**
- **arcade 里"美术太简单"一大半是偷懒用了 ASCII**（`@ O #` + 单色）。引擎能画真 PETSCII tile
  （8×8 位图 + 16 色，来自 atlas）。已在 arcade 的 `SHOOTER ★` 里用 Unicode 块字符**近似**演示：
  飞船 3 格 sprite + 圆块陨石 + 光束子弹 + 视差星空 + 爆炸帧——逻辑一行没改，只换 `draw()`。

**分层管线（便宜可靠 → 野心大）**：

| Tier | 做法 | 可靠性 | 复用资产 |
|---|---|---|---|
| 0 · 无 AI | 卡带用真 PETSCII tile + 多格 sprite + 全 16 色 | 100% | atlas / `.pix` |
| 1 · **检索（推荐先做）** | AI 按自然语言从 **2487 张 `.pix`** 挑 tile/sprite | 高 | PETSCII 库 + tags |
| 2 · 受约束生成 | AI 吐小 tile 矩阵（N×N cell + 调色板）= 生成 `.pix` | 中 | atlas 格式 |
| 3 · image2 → petii | 扩散模型出参考图 → `tools/petii` 量化成 tile | 低/实验 | image2 当**素材源**，绝不当渲染输出 |

**设计不变量**（和逻辑生成同一条）：**AI 只输出受约束引用（tile/symbol id + 调色板），引擎负责渲染。**
"飞船用 sprite #1837" 可校验；"这是一张 PNG" 不可校验、不可网格化。

Tier 1 正好是 roadmap 的 **P0#3"AI PETSCII：先检索后生成"**——判断是对的，且检索天然统一风格，
绕开了生成方向最难的"风格漂移/动画帧不一致"。

**注**：`.pix` 已核实 **2487 个**；`tools/petii`、`tools/symbols` 存在。

---

## 7. 开放问题 / 未验证

1. **小模型首次成功率**：10/10 是前沿天花板，真实指标要用小/中/大多档模型（API）测。
2. **Lua vs pixel_basic 真实 A/B**：只有小模型才暴露方言惩罚。运行器已是现成 harness。
3. **"好玩"≠ RUNNABLE**：需人/LLM-judge 的可玩性评分。
4. **美术一致性（Tier 2/3）**：生成 tile 的风格统一、多帧动画连贯、调色板协调——最难、最没谱，
   检索（Tier 1）恰好躲开这些。
5. **卡带 × 引擎渲染尚未打通**：整条链路的"引擎真版"还没做。

---

## 8. 下一步（有序）

1. **接引擎（P0）**：建 `apps/cartplay` —— 把 Lua 卡带的 `plot/text/cls` 从"假网格"改成写
   RustPixel 的 `Buffer/Cell`（可复用 `pixel_basic` 的 `PixelGameContext`/`DrawCommand` 模型），
   `cargo pixel r cartplay g` 在窗口跑，`… t` 掉进终端跑。**这才算证明"卡带 × 引擎 × 一套代码到处跑"。**
2. **Tier-0 美术真版**：让 `cartplay` 用真 `.pix` atlas 渲染太空射击，对比浏览器近似，校准真 PETSCII 质感。
3. **真实多模型 A/B（P0）**：复用 `cartgen` 运行器，小/中/大模型 × (Lua vs pixel_basic) × 30–50 prompt，
   自动 生成→跑→≤3 轮修复，指标：首轮 RUNNABLE% / 修复后% / 轮数 / LLM-judge 好玩分。
   **决策规则**：小模型 Lua 首轮健康 + 修复能兜底 → 方向成立、Lua 定案；BASIC 大幅落后 → pixel_basic 出局。
4. **Tier-1 检索管线**：给 2487 张 `.pix` 建 tag/embedding 索引，AI 按 prompt 挑 tile/sprite。
5. **分享闭环**：`pixel share <cart>` → 终端可跑 + 一个 web 可玩链接（PICO-8 BBS 那套的对标）。

---

## 9. 文件索引

| 路径 | 说明 |
|---|---|
| `spike/cartgen/` | 隔离 spike crate（自带 workspace，不影响主 build，未 commit） |
| `spike/cartgen/src/main.rs` | headless Lua 卡带运行器 + 客观判定 |
| `spike/cartgen/CART_API.md` | 卡带 API 契约（喂给生成模型） |
| `spike/cartgen/carts/*.lua` | 10 个生成卡带 |
| `spike/cartgen/carts_broken/`、`carts_repaired/` | 自修复循环的翻车/修复样本 |
| `spike/cartgen/REPORT.md` | spike 详细报告 + 真实实验设计 |
| `spike/cartgen/arcade.html` | 可玩街机 demo（JS 移植，已发布 artifact） |
| `spike/cartgen/run_all.sh` | 一键复现 |
| `doc/roadmap_2026.md` | 原 roadmap（本方向对应 P0#3 AI PETSCII 支柱） |

---

*本文档是决策记录，不是承诺路线。核心待办：接引擎（§8.1）+ 真实多模型 A/B（§8.3）。*
