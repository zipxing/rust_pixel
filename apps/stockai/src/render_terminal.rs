// Stock AI - 终端模式渲染器
// 使用 Table widget 显示股票列表

use crate::model::{StockaiModel, ViewMode};
use rust_pixel::{
    context::Context,
    game::Render,
    render::scene::Scene,
    render::style::{Color, Style},
    ui::{Panel, BorderStyle, Widget, Table, Column, ColumnAlign, TableRow, TableCell},
    util::Rect,
};

const PANEL_WIDTH: u16 = 76;
const PANEL_HEIGHT: u16 = 28;

pub struct StockaiRender {
    pub scene: Scene,
    pub main_panel: Panel,
    pub stock_table: Table,
}

impl StockaiRender {
    pub fn new() -> Self {
        let scene = Scene::new();

        let mut main_panel = Panel::new()
            .with_bounds(Rect::new(0, 0, PANEL_WIDTH, PANEL_HEIGHT))
            .with_border(BorderStyle::Double)
            .with_title(" Stock AI - 智能股票分析 ");
        main_panel.enable_canvas(PANEL_WIDTH - 2, PANEL_HEIGHT - 2);

        let stock_table = Table::new()
            .with_columns(vec![
                Column::new(" ", 2).align(ColumnAlign::Left),
                Column::new("代码", 9).align(ColumnAlign::Left),
                Column::new("名称", 13).align(ColumnAlign::Left),
                Column::new("价格", 9).align(ColumnAlign::Right),
                Column::new("涨跌", 8).align(ColumnAlign::Right),
                Column::new("评分", 6).align(ColumnAlign::Right),
                Column::new("建议", 8).align(ColumnAlign::Left),
            ])
            .with_header(true)
            .with_header_style(Style::default().fg(Color::DarkGray).bg(Color::Reset))
            .with_selected_style(Style::default().fg(Color::White).bg(Color::Indexed(236)))
            .with_style(Style::default().fg(Color::White).bg(Color::Reset));

        Self { scene, main_panel, stock_table }
    }

    // 从 model 构建 Table 行数据
    fn build_stock_rows(model: &StockaiModel) -> Vec<TableRow> {
        model.stocks.iter().enumerate().map(|(i, stock)| {
            let is_selected = i == model.selected_index;
            let is_up = stock.is_up();
            let has_data = stock.current_price > 0.0;

            let marker = if is_selected { "▶" } else { " " };
            let symbol = stock.config.symbol.replace("sh", "").replace("sz", "");

            let price_color = if !has_data {
                Color::DarkGray
            } else if is_up {
                Color::Red
            } else {
                Color::Green
            };

            let price_str = if has_data {
                format!("{:.2}", stock.current_price)
            } else {
                "--".to_string()
            };

            let change_str = if !has_data {
                "--%".to_string()
            } else if is_up {
                format!("+{:.2}%", stock.change_pct)
            } else {
                format!("{:.2}%", stock.change_pct)
            };

            let (score_str, score_color, advice) = if let Some(ref analysis) = stock.analysis {
                let sc = if analysis.total_score >= 60 {
                    Color::Red
                } else if analysis.total_score >= 40 {
                    Color::Yellow
                } else {
                    Color::Green
                };
                let adv = if analysis.total_score >= 80 {
                    "强买"
                } else if analysis.total_score >= 60 {
                    "可买"
                } else if analysis.total_score >= 40 {
                    "观望"
                } else {
                    "不买"
                };
                (format!("{}分", analysis.total_score), sc, adv.to_string())
            } else {
                ("--分".to_string(), Color::DarkGray, "加载中".to_string())
            };

            TableRow::new(vec![
                TableCell::new(marker).with_style(Style::default().fg(Color::White)),
                TableCell::new(&symbol).with_style(Style::default().fg(Color::Yellow)),
                TableCell::new(&stock.config.name),
                TableCell::new(&price_str).with_style(Style::default().fg(price_color)),
                TableCell::new(&change_str).with_style(Style::default().fg(price_color)),
                TableCell::new(&score_str).with_style(Style::default().fg(score_color)),
                TableCell::new(&advice).with_style(Style::default().fg(score_color)),
            ])
        }).collect()
    }

