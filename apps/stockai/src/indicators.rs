// 股票技术指标计算模块
// 包含 MA、MACD、KDJ、RSI、BOLL 等常用指标

/// K线数据
#[derive(Clone, Debug)]
pub struct KLine {
    pub date: String,
    pub open: f64,
    pub close: f64,
    pub high: f64,
    pub low: f64,
    pub volume: u64,
}

/// 均线数据
#[derive(Clone, Debug, Default)]
pub struct MAData {
    pub ma5: f64,
    pub ma10: f64,
    pub ma20: f64,
    pub ma60: f64,
}

/// MACD 数据
#[derive(Clone, Debug, Default)]
pub struct MACDData {
    pub dif: f64,      // 快线
    pub dea: f64,      // 慢线
    pub macd: f64,     // 柱状图 (DIF - DEA) * 2
    pub histogram: f64, // 柱状值
}

/// KDJ 数据
#[derive(Clone, Debug, Default)]
pub struct KDJData {
    pub k: f64,
    pub d: f64,
    pub j: f64,
}

/// RSI 数据
#[derive(Clone, Debug, Default)]
pub struct RSIData {
    pub rsi6: f64,
    pub rsi12: f64,
    pub rsi24: f64,
}

/// 布林带数据
#[derive(Clone, Debug, Default)]
pub struct BOLLData {
    pub upper: f64,   // 上轨
    pub middle: f64,  // 中轨 (MA20)
    pub lower: f64,   // 下轨
}

/// 买入信号
#[derive(Clone, Debug)]
pub struct BuySignal {
    pub name: String,
    pub triggered: bool,
    pub score: i32,
    pub description: String,
}

/// 综合分析结果
#[derive(Clone, Debug, Default)]
pub struct AnalysisResult {
    pub ma: MAData,
    pub macd: MACDData,
    pub kdj: KDJData,
    pub rsi: RSIData,
    pub boll: BOLLData,
    pub signals: Vec<BuySignal>,
    pub total_score: i32,
    pub recommendation: String,
}

/// 计算简单移动平均线 (SMA)
pub fn calc_sma(prices: &[f64], period: usize) -> f64 {
    if prices.len() < period {
        return 0.0;
    }
    let sum: f64 = prices.iter().rev().take(period).sum();
    sum / period as f64
}

/// 计算指数移动平均线 (EMA)
pub fn calc_ema(prices: &[f64], period: usize) -> f64 {
    if prices.is_empty() {
        return 0.0;
    }
    if prices.len() < period {
        return calc_sma(prices, prices.len());
    }
    
    let multiplier = 2.0 / (period as f64 + 1.0);
    let mut ema = calc_sma(&prices[..period], period);
    
    for price in prices.iter().skip(period) {
        ema = (price - ema) * multiplier + ema;
    }
    
    ema
}

/// 计算 MA 均线
pub fn calc_ma(closes: &[f64]) -> MAData {
    MAData {
        ma5: calc_sma(closes, 5),
        ma10: calc_sma(closes, 10),
        ma20: calc_sma(closes, 20),
        ma60: calc_sma(closes, 60),
    }
}

/// 计算 MACD
/// DIF = EMA12 - EMA26
/// DEA = EMA9(DIF)
/// MACD柱 = (DIF - DEA) * 2
pub fn calc_macd(closes: &[f64]) -> MACDData {
    if closes.len() < 26 {
        return MACDData::default();
    }
    
    // 计算 EMA12 和 EMA26 序列
    let mut ema12_values = Vec::new();
    let mut ema26_values = Vec::new();
    
    // 初始化
    let ema12_init = calc_sma(&closes[..12], 12);
    let ema26_init = calc_sma(&closes[..26], 26);
    
    let mut ema12 = ema12_init;
    let mut ema26 = ema26_init;
    
    let mult12 = 2.0 / 13.0;
    let mult26 = 2.0 / 27.0;
    
    for (i, &price) in closes.iter().enumerate() {
        if i < 12 {
            ema12_values.push(calc_sma(&closes[..=i], i + 1));
        } else {
            ema12 = (price - ema12) * mult12 + ema12;
            ema12_values.push(ema12);
        }
        
        if i < 26 {
            ema26_values.push(calc_sma(&closes[..=i], i + 1));
        } else {
            ema26 = (price - ema26) * mult26 + ema26;
            ema26_values.push(ema26);
        }
    }
    
    // 计算 DIF 序列
    let dif_values: Vec<f64> = ema12_values.iter()
        .zip(ema26_values.iter())
        .map(|(&e12, &e26)| e12 - e26)
        .collect();
    
    // 计算 DEA (DIF 的 EMA9)
    let dea = calc_ema(&dif_values, 9);
    let dif = *dif_values.last().unwrap_or(&0.0);
    let histogram = (dif - dea) * 2.0;
    
    MACDData {
        dif,
        dea,
        macd: histogram,
        histogram,
    }
}

