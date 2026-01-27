/// GameContext trait - 游戏引擎接口抽象
///
/// 定义 BASIC 脚本与 rust_pixel 游戏引擎之间的接口。
/// 通过此 trait，BASIC 程序可以调用图形绘制、精灵管理、输入查询等功能。
///
/// # 设计原则
///
/// - **平台无关**: 此 trait 不依赖任何具体的渲染后端
/// - **同步 API**: 所有方法都是同步的，适合 BASIC 脚本调用
/// - **类型安全**: 使用 Option/Result 处理错误情况
///
/// # 坐标系统
///
/// - 原点 (0, 0) 位于左上角
/// - X 轴向右递增
/// - Y 轴向下递增
/// - 坐标单位为字符/单元格（Cell）
///
/// # 颜色编码
///
/// - 使用 0-255 的整数表示颜色索引
/// - 具体颜色映射由实现者定义（可映射到 rust_pixel 的 PixelColor）
pub trait GameContext: std::any::Any {
    // ============================================================
    // 图形绘制方法
    // ============================================================

    /// PLOT x, y, ch$, fg, bg - 在指定位置绘制字符
    ///
    /// # 参数
    ///
    /// - `x`: X 坐标
    /// - `y`: Y 坐标
    /// - `ch`: 要绘制的字符
    /// - `fg`: 前景色索引 (0-255)
    /// - `bg`: 背景色索引 (0-255)
    ///
    /// # BASIC 示例
    ///
    /// ```basic
    /// 10 PLOT 10, 5, "@", 2, 0
    /// ```
    fn plot(&mut self, x: i32, y: i32, ch: char, fg: u8, bg: u8);

    /// CLS - 清空屏幕
    ///
    /// 将所有单元格重置为空字符（空格），颜色为默认值。
    ///
    /// # BASIC 示例
    ///
    /// ```basic
    /// 10 CLS
    /// ```
    fn cls(&mut self);