    // 绘制列表视图
    fn draw_list_view(&mut self, model: &StockaiModel, ctx: &Context) {
        self.main_panel.clear_canvas();

        // 更新 Table 数据
        let rows = Self::build_stock_rows(model);
        self.stock_table.set_rows(rows);
        self.stock_table.select(Some(model.selected_index));

        // 设置 Table 的 bounds 在 canvas 坐标系中
        self.stock_table.set_bounds(Rect::new(0, 0, PANEL_WIDTH - 2, 20));

        // 渲染 Table 到 canvas buffer
        let canvas = self.main_panel.canvas_mut();
        let _ = self.stock_table.render(canvas, ctx);

        // 底部帮助
        let help_y = 22;
        self.main_panel.set_str(0, help_y,
            "─────────────────────────────────────────────────────────────────────",
            Color::DarkGray, Color::Reset);
        self.main_panel.set_str(0, help_y + 1,
            &format!(" 更新时间: {}    股票数: {}", model.update_time, model.stocks.len()),
            Color::Cyan, Color::Reset);
        self.main_panel.set_str(0, help_y + 2,
            " ↑↓:选择  Enter:查看分析  R:刷新  Q:退出",
            Color::DarkGray, Color::Reset);
    }

    // 绘制分析详情视图（保持原有实现）
    fn draw_analysis_view(&mut self, model: &StockaiModel) {
        self.main_panel.clear_canvas();

        let stock = match model.selected_stock() {
            Some(s) => s,
            None => return,
        };

        let analysis = match &stock.analysis {
            Some(a) => a,
            None => {
                self.main_panel.set_str(2, 2, "正在加载分析数据...", Color::Yellow, Color::Reset);
                return;
            }
        };

        let is_up = stock.is_up();
        let price_color = if is_up { Color::Red } else { Color::Green };

        // 股票基本信息
        let symbol = stock.config.symbol.replace("sh", "").replace("sz", "");
        self.main_panel.set_str(0, 0,
            &format!(" {} {} ", symbol, stock.config.name),
            Color::Yellow, Color::Indexed(236));

        let price_info = format!(
            "  价格: {:.2}  涨跌: {:+.2} ({:+.2}%)",
            stock.current_price, stock.change, stock.change_pct
        );
        self.main_panel.set_str(18, 0, &price_info, price_color, Color::Reset);

        // 分割线
        self.main_panel.set_str(0, 1,
            "═══════════════════════════════════════════════════════════════════════",
            Color::DarkGray, Color::Reset);

        // 技术指标
        self.main_panel.set_str(0, 2, " 技术指标:", Color::Cyan, Color::Reset);

        self.main_panel.set_str(2, 3,
            &format!("MA5:{:>8.2}  MA10:{:>8.2}  MA20:{:>8.2}  MA60:{:>8.2}",
                analysis.ma.ma5, analysis.ma.ma10, analysis.ma.ma20, analysis.ma.ma60),
            Color::White, Color::Reset);

        let macd_color = if analysis.macd.histogram > 0.0 { Color::Red } else { Color::Green };
        self.main_panel.set_str(2, 4,
            &format!("MACD: DIF={:>7.2}  DEA={:>7.2}  柱={:>7.2}",
                analysis.macd.dif, analysis.macd.dea, analysis.macd.histogram),
            macd_color, Color::Reset);

        let kdj_color = if analysis.kdj.k > analysis.kdj.d { Color::Red } else { Color::Green };
        self.main_panel.set_str(2, 5,
            &format!("KDJ:  K={:>6.1}  D={:>6.1}  J={:>6.1}",
                analysis.kdj.k, analysis.kdj.d, analysis.kdj.j),
            kdj_color, Color::Reset);

        let rsi_color = if analysis.rsi.rsi6 < 30.0 {
            Color::Green
        } else if analysis.rsi.rsi6 > 70.0 {
            Color::Red
        } else {
            Color::White
        };
        self.main_panel.set_str(2, 6,
            &format!("RSI:  RSI6={:>5.1}  RSI12={:>5.1}  RSI24={:>5.1}",
                analysis.rsi.rsi6, analysis.rsi.rsi12, analysis.rsi.rsi24),
            rsi_color, Color::Reset);

        self.main_panel.set_str(2, 7,
            &format!("BOLL: 上轨={:>8.2}  中轨={:>8.2}  下轨={:>8.2}",
                analysis.boll.upper, analysis.boll.middle, analysis.boll.lower),
            Color::White, Color::Reset);

        // 分割线
        self.main_panel.set_str(0, 8,
            "───────────────────────────────────────────────────────────────────────",
            Color::DarkGray, Color::Reset);

        // 信号分析
        self.main_panel.set_str(0, 9, " 买入信号:", Color::Cyan, Color::Reset);

        for (i, signal) in analysis.signals.iter().enumerate() {
            let y = (10 + i) as u16;
            if y > 20 {
                break;
            }

            let marker = if signal.triggered { "✓" } else { "○" };
            let color = if signal.triggered { Color::Red } else { Color::DarkGray };

            self.main_panel.set_str(2, y, marker, color, Color::Reset);
            self.main_panel.set_str(4, y,
                &format!("{:<10}", signal.name),
                color, Color::Reset);
            self.main_panel.set_str(15, y,
                &format!("+{:>2}分", signal.score),
                if signal.score > 0 { Color::Yellow } else { Color::DarkGray },
                Color::Reset);
            self.main_panel.set_str(22, y, &signal.description, Color::White, Color::Reset);
        }

        // 分割线
        self.main_panel.set_str(0, 19,
            "═══════════════════════════════════════════════════════════════════════",
            Color::DarkGray, Color::Reset);

        // 综合评分和建议
        let score_color = if analysis.total_score >= 60 {
            Color::Red
        } else if analysis.total_score >= 40 {
            Color::Yellow
        } else {
            Color::Green
        };

        self.main_panel.set_str(0, 20, " 综合评分:", Color::Cyan, Color::Reset);
        self.main_panel.set_str(12, 20,
            &format!("{}/100", analysis.total_score),
            score_color, Color::Reset);

        self.main_panel.set_str(0, 21, " 操作建议:", Color::Cyan, Color::Reset);
        self.main_panel.set_str(12, 21, &analysis.recommendation, score_color, Color::Reset);

        self.main_panel.set_str(0, 22,
            &format!(" 分析数据: 最近{}个交易日K线", stock.klines.len()),
            Color::DarkGray, Color::Reset);

        self.main_panel.set_str(0, 23,
            "───────────────────────────────────────────────────────────────────────",
            Color::DarkGray, Color::Reset);
        self.main_panel.set_str(0, 24,
            " ←→:切换股票  R:刷新  Esc:返回 | ⚠ 仅供参考,不构成投资建议!",
            Color::DarkGray, Color::Reset);
    }

    pub fn draw_view(&mut self, model: &StockaiModel, ctx: &Context) {
        match model.view_mode {
            ViewMode::List => self.draw_list_view(model, ctx),
            ViewMode::Analysis => self.draw_analysis_view(model),
        }
    }
}

impl Render for StockaiRender {
    type Model = StockaiModel;

    fn init(&mut self, context: &mut Context, _model: &mut Self::Model) {
        context.adapter.init(
            PANEL_WIDTH,
            PANEL_HEIGHT,
            1.0,
            1.0,
            "stockai".to_string(),
        );
        self.scene.init(context);
    }

    fn handle_event(&mut self, _context: &mut Context, _model: &mut Self::Model, _dt: f32) {}

    fn handle_timer(&mut self, _context: &mut Context, _model: &mut Self::Model, _dt: f32) {}

    fn draw(&mut self, context: &mut Context, model: &mut Self::Model, _dt: f32) {
        self.draw_view(model, context);

        let buffer = self.scene.tui_buffer_mut();
        let _ = self.main_panel.render(buffer, context);
        self.scene.draw(context).unwrap();
    }
}
