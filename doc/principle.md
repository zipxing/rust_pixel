### gameloop
- 每帧会交替执行model和render的update方法
- 模块之间可以通过消息机制进行通信解耦
- 在消息总线上，内置了一个定时器模块

![gameloop](./p1.jpg)

Model管理游戏的状态数据和逻辑，以下主要介绍Render原理

### text mode render
- 文本模式的渲染流程比较简单
- Panel包含若干layer(精灵集合)
- 所有layer的精灵，都会merge到main buffer中
- main buffer经过双缓冲比对后，改变的内容通过crossterm绘制到终端上

![textrender](./p2.jpg)

### graphical mode render pass 1
- 图形模式的渲染除了普通的精灵集合外，还支持Pixel Layers
- Pixel Layers管理Pixel Sprite，Pixel Sprite跟普通精灵区别是能够按照单个像素移动
- 图形模式下普通精灵也会merge到main buffer中，可以用来展现背景元素
- Pixel Sprites会分别渲染，同时也支持透明度
- 每个PixelSprite对应的buffer和main buffer都会被添加到RenderBuffer里
- RenderBuffer的每个元素是RenderCell

![graphrender1](./p3.jpg)

### graphical mode render pass 2
- 图形模式的渲染除了普通的精灵集合外，还支持Pixel Layers
- Pixel Layers管理Pixel Sprite，Pixel Sprite跟普通精灵区别是能够按照单个像素移动

![graphrender2](./p4.jpg)
