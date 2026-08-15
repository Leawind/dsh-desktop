use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub args: BTreeMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub technical_details: Option<String>,
}

impl AppError {
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            args: BTreeMap::new(),
            technical_details: None,
        }
    }

    pub fn arg(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(value) = serde_json::to_value(value) {
            self.args.insert(key.into(), value);
        }
        self
    }

    pub fn technical(mut self, details: impl Into<String>) -> Self {
        self.technical_details = Some(details.into());
        self
    }
}

impl Display for AppError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.code)
    }
}

impl std::error::Error for AppError {}

pub type AppResult<T> = Result<T, AppError>;
