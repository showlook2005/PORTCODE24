use std::fmt::Display;

pub trait Logger: Send + Sync {
    fn info(&self, msg: &str, keys_and_values: &[(&str, &dyn Display)]);
    fn error(&self, err: &dyn std::error::Error, msg: &str, keys_and_values: &[(&str, &dyn Display)]);
}

pub struct DiscardLogger;

impl Logger for DiscardLogger {
    fn info(&self, _msg: &str, _keys_and_values: &[(&str, &dyn Display)]) {}
    fn error(&self, _err: &dyn std::error::Error, _msg: &str, _keys_and_values: &[(&str, &dyn Display)]) {}
}
