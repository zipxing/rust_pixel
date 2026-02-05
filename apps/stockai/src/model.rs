// Stock AI - 股票分析模型
// 获取历史数据并进行技术分析

use crate::indicators::{KLine, AnalysisResult, analyze};
use rust_pixel::event::{Event, KeyCode};
use rust_pixel::{
    context::Context,
    game::Model,
};
use chrono::Local;
use std::io::Read;

// 股票配置
#[derive(Clone)]
pub struct StockConfig {
    pub symbol: String,  // sh600519 或 sz000001
    pub name: String,
}

// 股票完整数据
#[derive(Clone)]
pub struct StockData {
    pub config: StockConfig,
    pub current_price: f64,
    pub prev_close: f64,
    pub change: f64,
    pub change_pct: f64,
    pub klines: Vec<KLine>,
    pub analysis: Option<AnalysisResult>,
}

impl StockData {
    pub fn new(symbol: &str, name: &str) -> Self {
        Self {
            config: StockConfig {
                symbol: symbol.to_string(),
                name: name.to_string(),
            },
            current_price: 0.0,
            prev_close: 0.0,
            change: 0.0,
            change_pct: 0.0,
            klines: Vec::new(),
            analysis: None,
        }
    }

    pub fn is_up(&self) -> bool {
        self.change >= 0.0
    }
}

// 股票 AI 模型
pub struct StockaiModel {
    pub stocks: Vec<StockData>,
    pub selected_index: usize,
    pub update_time: String,
    pub frame_count: u32,
    pub loading: bool,
    pub last_error: Option<String>,
    pub view_mode: ViewMode,  // 当前视图模式
}

#[derive(Clone, Copy, PartialEq)]
pub enum ViewMode {
    List,      // 股票列表
    Analysis,  // 分析详情
}

impl StockaiModel {
    pub fn new() -> Self {
        Self {
            stocks: Vec::new(),
            selected_index: 0,
            update_time: String::new(),
            frame_count: 0,
            loading: false,
            last_error: None,
            view_mode: ViewMode::List,
        }
    }

    // 初始化股票列表
    pub fn init_stocks(&mut self) {
        self.stocks.clear();
        
        // 添加监测的股票
        self.stocks.push(StockData::new("sh600519", "贵州茅台"));
        self.stocks.push(StockData::new("sz000858", "五粮液"));
        self.stocks.push(StockData::new("sh601318", "中国平安"));
        self.stocks.push(StockData::new("sz300750", "宁德时代"));
        self.stocks.push(StockData::new("sh600036", "招商银行"));
        self.stocks.push(StockData::new("sz000001", "平安银行"));
        self.stocks.push(StockData::new("sz000333", "美的集团"));
        self.stocks.push(StockData::new("sz002415", "海康威视"));
        
        self.selected_index = 0;
    }

    // 更新时间
    pub fn update_time(&mut self) {
        let now = Local::now();
        self.update_time = now.format("%H:%M:%S").to_string();
    }

    // 获取历史 K 线数据（腾讯接口）
    pub fn fetch_kline_data(&mut self, index: usize) {
        if index >= self.stocks.len() {
            return;
        }
        
        let symbol = self.stocks[index].config.symbol.clone();
        
        // 腾讯 K 线接口
        // http://web.ifzq.gtimg.cn/appstock/app/fqkline/get?param=sh600519,day,2024-01-01,2025-12-31,500,qfq
        let end_date = Local::now().format("%Y-%m-%d").to_string();
        let start_date = Local::now()
            .checked_sub_signed(chrono::Duration::days(180))
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "2024-01-01".to_string());
        
        let url = format!(
            "http://web.ifzq.gtimg.cn/appstock/app/fqkline/get?param={},day,{},{},500,qfq",
            symbol, start_date, end_date
        );
        