/// 计算 KDJ
/// RSV = (C - L9) / (H9 - L9) * 100
/// K = 2/3 * K(-1) + 1/3 * RSV
/// D = 2/3 * D(-1) + 1/3 * K
/// J = 3K - 2D
pub fn calc_kdj(klines: &[KLine]) -> KDJData {
    if klines.len() < 9 {
        return KDJData { k: 50.0, d: 50.0, j: 50.0 };
    }
    
    let mut k = 50.0;
    let mut d = 50.0;
    
    for i in 8..klines.len() {
        let slice = &klines[i-8..=i];
        let high9: f64 = slice.iter().map(|k| k.high).fold(f64::NEG_INFINITY, f64::max);
        let low9: f64 = slice.iter().map(|k| k.low).fold(f64::INFINITY, f64::min);
        let close = klines[i].close;
        
        let rsv = if (high9 - low9).abs() < 0.0001 {
            50.0
        } else {
            (close - low9) / (high9 - low9) * 100.0
        };
        
        k = 2.0 / 3.0 * k + 1.0 / 3.0 * rsv;
        d = 2.0 / 3.0 * d + 1.0 / 3.0 * k;
    }
    
    let j = 3.0 * k - 2.0 * d;
    
    KDJData { k, d, j }
}

/// 计算 RSI
/// RSI = 100 * RS / (1 + RS)
/// RS = 平均涨幅 / 平均跌幅
pub fn calc_rsi(closes: &[f64], period: usize) -> f64 {
    if closes.len() < period + 1 {
        return 50.0;
    }
    
    let mut gains = 0.0;
    let mut losses = 0.0;
    
    for i in (closes.len() - period)..closes.len() {
        let change = closes[i] - closes[i - 1];
        if change > 0.0 {
            gains += change;
        } else {
            losses -= change;
        }
    }
    
    if losses < 0.0001 {
        return 100.0;
    }
    
    let rs = gains / losses;
    100.0 * rs / (1.0 + rs)
}

/// 计算 RSI 组合
pub fn calc_rsi_all(closes: &[f64]) -> RSIData {
    RSIData {
        rsi6: calc_rsi(closes, 6),
        rsi12: calc_rsi(closes, 12),
        rsi24: calc_rsi(closes, 24),
    }
}

/// 计算布林带
/// 中轨 = MA20
/// 上轨 = MA20 + 2 * 标准差
/// 下轨 = MA20 - 2 * 标准差
pub fn calc_boll(closes: &[f64]) -> BOLLData {
    if closes.len() < 20 {
        return BOLLData::default();
    }
    
    let middle = calc_sma(closes, 20);
    
    // 计算标准差
    let recent: Vec<f64> = closes.iter().rev().take(20).cloned().collect();
    let variance: f64 = recent.iter()
        .map(|&x| (x - middle).powi(2))
        .sum::<f64>() / 20.0;
    let std_dev = variance.sqrt();
    
    BOLLData {
        upper: middle + 2.0 * std_dev,
        middle,
        lower: middle - 2.0 * std_dev,
    }
}

