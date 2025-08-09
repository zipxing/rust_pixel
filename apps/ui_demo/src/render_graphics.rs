use crate::model::{UiDemoModel, UI_DEMO_WIDTH, UI_DEMO_HEIGHT};
use rust_pixel::{
    context::Context,
    game::Render,
    render::panel::Panel,
};
use log::info;

pub struct UiDemoRender {
    pub panel: Panel,
}

impl UiDemoRender {
    pub fn new() -> Self {
        Self {
            panel: Panel::new(),
        }
    }
}

impl Render for UiDemoRender {
    type Model = UiDemoModel;
    
    fn init(&mut self, ctx: &mut Context, _model: &mut UiDemoModel) {
        info!("UI Demo render initialized (graphics mode)");
        // Initialize adapter for graphics mode
        ctx.adapter.init(UI_DEMO_WIDTH as u16, UI_DEMO_HEIGHT as u16, 1.0, 0.5, String::new());
        // Initialize the panel to cover the full screen
        self.panel.init(ctx);
    }
    
    fn handle_event(&mut self, _ctx: &mut Context, _model: &mut UiDemoModel, _dt: f32) {
        // Events are handled in the model
    }
    
    fn handle_timer(&mut self, _ctx: &mut Context, _model: &mut UiDemoModel, _dt: f32) {
        // Timer events
    }
    
    fn draw(&mut self, ctx: &mut Context, model: &mut UiDemoModel, _dt: f32) {
        // This is the main drawing method
        self.update(ctx, model, _dt);
    }
    
    fn update(&mut self, ctx: &mut Context, model: &mut UiDemoModel, _dt: f32) {
        // Clear the current buffer
        let buffer = self.panel.current_buffer_mut();
        buffer.reset();
        
        // Copy UI buffer to render buffer
        let ui_buffer = model.ui_app.buffer();
        let ui_area = ui_buffer.area();
        
        // Copy each cell from UI buffer to render buffer
        for y in 0..ui_area.height.min(UI_DEMO_HEIGHT as u16) {
            for x in 0..ui_area.width.min(UI_DEMO_WIDTH as u16) {
                let ui_cell = ui_buffer.get(x, y);
                if !ui_cell.is_blank() {
                    buffer.set_string(
                        x, y,
                        &ui_cell.symbol,
                        ui_cell.style(),
                    );
                }
            }
        }
        
        // Draw to screen
        let _ = self.panel.draw(ctx);
    }
}