    /// LINE x0, y0, x1, y1, ch$ - 绘制线段
    ///
    /// 使用 Bresenham 算法绘制从 (x0, y0) 到 (x1, y1) 的直线。
    ///
    /// # 参数
    ///
    /// - `x0`, `y0`: 起点坐标
    /// - `x1`, `y1`: 终点坐标
    /// - `ch`: 用于绘制线段的字符
    ///
    /// # BASIC 示例
    ///
    /// ```basic
    /// 10 LINE 0, 0, 10, 10, "*"
    /// ```
    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, ch: char);

    /// BOX x, y, w, h, style - 绘制矩形边框
    ///
    /// # 参数
    ///
    /// - `x`, `y`: 左上角坐标
    /// - `w`: 宽度（字符数）
    /// - `h`: 高度（字符数）
    /// - `style`: 边框样式 (0=ASCII, 1=单线框, 2=双线框)
    ///
    /// # BASIC 示例
    ///
    /// ```basic
    /// 10 BOX 5, 5, 10, 8, 1
    /// ```
    fn box_draw(&mut self, x: i32, y: i32, w: i32, h: i32, style: u8);

    /// CIRCLE cx, cy, r, ch$ - 绘制圆形
    ///
    /// 使用中点圆算法绘制圆形边框。
    ///
    /// # 参数
    ///
    /// - `cx`, `cy`: 圆心坐标
    /// - `r`: 半径（字符单位）
    /// - `ch`: 用于绘制圆形的字符
    ///
    /// # BASIC 示例
    ///
    /// ```basic
    /// 10 CIRCLE 20, 12, 5, "O"
    /// ```
    fn circle(&mut self, cx: i32, cy: i32, r: i32, ch: char);

    // ============================================================
    // 精灵管理方法
    // ============================================================

    /// SPRITE id, x, y, ch$ - 创建或更新精灵
    ///
    /// 如果指定 ID 的精灵不存在，则创建新精灵；否则更新现有精灵。
    ///
    /// # 参数
    ///
    /// - `id`: 精灵 ID (0-65535)
    /// - `x`, `y`: 精灵位置
    /// - `ch`: 精灵显示的字符
    ///
    /// # BASIC 示例
    ///
    /// ```basic
    /// 10 SPRITE 1, 10, 20, "@"
    /// ```
    fn sprite_create(&mut self, id: u32, x: i32, y: i32, ch: char);

    /// SMOVE id, dx, dy - 相对移动精灵
    ///
    /// 将精灵从当前位置移动指定的偏移量。
    ///
    /// # 参数
    ///
    /// - `id`: 精灵 ID
    /// - `dx`: X 方向偏移
    /// - `dy`: Y 方向偏移
    ///
    /// # BASIC 示例
    ///
    /// ```basic
    /// 10 SMOVE 1, 2, -1
    /// ```
    fn sprite_move(&mut self, id: u32, dx: i32, dy: i32);

    /// SPOS id, x, y - 设置精灵绝对位置
    ///
    /// # 参数
    ///
    /// - `id`: 精灵 ID
    /// - `x`, `y`: 新的绝对坐标
    ///
    /// # BASIC 示例
    ///
    /// ```basic
    /// 10 SPOS 1, 15, 10
    /// ```
    fn sprite_pos(&mut self, id: u32, x: i32, y: i32);

    /// SHIDE id, hidden - 控制精灵可见性
    ///
    /// # 参数
    ///
    /// - `id`: 精灵 ID
    /// - `hidden`: true=隐藏, false=显示
    ///
    /// # BASIC 示例
    ///
    /// ```basic
    /// 10 SHIDE 1, 1  ' 隐藏精灵 1
    /// 20 SHIDE 1, 0  ' 显示精灵 1
    /// ```
    fn sprite_hide(&mut self, id: u32, hidden: bool);

    /// SCOLOR id, fg, bg - 设置精灵颜色
    ///
    /// # 参数
    ///
    /// - `id`: 精灵 ID
    /// - `fg`: 前景色索引
    /// - `bg`: 背景色索引
    ///
    /// # BASIC 示例
    ///
    /// ```basic
    /// 10 SCOLOR 1, 14, 0  ' 黄色前景，黑色背景
    /// ```
    fn sprite_color(&mut self, id: u32, fg: u8, bg: u8);

    // ============================================================
    // 输入查询方法
    // ============================================================

    /// INKEY() - 返回最后按下的按键 ASCII 码
    ///
    /// # 返回值
    ///
    /// - 如果有按键: 返回按键的 ASCII 码 (1-255)
    /// - 如果无按键: 返回 0
    ///
    /// # BASIC 示例
    ///
    /// ```basic
    /// 10 K = INKEY()
    /// 20 IF K = 0 THEN GOTO 10
    /// 30 PRINT "KEY: "; K
    /// ```
    fn inkey(&self) -> u32;

    /// KEY(key$) - 检查指定按键是否当前按下
    ///
    /// # 参数
    ///
    /// - `key`: 按键名称字符串 (如 "W", "SPACE", "UP", "ESC")
    ///
    /// # 返回值
    ///
    /// - true: 按键当前被按下
    /// - false: 按键未被按下
    ///
    /// # BASIC 示例
    ///
    /// ```basic
    /// 10 IF KEY("W") THEN Y=Y-1
    /// 20 IF KEY("S") THEN Y=Y+1
    /// ```
    fn key(&self, key: &str) -> bool;

    /// MOUSEX() - 返回鼠标光标 X 坐标
    ///
    /// # 返回值
    ///
    /// 鼠标当前的 X 坐标（字符单位）
    ///
    /// # BASIC 示例
    ///
    /// ```basic
    /// 10 MX = MOUSEX()
    /// ```
    fn mouse_x(&self) -> i32;

    /// MOUSEY() - 返回鼠标光标 Y 坐标
    ///
    /// # 返回值
    ///
    /// 鼠标当前的 Y 坐标（字符单位）
    ///
    /// # BASIC 示例
    ///
    /// ```basic
    /// 10 MY = MOUSEY()
    /// ```
    fn mouse_y(&self) -> i32;

    /// MOUSEB() - 返回鼠标按钮状态位掩码
    ///
    /// # 返回值
    ///
    /// 位掩码值:
    /// - 1 (bit 0): 左键按下
    /// - 2 (bit 1): 右键按下
    /// - 4 (bit 2): 中键按下
    ///
    /// # BASIC 示例
    ///
    /// ```basic
    /// 10 MB = MOUSEB()
    /// 20 IF MB AND 1 THEN PRINT "LEFT CLICK"
    /// ```
    fn mouse_button(&self) -> u8;

    // ============================================================
    // 精灵查询方法
    // ============================================================

    /// SPRITEX(id) - 返回精灵 X 坐标
    ///
    /// # 参数
    ///
    /// - `id`: 精灵 ID
    ///
    /// # 返回值
    ///
    /// - Some(x): 精灵存在，返回 X 坐标
    /// - None: 精灵不存在
    ///
    /// # BASIC 示例
    ///
    /// ```basic
    /// 10 X = SPRITEX(1)
    /// ```
    fn sprite_x(&self, id: u32) -> Option<i32>;

    /// SPRITEY(id) - 返回精灵 Y 坐标
    ///
    /// # 参数
    ///
    /// - `id`: 精灵 ID
    ///
    /// # 返回值
    ///
    /// - Some(y): 精灵存在，返回 Y 坐标
    /// - None: 精灵不存在
    ///
    /// # BASIC 示例
    ///
    /// ```basic
    /// 10 Y = SPRITEY(1)
    /// ```
    fn sprite_y(&self, id: u32) -> Option<i32>;

    /// SPRITEHIT(id1, id2) - 检测两个精灵是否碰撞
    ///
    /// # 参数
    ///
    /// - `id1`: 第一个精灵 ID
    /// - `id2`: 第二个精灵 ID
    ///
    /// # 返回值
    ///
    /// - true: 两个精灵位置重叠（碰撞）
    /// - false: 未碰撞或任一精灵不存在
    ///
    /// # BASIC 示例
    ///
    /// ```basic
    /// 10 IF SPRITEHIT(1, 2) THEN PRINT "COLLISION!"
    /// ```
    fn sprite_hit(&self, id1: u32, id2: u32) -> bool;
}

