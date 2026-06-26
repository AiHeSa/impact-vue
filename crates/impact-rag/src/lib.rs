//! RAG (Retrieval-Augmented Generation) 模块
//!
//! 提供代码场景的 RAG 功能：
//! - 代码解析和分块
//! - 向量嵌入（Ollama/OpenAI）
//! - SQLite 存储和向量索引
//! - 混合搜索（向量 + 词项）

pub mod chunker;
pub mod embedding;
pub mod parser;
pub mod searcher;
pub mod store;

pub use chunker::{Chunk, Chunker};
pub use embedding::{Embedder, EmbeddingConfig};
pub use searcher::{SearchResult, Searcher};
pub use store::{RagStore, StoreConfig};
