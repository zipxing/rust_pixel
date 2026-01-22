TUI支持设计
- 文本模式下，全部的精灵包括UI组件，都合并到main buffer中。统一使用的都是“瘦高”字符。坐标计算也统一。(保持目前不变)
- 图形模式下，采用若干层PixelSprite + 最上层TUI的设计
- 图形模式下，普通的sprite包括UI组件，同样合并到main buffer中，也使用“瘦高”字符图形。坐标按“瘦”坐标计算。
- 图形模式下，Pixel Sprite仍然保持按顺序分层独立渲染
- 图形模式下，main buffer对应的TUI层，永远渲染在最上层
- 图形模式下，TUI/Emoji/Sprite/CJK 统一使用 symbols.png（2048x2048），TUI 区域使用 8x16 字符
- 图形模式下，鼠标事件返回两个坐标，一个tui坐标，一个普通坐标，分别用于处理TUI和普通游戏内容
- 图形模式下，main buffer和各层pixel sprite，仍然全部合并到RenderBuffer交给gpu渲染，仍然一次draw call
