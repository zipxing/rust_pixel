use crate::model::{PixelSsfModel, SSFPLAYERW, SSFPLAYERH};
use rust_pixel::{
    asset::{AssetState, AssetType},
    asset2sprite_raw,
    context::Context,
    event::{event_check, event_register},
    game::Render,
    render::panel::Panel,
    render::sprite::{BorderType, Borders, Sprite},
    render::style::Color,
};
use log::info;

pub struct PixelSsfRender {
    pub panel: Panel,
}

impl PixelSsfRender {
    pub fn new() -> Self {
        Self {
            panel: Panel::new(),
        }
    }

    fn create_sprites(&mut self, _ctx: &mut Context) {
        // 创建主播放区域精灵
        let mut player_sprite = Sprite::new(1, 1, SSFPLAYERW - 2, SSFPLAYERH - 2);
        player_sprite.set_border(
            Borders::ALL,
            BorderType::Rounded,
            rust_pixel::render::style::Style::default().fg(Color::White),
        );
        self.panel.add_sprite(player_sprite, "player");

        // 创建控制信息显示区域
        let mut info_sprite = Sprite::new(1, SSFPLAYERH - 5, SSFPLAYERW - 2, 3);
        info_sprite.set_border(
            Borders::ALL,
            BorderType::Thick,
            rust_pixel::render::style::Style::default().fg(Color::White),
        );
        self.panel.add_sprite(info_sprite, "info");

        // 创建SSF动画精灵
        let ssf_sprite = Sprite::new(2, 2, SSFPLAYERW - 4, SSFPLAYERH - 8);
        self.panel.add_sprite(ssf_sprite, "ssf_animation");
    }

    fn update_info(&mut self, _ctx: &mut Context, model: &PixelSsfModel) {
        let info_sprite = self.panel.get_sprite("info");
        info_sprite.content.reset();

        // 显示当前帧信息
        let frame_info = format!(
            "Frame: {}/{} | Speed: {:.1}x | Mode: {} | Auto: {}",
            model.frame_idx + 1,
            model.frame_count,
            1.0 / model.play_speed,
            if model.loop_mode { "Loop" } else { "Once" },
            if model.auto_play { "ON" } else { "OFF" }
        );

        info_sprite.content.set_str(
            2,
            1,
            &frame_info,
            rust_pixel::render::style::Style::default().fg(Color::Yellow),
        );

        // 显示控制说明
        let controls = "Space:Play/Pause | ←→:Frame | R:Reset | L:Loop | ±:Speed | Q:Quit";
        info_sprite.content.set_str(
            2,
            2,
            controls,
            rust_pixel::render::style::Style::default().fg(Color::Cyan),
        );
    }

    fn update_ssf_animation(&mut self, ctx: &mut Context, model: &PixelSsfModel) {
        if model.frame_count == 0 {
            return;
        }

        let ssf_sprite = self.panel.get_sprite("ssf_animation");
        
        // 加载并显示当前帧
        asset2sprite_raw!(
            ssf_sprite,
            ctx,
            &model.ssf_file,
            model.frame_idx
        );
    }

    fn check_ssf_ready(&mut self, ctx: &mut Context, model: &mut PixelSsfModel) {
        // 检查SSF资源是否已加载
        // asset2sprite宏使用完整路径作为key，所以我们也要用完整路径
        let asset_key = format!("{}", &model.ssf_file);
        if let Some(asset) = ctx.asset_manager.get(&asset_key) {
            if asset.get_state() == AssetState::Ready {
                let new_frame_count = asset.get_base().frame_count;
                if model.frame_count != new_frame_count {
                    model.frame_count = new_frame_count;
                    info!("SSF loaded: {} frames", model.frame_count);
                    if model.frame_idx >= model.frame_count {
                        model.frame_idx = 0;
                    }
                }
            }
        }
    }
}

impl Render for PixelSsfRender {
    type Model = PixelSsfModel;

    fn init(&mut self, ctx: &mut Context, model: &mut Self::Model) {
        ctx.adapter.init(
            SSFPLAYERW,
            SSFPLAYERH,
            0.5,
            0.5,
            "PixelSSF Player".to_string(),
        );

        self.create_sprites(ctx);
        self.panel.init(ctx);

        // 注册事件
        event_register("PixelSsf.UpdateFrame", "update_frame");

        // 初始加载SSF文件
        let ssf_sprite = self.panel.get_sprite("ssf_animation");
        asset2sprite_raw!(ssf_sprite, ctx, &model.ssf_file, 0);

        info!("Graphics mode initialized");
    }

    fn handle_event(&mut self, ctx: &mut Context, model: &mut Self::Model, _dt: f32) {
        // 检查更新帧事件
        if event_check("PixelSsf.UpdateFrame", "update_frame") {
            self.update_ssf_animation(ctx, model);
        }
    }

    fn handle_timer(&mut self, ctx: &mut Context, model: &mut Self::Model, _dt: f32) {
        self.check_ssf_ready(ctx, model);
    }

    fn draw(&mut self, ctx: &mut Context, model: &mut Self::Model, _dt: f32) {
        // 更新信息显示
        self.update_info(ctx, model);

        // 更新SSF动画
        self.update_ssf_animation(ctx, model);

        // 绘制所有组件
        self.panel.draw(ctx).unwrap();
    }
} 
