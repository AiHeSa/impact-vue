//! SQLite 存储和向量索引模块

use std::path::Path;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use super::parser::Chunk;

/// 存储配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreConfig {
    pub db_path: String,
    pub dimension: usize,
}

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub chunk: Chunk,
    pub score: f64,
    pub vector_sim: f64,
    pub term_sim: f64,
}

/// RAG 存储
pub struct RagStore {
    conn: Connection,
    dimension: usize,
}

impl RagStore {
    /// 创建新的存储
    pub fn new(config: &StoreConfig) -> anyhow::Result<Self> {
        let db_path = Path::new(&config.db_path);
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let conn = Connection::open(db_path)?;
        let store = Self {
            conn,
            dimension: config.dimension,
        };
        store.init()?;
        Ok(store)
    }
    
    /// 初始化数据库
    fn init(&self) -> anyhow::Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS chunks (
                id TEXT PRIMARY KEY,
                file_path TEXT NOT NULL,
                language TEXT,
                chunk_type TEXT,
                name TEXT,
                content TEXT,
                start_line INTEGER,
                end_line INTEGER,
                metadata TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_chunks_file ON chunks(file_path);
            CREATE INDEX IF NOT EXISTS idx_chunks_language ON chunks(language);
            CREATE INDEX IF NOT EXISTS idx_chunks_type ON chunks(chunk_type);
            
            CREATE TABLE IF NOT EXISTS vectors (
                id TEXT PRIMARY KEY,
                vector BLOB NOT NULL,
                dimension INTEGER NOT NULL,
                FOREIGN KEY (id) REFERENCES chunks(id) ON DELETE CASCADE
            );
            "
        )?;
        Ok(())
    }
    
    /// 存储代码块
    pub fn store_chunks(&self, chunks: &[Chunk]) -> anyhow::Result<()> {
        let mut stmt = self.conn.prepare(
            "INSERT OR REPLACE INTO chunks (id, file_path, language, chunk_type, name, content, start_line, end_line, metadata) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
        )?;
        
        for chunk in chunks {
            let metadata = serde_json::to_string(&chunk.metadata)?;
            stmt.execute(rusqlite::params![
                chunk.id,
                chunk.file_path,
                chunk.language,
                serde_json::to_string(&chunk.chunk_type)?,
                chunk.name,
                chunk.content,
                chunk.start_line as i64,
                chunk.end_line as i64,
                metadata,
            ])?;
        }
        
        Ok(())
    }
    
    /// 存储向量
    pub fn store_vector(&self, id: &str, vector: &[f32]) -> anyhow::Result<()> {
        let vector_bytes: Vec<u8> = vector.iter().flat_map(|f| f.to_le_bytes()).collect();
        
        self.conn.execute(
            "INSERT OR REPLACE INTO vectors (id, vector, dimension) VALUES (?1, ?2, ?3)",
            rusqlite::params![id, vector_bytes, self.dimension as i64],
        )?;
        
        Ok(())
    }
    
    /// 获取代码块
    pub fn get_chunk(&self, id: &str) -> anyhow::Result<Option<Chunk>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, file_path, language, chunk_type, name, content, start_line, end_line, metadata FROM chunks WHERE id = ?1"
        )?;
        
        let result = stmt.query_row(rusqlite::params![id], |row| {
            let chunk_type_str: String = row.get(2)?;
            let metadata_str: String = row.get(8)?;
            
            Ok(Chunk {
                id: row.get(0)?,
                file_path: row.get(1)?,
                language: row.get(2)?,
                chunk_type: serde_json::from_str(&chunk_type_str).unwrap_or(super::parser::ChunkType::Module),
                name: row.get(4)?,
                content: row.get(5)?,
                start_line: row.get::<_, i64>(6)? as usize,
                end_line: row.get::<_, i64>(7)? as usize,
                metadata: serde_json::from_str(&metadata_str).unwrap_or_default(),
            })
        });
        
        match result {
            Ok(chunk) => Ok(Some(chunk)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
    
    /// 获取所有向量
    pub fn get_all_vectors(&self) -> anyhow::Result<Vec<(String, Vec<f32>)>> {
        let mut stmt = self.conn.prepare("SELECT id, vector FROM vectors")?;
        
        let mut results = Vec::new();
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let vector_bytes: Vec<u8> = row.get(1)?;
            Ok((id, vector_bytes))
        })?;
        
        for row in rows {
            let (id, vector_bytes) = row?;
            let vector: Vec<f32> = vector_bytes
                .chunks_exact(4)
                .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
                .collect();
            results.push((id, vector));
        }
        
        Ok(results)
    }
    
    /// 获取所有代码块
    pub fn get_all_chunks(&self) -> anyhow::Result<Vec<Chunk>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, file_path, language, chunk_type, name, content, start_line, end_line, metadata FROM chunks"
        )?;
        
        let mut chunks = Vec::new();
        let rows = stmt.query_map([], |row| {
            let chunk_type_str: String = row.get(2)?;
            let metadata_str: String = row.get(8)?;
            
            Ok(Chunk {
                id: row.get(0)?,
                file_path: row.get(1)?,
                language: row.get(2)?,
                chunk_type: serde_json::from_str(&chunk_type_str).unwrap_or(super::parser::ChunkType::Module),
                name: row.get(4)?,
                content: row.get(5)?,
                start_line: row.get::<_, i64>(6)? as usize,
                end_line: row.get::<_, i64>(7)? as usize,
                metadata: serde_json::from_str(&metadata_str).unwrap_or_default(),
            })
        })?;
        
        for row in rows {
            chunks.push(row?);
        }
        
        Ok(chunks)
    }
    
    /// 删除文件的所有代码块
    pub fn delete_chunks_by_file(&self, file_path: &str) -> anyhow::Result<()> {
        self.conn.execute(
            "DELETE FROM chunks WHERE file_path = ?1",
            rusqlite::params![file_path],
        )?;
        Ok(())
    }
    
    /// 获取统计信息
    pub fn stats(&self) -> anyhow::Result<StoreStats> {
        let chunk_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM chunks", [], |row| row.get(0)
        )?;
        
        let vector_count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM vectors", [], |row| row.get(0)
        )?;
        
        let file_count: i64 = self.conn.query_row(
            "SELECT COUNT(DISTINCT file_path) FROM chunks", [], |row| row.get(0)
        )?;
        
        Ok(StoreStats {
            chunk_count: chunk_count as usize,
            vector_count: vector_count as usize,
            file_count: file_count as usize,
        })
    }
}

/// 存储统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreStats {
    pub chunk_count: usize,
    pub vector_count: usize,
    pub file_count: usize,
}
