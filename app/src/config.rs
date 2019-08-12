use std::collections::HashMap;
use std::path::PathBuf;

use failure::{Error, ResultExt};
use log::*;
use r2d2::Pool;
use serde::{Deserialize, Serialize};

use crate::persistence;

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Config {
    pub db: SledConfig,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct SledConfig {
    pub path: PathBuf,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
enum LogLevel {
    Off,
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl SledConfig {
    pub(crate) fn build(&self) -> Result<Pool<persistence::DocumentConnectionManager>, Error> {
        debug!("Build pool from {:?}", self);

        let manager =
            persistence::DocumentConnectionManager::new(sled::Db::start_default(&self.path)?);

        let builder = r2d2::Pool::builder();

        debug!("Pool builder: {:?}", builder);
        let pool = builder.build(manager).context("build pool")?;

        Ok(pool)
    }
}

#[derive(Deserialize, Debug)]
pub struct EnvLogger {
    level: Option<LogLevel>,
    modules: HashMap<String, LogLevel>,
    timestamp_nanos: bool,
}

impl LogLevel {
    fn to_filter(&self) -> log::LevelFilter {
        match self {
            &LogLevel::Off => log::LevelFilter::Off,
            &LogLevel::Error => log::LevelFilter::Error,
            &LogLevel::Warn => log::LevelFilter::Warn,
            &LogLevel::Info => log::LevelFilter::Info,
            &LogLevel::Debug => log::LevelFilter::Debug,
            &LogLevel::Trace => log::LevelFilter::Trace,
        }
    }
}

impl EnvLogger {
    pub fn builder(&self) -> env_logger::Builder {
        let mut b = env_logger::Builder::from_default_env();
        if let Some(level) = self.level.as_ref() {
            b.filter_level(level.to_filter());
        }

        for (module, level) in self.modules.iter() {
            b.filter_module(&module, level.to_filter());
        }

        b.default_format_timestamp_nanos(self.timestamp_nanos);

        return b;
    }
}
