use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub id: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub snippet: String,
    pub reason: String,
}

impl Evidence {
    pub fn new(file: String, line: usize, column: usize, snippet: String, reason: String) -> Self {
        let id = format!("{}:{}:{}", file, line, column);
        Self { id, file, line, column, snippet, reason }
    }
}
