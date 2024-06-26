# 0.3.1
1. Add bezier algorithm, refer to algorithm/bezier.rs
2. Add a demo about keyframe animation and bezier path,refer to games/template/model.rs & render.rs
- model
```
        let in_points = [
            PointF32 { x: 0.0, y: 0.0 },
            PointF32 { x: 1200.0, y: 100.0 },
            PointF32 {
                x: TEMPLATEW as f32 * 16.0,
                y: TEMPLATEH as f32 * 16.0,
            },
        ];
        let num = 100;
        let mut pts = vec![PointF32 { x: 0.0, y: 0.0 }; num];
        draw_bezier_curves(&in_points, &mut pts);
        let mut ks = Vec::new();
        for i in 0..num {
            ks.push((pts[i], i as f64 / num as f64).into());
        }
        self.bezier = AnimationSequence::from(ks);
```
- render
```
        for i in 0..15 {
            let mut pl = Sprite::new(4, 6, 1, 1);
            pl.set_graph_sym(0, 0, 1, 83, Color::Indexed(203));
            pl.set_alpha(255 - 15*(15 - i));
            panel.add_pixel_sprite(pl, &format!("PL{}", i+1));
        }
        ...
        for i in 0..15 {
            let pl = &mut self.panel.get_pixel_sprite(&format!("PL{}", i+1));
            d.bezier.advance_and_maybe_reverse(dt as f64 * 0.1 + 0.01 * i as f64);
            let kf_now = d.bezier.now_strict().unwrap();
            pl.set_pos(kf_now.x as u16, kf_now.y as u16);
            d.bezier.advance_and_maybe_reverse(-0.01 * i as f64);
        }
```
3. Fix some bugs...

# 0.3.0
1. Add particle system
```
    //refer to games/snake/model.rs
    self.pats.fire_at(10.0, 10.0);
    ...
    self.pats.update(dt as f64);
    
    //refer to games/snake/render.rs
    pub fn draw_movie<G: Model>(&mut self, _ctx: &mut Context, model: &mut G) {
        let d = model.as_any().downcast_mut::<SnakeModel>().unwrap();

        self.panel.draw_objpool(&mut d.pats.particles, |pl, m| {
            pl.set_pos(m.obj.loc[0] as u16, m.obj.loc[1] as u16);
        });
    }
```
2. Optimize some render APIs for sprite
```
    /// set string content at (x,y) with fg/bg color...
    pub fn set_color_str<S>(&mut self, x: u16, y: u16, string: S, f: Color, b: Color);

    /// set string content at (0,0) with default style...
    pub fn set_default_str<S>(&mut self, string: S);

    /// set graphic model symbol(texture:texture_id, index:sym) at (x,y) with fgcolor...
    pub fn set_graph_sym(&mut self, x: u16, y: u16, texture_id: u8, sym: u8, f: Color);
```

# 0.2.0
1. Add a good template with bin / lib / ffi / wasm for create your own game, app or library
- refer to games/template
- create your own game, run:
```
cargo pixel c <my_game_name> 
```
2. Add global alpha for sprite
```
let sp: Sprite ... ...
sp.set_alpha(100);
```

