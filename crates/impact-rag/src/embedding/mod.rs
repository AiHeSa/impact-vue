//! 向量嵌入模块

use serde::{Deserialize, Serialize};

use serde_json::Value;

/// 嵌入配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub provider: EmbeddingProvider,
    pub model: String,
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub dimension: usize,
}

/// 嵌入提供者
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmbeddingProvider {
    OpenAI,
    Ollama,
    Local,
}

/// 嵌入接口
pub trait Embedder: Send + Sync {
    /// 生成嵌入向量
    fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>>;
    
    /// 批量生成嵌入向量
    fn embed_batch(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>>;
    
    /// 向量维度
    fn dimension(&self) -> usize;
}

/// OpenAI 嵌入
pub struct OpenAIEmbedder {
    config: EmbeddingConfig,
}

impl OpenAIEmbedder {
    pub fn new(config: EmbeddingConfig) -> Self {
        Self { config }
    }
}

impl Embedder for OpenAIEmbedder {
    fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("OpenAI API key not set. Set RAG_API_KEY or OPENAI_API_KEY env var"))?;
        
        let base_url = self.config.base_url.as_deref()
            .unwrap_or("https://api.openai.com/v1");
        
        let url = format!("{}/embeddings", base_url);
        
        let body = serde_json::json!({
            "model": self.config.model,
            "input": text,
        });
        
        let client = reqwest::blocking::Client::new();
        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()?;
        
        let json: serde_json::Value = resp.json()?;
        let embedding = json["data"][0]["embedding"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Invalid response format"))?
            .iter()
            .map(|v: &Value| v.as_f64().unwrap_or(0.0) as f32)
            .collect();
        
        Ok(embedding)
    }
    
    fn embed_batch(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| anyhow::anyhow!("OpenAI API key not set"))?;
        
        let base_url = self.config.base_url.as_deref()
            .unwrap_or("https://api.openai.com/v1");
        
        let url = format!("{}/embeddings", base_url);
        
        let body = serde_json::json!({
            "model": self.config.model,
            "input": texts,
        });
        
        let client = reqwest::blocking::Client::new();
        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()?;
        
        let json: serde_json::Value = resp.json()?;
        let data = json["data"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Invalid response format"))?;
        
        let mut results = Vec::new();
        for item in data {
            let embedding = item["embedding"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("Invalid embedding format"))?
                .iter()
                .map(|v: &Value| v.as_f64().unwrap_or(0.0) as f32)
                .collect();
            results.push(embedding);
        }
        
        Ok(results)
    }
    
    fn dimension(&self) -> usize {
        self.config.dimension
    }
}

/// Ollama 嵌入
pub struct OllamaEmbedder {
    config: EmbeddingConfig,
}

impl OllamaEmbedder {
    pub fn new(config: EmbeddingConfig) -> Self {
        Self { config }
    }
}

impl Embedder for OllamaEmbedder {
    fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let base_url = self.config.base_url.as_deref()
            .unwrap_or("http://localhost:11434");
        
        let url = format!("{}/api/embeddings", base_url);
        
        let body = serde_json::json!({
            "model": self.config.model,
            "prompt": text,
        });
        
        let client = reqwest::blocking::Client::new();
        let resp = client
            .post(&url)
            .json(&body)
            .send()?;
        
        let json: serde_json::Value = resp.json()?;
        let embedding = json["embedding"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Invalid response format"))?
            .iter()
            .map(|v: &Value| v.as_f64().unwrap_or(0.0) as f32)
            .collect();
        
        Ok(embedding)
    }
    
    fn embed_batch(&self, texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
        let mut results = Vec::new();
        for text in texts {
            results.push(self.embed(text)?);
        }
        Ok(results)
    }
    
    fn dimension(&self) -> usize {
        self.config.dimension
    }
}

/// 创建嵌入器
pub fn create_embedder(config: EmbeddingConfig) -> Box<dyn Embedder> {
    match config.provider {
        EmbeddingProvider::OpenAI => Box::new(OpenAIEmbedder::new(config)),
        EmbeddingProvider::Ollama => Box::new(OllamaEmbedder::new(config)),
        EmbeddingProvider::Local => {
            panic!("Local embedding not implemented yet")
        }
    }
}

/// 从环境变量加载 API key
pub fn load_api_key() -> Option<String> {
    std::env::var("RAG_API_KEY").ok()
        .or_else(|| std::env::var("OPENAI_API_KEY").ok())
}