        match ureq::get(&url)
            .set("Referer", "http://finance.qq.com")
            .set("User-Agent", "Mozilla/5.0")
            .call()
        {
            Ok(response) => {
                let body = match response.into_string() {
                    Ok(b) => b,
                    Err(e) => {
                        self.last_error = Some(format!("读取响应失败: {}", e));
                        return;
                    }
                };
                
                // 解析 JSON
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                    self.parse_kline_json(&json, index);
                } else {
                    self.last_error = Some("JSON解析失败".to_string());
                }
            }
            Err(e) => {
                self.last_error = Some(format!("请求失败: {}", e));
            }
        }
    }

    // 解析 K 线 JSON 数据
    fn parse_kline_json(&mut self, json: &serde_json::Value, index: usize) {
        let symbol = &self.stocks[index].config.symbol;
        
        // 路径: data -> {symbol} -> day 或 qfqday
        if let Some(data) = json.get("data") {
            if let Some(stock_data) = data.get(symbol) {
                // 尝试 qfqday (前复权) 或 day
                let kline_key = if stock_data.get("qfqday").is_some() {
                    "qfqday"
                } else {
                    "day"
                };
                
                if let Some(klines) = stock_data.get(kline_key).and_then(|v| v.as_array()) {
                    let mut parsed_klines = Vec::new();
                    
                    for kline in klines {
                        if let Some(arr) = kline.as_array() {
                            if arr.len() >= 6 {
                                let date = arr[0].as_str().unwrap_or("").to_string();
                                let open: f64 = arr[1].as_str()
                                    .and_then(|s| s.parse().ok())
                                    .unwrap_or(0.0);
                                let close: f64 = arr[2].as_str()
                                    .and_then(|s| s.parse().ok())
                                    .unwrap_or(0.0);
                                let high: f64 = arr[3].as_str()
                                    .and_then(|s| s.parse().ok())
                                    .unwrap_or(0.0);
                                let low: f64 = arr[4].as_str()
                                    .and_then(|s| s.parse().ok())
                                    .unwrap_or(0.0);
                                let volume: u64 = arr[5].as_str()
                                    .and_then(|s| s.parse().ok())
                                    .unwrap_or(0);
                                
                                if close > 0.0 {
                                    parsed_klines.push(KLine {
                                        date,
                                        open,
                                        close,
                                        high,
                                        low,
                                        volume,
                                    });
                                }
                            }
                        }
                    }
                    
                    if !parsed_klines.is_empty() {
                        // 更新股票数据
                        let stock = &mut self.stocks[index];
                        stock.klines = parsed_klines;
                        
                        // 更新当前价格
                        if let Some(last) = stock.klines.last() {
                            stock.current_price = last.close;
                        }
                        if stock.klines.len() >= 2 {
                            let prev = &stock.klines[stock.klines.len() - 2];
                            stock.prev_close = prev.close;
                            stock.change = stock.current_price - stock.prev_close;
                            stock.change_pct = if stock.prev_close > 0.0 {
                                (stock.change / stock.prev_close) * 100.0
                            } else {
                                0.0
                            };
                        }
                        
                        // 执行技术分析
                        stock.analysis = Some(analyze(&stock.klines));
                        self.last_error = None;
                    }
                }
            }
        }
    }

    // 获取实时价格（腾讯接口）
    pub fn fetch_realtime_prices(&mut self) {
        let symbols: Vec<&str> = self.stocks.iter()
            .map(|s| s.config.symbol.as_str())
            .collect();
        let symbols_str = symbols.join(",");
        let url = format!("http://qt.gtimg.cn/q={}", symbols_str);
        
        match ureq::get(&url)
            .set("Referer", "http://finance.qq.com")
            .set("User-Agent", "Mozilla/5.0")
            .call()
        {
            Ok(response) => {
                let mut bytes = Vec::new();
                if response.into_reader().read_to_end(&mut bytes).is_ok() {
                    let (content, _, _) = encoding_rs::GBK.decode(&bytes);
                    self.parse_realtime_response(&content);
                }
            }
            Err(_) => {}
        }
        
        self.update_time();
    }

    // 解析实时价格响应
    fn parse_realtime_response(&mut self, body: &str) {
        for line in body.lines() {
            if !line.starts_with("v_") {
                continue;
            }
            
            let code_end = line.find('=').unwrap_or(0);
            if code_end < 3 {
                continue;
            }
            let full_symbol = &line[2..code_end];
            
            let start = line.find('"').map(|i| i + 1).unwrap_or(0);
            let end = line.rfind('"').unwrap_or(line.len());
            if start >= end {
                continue;
            }
            let data = &line[start..end];
            let parts: Vec<&str> = data.split('~').collect();
            
            if parts.len() < 45 {
                continue;
            }
            
            for stock in &mut self.stocks {
                if stock.config.symbol == full_symbol {
                    let name = parts.get(1).unwrap_or(&"").to_string();
                    let price: f64 = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    let prev_close: f64 = parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    let change: f64 = parts.get(31).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    let change_pct: f64 = parts.get(32).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    
                    if !name.is_empty() {
                        stock.config.name = name;
                    }
                    if price > 0.0 {
                        stock.current_price = price;
                        stock.prev_close = prev_close;
                        stock.change = change;
                        stock.change_pct = change_pct;
                    }
                    break;
                }
            }
        }
    }

    // 获取当前选中的股票
    pub fn selected_stock(&self) -> Option<&StockData> {
        self.stocks.get(self.selected_index)
    }

    // 刷新所有数据
    pub fn refresh_all(&mut self) {
        self.loading = true;
        
        // 获取实时价格
        self.fetch_realtime_prices();
        
        // 获取所有股票的 K 线数据
        for i in 0..self.stocks.len() {
            self.fetch_kline_data(i);
        }
        
        self.loading = false;
        self.update_time();
    }

    // 刷新单只股票
    pub fn refresh_selected(&mut self) {
        self.loading = true;
        self.fetch_kline_data(self.selected_index);
        self.loading = false;
    }
}

