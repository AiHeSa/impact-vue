//! 混合搜索器模块

use serde::{Deserialize, Serialize};

use super::parser::Chunk;
use super::store::RagStore;

/// 搜索选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    pub top_k: usize,
    pub min_score: f64,
    pub vector_weight: f64,
    pub term_weight: f64,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            top_k: 10,
            min_score: 0.1,
            vector_weight: 0.7,
            term_weight: 0.3,
        }
    }
}

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub chunk: Chunk,
    pub score: f64,
    pub vector_sim: f64,
    pub term_sim: f64,
}

/// 搜索器
pub struct Searcher {
    store: RagStore,
}

impl Searcher {
    pub fn new(store: RagStore) -> Self {
        Self { store }
    }
    
    /// 向量搜索
    pub fn search_vector(
        &self,
        query_vector: &[f32],
        options: &SearchOptions,
    ) -> anyhow::Result<Vec<SearchResult>> {
        let all_vectors = self.store.get_all_vectors()?;
        let mut results = Vec::new();
        
        for (id, vector) in all_vectors {
            let similarity = cosine_similarity(query_vector, &vector);
            
            if similarity >= options.min_score as f32 {
                if let Some(chunk) = self.store.get_chunk(&id)? {
                    results.push(SearchResult {
                        chunk,
                        score: similarity as f64,
                        vector_sim: similarity as f64,
                        term_sim: 0.0,
                    });
                }
            }
        }
        
        // 按分数排序
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(options.top_k);
        
        Ok(results)
    }
    
    /// 词项搜索
    pub fn search_term(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> anyhow::Result<Vec<SearchResult>> {
        let query_lower = query.to_lowercase();
        let query_tokens: Vec<&str> = query_lower.split_whitespace().collect();
        
        // 从 store 获取所有 chunks
        let all_chunks = self.get_all_chunks()?;
        let mut results = Vec::new();
        
        for chunk in all_chunks {
            let content_lower = chunk.content.to_lowercase();
            let name_lower = chunk.name.to_lowercase();
            let file_lower = chunk.file_path.to_lowercase();
            
            let mut match_count = 0;
            for token in &query_tokens {
                if content_lower.contains(token) || name_lower.contains(token) || file_lower.contains(token) {
                    match_count += 1;
                }
            }
            
            if match_count > 0 {
                let score = match_count as f64 / query_tokens.len() as f64;
                if score >= options.min_score {
                    results.push(SearchResult {
                        chunk,
                        score,
                        vector_sim: 0.0,
                        term_sim: score,
                    });
                }
            }
        }
        
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(options.top_k);
        
        Ok(results)
    }
    
    /// 混合搜索
    pub fn search_hybrid(
        &self,
        query: &str,
        query_vector: &[f32],
        options: &SearchOptions,
    ) -> anyhow::Result<Vec<SearchResult>> {
        let vector_results = self.search_vector(query_vector, options)?;
        let term_results = self.search_term(query, options)?;
        
        // 合并结果
        let mut merged: std::collections::HashMap<String, SearchResult> = std::collections::HashMap::new();
        
        for r in vector_results {
            merged.insert(r.chunk.id.clone(), SearchResult {
                chunk: r.chunk,
                score: r.vector_sim * options.vector_weight,
                vector_sim: r.vector_sim,
                term_sim: 0.0,
            });
        }
        
        for r in term_results {
            if let Some(existing) = merged.get_mut(&r.chunk.id) {
                existing.score += r.term_sim * options.term_weight;
                existing.term_sim = r.term_sim;
            } else {
                merged.insert(r.chunk.id.clone(), SearchResult {
                    chunk: r.chunk,
                    score: r.term_sim * options.term_weight,
                    vector_sim: 0.0,
                    term_sim: r.term_sim,
                });
            }
        }
        
        let mut results: Vec<SearchResult> = merged.into_values().collect();
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(options.top_k);
        
        Ok(results)
    }
    
    /// 获取所有代码块
    fn get_all_chunks(&self) -> anyhow::Result<Vec<Chunk>> {
        self.store.get_all_chunks()
    }
}

/// 余弦相似度
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    
    dot_product / (norm_a * norm_b)
}