/// NullGameContext - 空实现，用于测试和脚本验证
///
/// 所有图形/精灵/输入操作都是空操作（no-op），
/// 可用于单元测试 BASIC 脚本逻辑而不依赖实际的游戏引擎。
pub struct NullGameContext;

impl GameContext for NullGameContext {
    fn plot(&mut self, _x: i32, _y: i32, _ch: char, _fg: u8, _bg: u8) {}
    fn cls(&mut self) {}
    fn line(&mut self, _x0: i32, _y0: i32, _x1: i32, _y1: i32, _ch: char) {}
    fn box_draw(&mut self, _x: i32, _y: i32, _w: i32, _h: i32, _style: u8) {}
    fn circle(&mut self, _cx: i32, _cy: i32, _r: i32, _ch: char) {}

    fn sprite_create(&mut self, _id: u32, _x: i32, _y: i32, _ch: char) {}
    fn sprite_move(&mut self, _id: u32, _dx: i32, _dy: i32) {}
    fn sprite_pos(&mut self, _id: u32, _x: i32, _y: i32) {}
    fn sprite_hide(&mut self, _id: u32, _hidden: bool) {}
    fn sprite_color(&mut self, _id: u32, _fg: u8, _bg: u8) {}

    fn inkey(&self) -> u32 { 0 }
    fn key(&self, _key: &str) -> bool { false }
    fn mouse_x(&self) -> i32 { 0 }
    fn mouse_y(&self) -> i32 { 0 }
    fn mouse_button(&self) -> u8 { 0 }

    fn sprite_x(&self, _id: u32) -> Option<i32> { None }
    fn sprite_y(&self, _id: u32) -> Option<i32> { None }
    fn sprite_hit(&self, _id1: u32, _id2: u32) -> bool { false }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_null_context_compiles() {
        let mut ctx = NullGameContext;
        ctx.plot(0, 0, '@', 1, 0);
        ctx.cls();
        ctx.sprite_create(1, 10, 20, '@');
        assert_eq!(ctx.inkey(), 0);
        assert_eq!(ctx.sprite_x(1), None);
    }
}
