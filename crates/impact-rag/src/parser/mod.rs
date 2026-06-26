//! 代码解析器模块

use serde::{Deserialize, Serialize};

/// 代码块
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Chunk {
    pub id: String,
    pub file_path: String,
    pub language: String,
    pub chunk_type: ChunkType,
    pub name: String,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    pub metadata: ChunkMetadata,
}

/// 代码块类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ChunkType {
    Function,
    Class,
    Module,
    Section,
    Component,
}

/// 代码块元数据
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub struct ChunkMetadata {
    pub package_name: Option<String>,
    pub class_name: Option<String>,
    pub imports: Vec<String>,
    pub comments: Option<String>,
    pub signature: Option<String>,
}

/// 解析器接口
pub trait Parser: Send + Sync {
    /// 解析文件
    fn parse(&self, file_path: &str, content: &str) -> anyhow::Result<Vec<Chunk>>;

    /// 支持的语言
    fn supported_languages(&self) -> Vec<&str>;
}

/// 文件类型检测
pub fn detect_language(file_path: &str) -> &'static str {
    let ext = std::path::Path::new(file_path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    
    match ext {
        "vue" => "vue",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "go" => "go",
        "py" => "python",
        "rs" => "rust",
        "java" => "java",
        "md" => "markdown",
        _ => "text",
    }
}

/// 生成 chunk ID
pub fn generate_chunk_id(file_path: &str, name: &str, chunk_type: &ChunkType) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    file_path.hash(&mut hasher);
    name.hash(&mut hasher);
    chunk_type.hash(&mut hasher);
    
    format!("{:x}", hasher.finish())
}
