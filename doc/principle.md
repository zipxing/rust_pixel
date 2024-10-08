### gameloop

![gameloop](./p1.jpg)

- In each frame, the `update` methods of both `model` and `render` are executed alternately.
- Modules can communicate and decouple through a messaging mechanism.
- A timer module is built into the message bus.
- The `model` manages game state data and logic, while the `render` mechanism is explained in detail below.

<br>

### text mode render

![textrender](./p2.jpg)

- The rendering process in text mode is relatively simple.
- A `Panel` contains several layers (sprite collections).
- All sprites from these layers are merged into the main buffer.
- After the double-buffering comparison, the changed content is drawn to the terminal using `crossterm`.

<br>

### graphical mode render pass 1

![graphrender1](./p3.jpg)

- In graphical mode, rendering supports Pixel Layers in addition to regular sprite collections.
- Pixel Layers manage Pixel Sprites, which differ from regular sprites by being able to move at the pixel level.
- In graphical mode, regular sprites are also merged into the main buffer and can be used to display background elements.
- Pixel Sprites are rendered separately and support transparency.
- Each Pixel Sprite's buffer, along with the main buffer, is added to the `RenderBuffer`.
- Each element of the `RenderBuffer` is a `RenderCell`.

<br>

### graphical mode render pass 2

![graphrender2](./p4.jpg)

- In graphical mode, rendering supports Pixel Layers in addition to regular sprite collections.
- Pixel Layers manage Pixel Sprites, which differ from regular sprites by being able to move at the pixel level.

<br>

