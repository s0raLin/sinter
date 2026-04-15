use serde::{Deserialize, Serialize};

// Similar to upickle
pub fn read<'de, T: Deserialize<'de>>(s: &'de str) -> Result<T, serde_json::Error> {
    serde_json::from_str(s)
}

pub fn write<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string(value)
}

pub fn write_pretty<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(value)
}

// Similar to ujson
pub use serde_json::Value;

pub fn read_value(s: &str) -> Result<Value, serde_json::Error> {
    serde_json::from_str(s)
}

pub fn write_value(value: &Value) -> Result<String, serde_json::Error> {
    serde_json::to_string(value)
}

pub fn write_value_pretty(value: &Value) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(value)
}
