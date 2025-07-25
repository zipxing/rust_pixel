// RustPixel
// copyright zipxing@hotmail.com 2022~2024


//! Log module provides various log functions, reference
//! https://docs.rs/log4rs


#[cfg(not(target_arch = "wasm32"))]
use crate::util::get_abs_path;
use log::LevelFilter;

#[cfg(not(target_arch = "wasm32"))]
use log4rs::{
    append::file::FileAppender,
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
    filter::threshold::ThresholdFilter,
};

/// init logs system
#[allow(unused)]
pub fn init_log(level: LevelFilter, file_path: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        wasm_logger::init(wasm_logger::Config::default());
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let fpstr = get_abs_path(file_path);
        let logfile = FileAppender::builder()
            .encoder(Box::new(PatternEncoder::new(
                "{d(%Y-%m-%d %H:%M:%S)} {l} {t} {m}{n}\n",
            )))
            .build(fpstr)
            .unwrap();
        let config = Config::builder()
            .appender(
                Appender::builder()
                    .filter(Box::new(ThresholdFilter::new(level)))
                    .build("logfile", Box::new(logfile)),
            )
            .build(
                Root::builder()
                    .appender("logfile")
                    .build(level),
            )
            .unwrap();
        let _handle = log4rs::init_config(config).unwrap();
    }
}