impl Model for StockaiModel {
    fn init(&mut self, context: &mut Context) {
        self.init_stocks();
        self.refresh_all();
        context.input_events.clear();
    }

    fn handle_input(&mut self, context: &mut Context, _dt: f32) {
        let events = context.input_events.clone();
        
        for e in &events {
            if let Event::Key(key) = e {
                match self.view_mode {
                    ViewMode::List => {
                        match key.code {
                            KeyCode::Up | KeyCode::Char('k') => {
                                if self.selected_index > 0 {
                                    self.selected_index -= 1;
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if self.selected_index < self.stocks.len().saturating_sub(1) {
                                    self.selected_index += 1;
                                }
                            }
                            // Enter 或 空格 或 v 进入分析详情
                            KeyCode::Enter | KeyCode::Char(' ') | KeyCode::Char('v') => {
                                self.view_mode = ViewMode::Analysis;
                                self.refresh_selected();
                            }
                            KeyCode::Char('r') | KeyCode::Char('R') => {
                                self.refresh_all();
                            }
                            _ => {}
                        }
                    }
                    ViewMode::Analysis => {
                        match key.code {
                            // Esc, Backspace, Q 返回列表
                            KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('q') | KeyCode::Char('Q') => {
                                self.view_mode = ViewMode::List;
                            }
                            KeyCode::Left | KeyCode::Char('h') | KeyCode::Up => {
                                // 上一只股票
                                if self.selected_index > 0 {
                                    self.selected_index -= 1;
                                    self.refresh_selected();
                                }
                            }
                            KeyCode::Right | KeyCode::Char('l') | KeyCode::Down => {
                                // 下一只股票
                                if self.selected_index < self.stocks.len().saturating_sub(1) {
                                    self.selected_index += 1;
                                    self.refresh_selected();
                                }
                            }
                            KeyCode::Char('r') | KeyCode::Char('R') => {
                                self.refresh_selected();
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        context.input_events.clear();
    }

    fn handle_auto(&mut self, _context: &mut Context, _dt: f32) {
        self.frame_count = self.frame_count.wrapping_add(1);
        
        // 每 5 分钟自动刷新实时价格
        if self.frame_count % (60 * 60 * 5) == 0 {
            self.fetch_realtime_prices();
        }
    }

    fn handle_event(&mut self, _context: &mut Context, _dt: f32) {}
    fn handle_timer(&mut self, _context: &mut Context, _dt: f32) {}
}