/// 检测买入信号
pub fn detect_signals(
    current_price: f64,
    ma: &MAData,
    macd: &MACDData,
    kdj: &KDJData,
    rsi: &RSIData,
    boll: &BOLLData,
    prev_macd: Option<&MACDData>,
    prev_kdj: Option<&KDJData>,
) -> Vec<BuySignal> {
    let mut signals = Vec::new();
    
    // 1. MA 多头排列
    let ma_bullish = ma.ma5 > ma.ma10 && ma.ma10 > ma.ma20;
    signals.push(BuySignal {
        name: "MA多头".to_string(),
        triggered: ma_bullish,
        score: if ma_bullish { 15 } else { 0 },
        description: if ma_bullish {
            "MA5>MA10>MA20 多头排列".to_string()
        } else {
            "均线未形成多头排列".to_string()
        },
    });
    
    // 2. MA 金叉 (MA5 上穿 MA20)
    let ma_golden = ma.ma5 > ma.ma20 && current_price > ma.ma5;
    signals.push(BuySignal {
        name: "MA金叉".to_string(),
        triggered: ma_golden,
        score: if ma_golden { 15 } else { 0 },
        description: if ma_golden {
            "MA5上穿MA20，价格站上MA5".to_string()
        } else {
            "未出现MA金叉".to_string()
        },
    });
    
    // 3. MACD 金叉
    let macd_golden = if let Some(prev) = prev_macd {
        macd.dif > macd.dea && prev.dif <= prev.dea
    } else {
        macd.dif > macd.dea && macd.histogram > 0.0
    };
    signals.push(BuySignal {
        name: "MACD金叉".to_string(),
        triggered: macd_golden,
        score: if macd_golden { 20 } else { 0 },
        description: if macd_golden {
            format!("DIF({:.2})上穿DEA({:.2})", macd.dif, macd.dea)
        } else {
            "未出现MACD金叉".to_string()
        },
    });
    
    // 4. MACD 红柱
    let macd_positive = macd.histogram > 0.0;
    signals.push(BuySignal {
        name: "MACD红柱".to_string(),
        triggered: macd_positive,
        score: if macd_positive { 10 } else { 0 },
        description: if macd_positive {
            format!("MACD柱状值: {:.2}", macd.histogram)
        } else {
            "MACD为绿柱".to_string()
        },
    });
    
    // 5. KDJ 金叉
    let kdj_golden = if let Some(prev) = prev_kdj {
        kdj.k > kdj.d && prev.k <= prev.d && kdj.k < 80.0
    } else {
        kdj.k > kdj.d && kdj.k < 50.0
    };
    signals.push(BuySignal {
        name: "KDJ金叉".to_string(),
        triggered: kdj_golden,
        score: if kdj_golden { 15 } else { 0 },
        description: if kdj_golden {
            format!("K({:.1})上穿D({:.1})，低位金叉", kdj.k, kdj.d)
        } else {
            "未出现KDJ低位金叉".to_string()
        },
    });
    
    // 6. KDJ 超卖
    let kdj_oversold = kdj.k < 20.0 || kdj.j < 0.0;
    signals.push(BuySignal {
        name: "KDJ超卖".to_string(),
        triggered: kdj_oversold,
        score: if kdj_oversold { 15 } else { 0 },
        description: if kdj_oversold {
            format!("K={:.1}, J={:.1} 超卖区域", kdj.k, kdj.j)
        } else {
            "KDJ未进入超卖区".to_string()
        },
    });
    
    // 7. RSI 超卖
    let rsi_oversold = rsi.rsi6 < 30.0 || rsi.rsi12 < 35.0;
    signals.push(BuySignal {
        name: "RSI超卖".to_string(),
        triggered: rsi_oversold,
        score: if rsi_oversold { 15 } else { 0 },
        description: if rsi_oversold {
            format!("RSI6={:.1} 超卖区域", rsi.rsi6)
        } else {
            format!("RSI6={:.1} 正常区域", rsi.rsi6)
        },
    });
    
    // 8. 布林带下轨支撑
    let boll_support = current_price <= boll.lower * 1.02;
    signals.push(BuySignal {
        name: "BOLL下轨".to_string(),
        triggered: boll_support,
        score: if boll_support { 15 } else { 0 },
        description: if boll_support {
            format!("价格({:.2})接近下轨({:.2})", current_price, boll.lower)
        } else {
            format!("价格距下轨较远")
        },
    });
    
    signals
}

/// 综合分析
pub fn analyze(klines: &[KLine]) -> AnalysisResult {
    if klines.is_empty() {
        return AnalysisResult::default();
    }
    
    let closes: Vec<f64> = klines.iter().map(|k| k.close).collect();
    let current_price = *closes.last().unwrap_or(&0.0);
    
    // 计算各项指标
    let ma = calc_ma(&closes);
    let macd = calc_macd(&closes);
    let kdj = calc_kdj(klines);
    let rsi = calc_rsi_all(&closes);
    let boll = calc_boll(&closes);
    
    // 计算前一天的指标（用于金叉判断）
    let prev_macd = if closes.len() > 1 {
        Some(calc_macd(&closes[..closes.len()-1]))
    } else {
        None
    };
    
    let prev_kdj = if klines.len() > 1 {
        Some(calc_kdj(&klines[..klines.len()-1]))
    } else {
        None
    };
    
    // 检测信号
    let signals = detect_signals(
        current_price,
        &ma,
        &macd,
        &kdj,
        &rsi,
        &boll,
        prev_macd.as_ref(),
        prev_kdj.as_ref(),
    );
    
    // 计算总分
    let total_score: i32 = signals.iter().map(|s| s.score).sum();
    
    // 生成建议
    let recommendation = if total_score >= 80 {
        "★★★★★ 强烈建议买入".to_string()
    } else if total_score >= 60 {
        "★★★★☆ 可以考虑买入".to_string()
    } else if total_score >= 40 {
        "★★★☆☆ 观望等待".to_string()
    } else if total_score >= 20 {
        "★★☆☆☆ 暂不建议".to_string()
    } else {
        "★☆☆☆☆ 不建议买入".to_string()
    };
    
    AnalysisResult {
        ma,
        macd,
        kdj,
        rsi,
        boll,
        signals,
        total_score,
        recommendation,
    }
}
