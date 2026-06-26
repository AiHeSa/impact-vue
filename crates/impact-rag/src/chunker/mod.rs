//! 代码分块器模块

pub use super::parser::{Chunk, ChunkMetadata, ChunkType};

/// 分块器接口
pub trait Chunker: Send + Sync {
    /// 分块
    fn chunk(&self, file_path: &str, content: &str) -> anyhow::Result<Vec<Chunk>>;
}

/// 智能分块器
pub struct SmartChunker;

impl SmartChunker {
    pub fn new() -> Self {
        Self
    }
}

impl Chunker for SmartChunker {
    fn chunk(&self, file_path: &str, content: &str) -> anyhow::Result<Vec<Chunk>> {
        let language = super::parser::detect_language(file_path);
        
        match language {
            "vue" => chunk_vue(file_path, content),
            "typescript" | "javascript" => chunk_js_ts(file_path, content),
            _ => chunk_generic(file_path, content),
        }
    }
}

/// Vue SFC 分块
fn chunk_vue(file_path: &str, content: &str) -> anyhow::Result<Vec<Chunk>> {
    let mut chunks = Vec::new();
    let mut chunk_id = 0;
    
    // 提取 script 块
    let re_script = regex::Regex::new(r"(?s)<script\b[^>]*>(.*?)</script>").unwrap();
    if let Some(cap) = re_script.captures(content) {
        let script_content = cap[1].to_string();
        let start = content[..cap.get(1).unwrap().start()].matches('\n').count() + 1;
        let end = start + script_content.matches('\n').count();
        
        chunk_id += 1;
        chunks.push(Chunk {
            id: format!("{}:script:{}", file_path, chunk_id),
            file_path: file_path.to_string(),
            language: "vue".to_string(),
            chunk_type: ChunkType::Component,
            name: "script".to_string(),
            content: script_content,
            start_line: start,
            end_line: end,
            metadata: ChunkMetadata::default(),
        });
    }
    
    // 提取 template 块
    let re_template = regex::Regex::new(r"(?s)<template\b[^>]*>(.*?)</template>").unwrap();
    if let Some(cap) = re_template.captures(content) {
        let template_content = cap[1].to_string();
        let start = content[..cap.get(1).unwrap().start()].matches('\n').count() + 1;
        let end = start + template_content.matches('\n').count();
        
        chunk_id += 1;
        chunks.push(Chunk {
            id: format!("{}:template:{}", file_path, chunk_id),
            file_path: file_path.to_string(),
            language: "vue".to_string(),
            chunk_type: ChunkType::Component,
            name: "template".to_string(),
            content: template_content,
            start_line: start,
            end_line: end,
            metadata: ChunkMetadata::default(),
        });
    }
    
    Ok(chunks)
}

/// JS/TS 分块
fn chunk_js_ts(file_path: &str, content: &str) -> anyhow::Result<Vec<Chunk>> {
    let mut chunks = Vec::new();
    
    // 匹配函数声明
    let re_fn = regex::Regex::new(r#"(?:export\s+)?(?:async\s+)?function\s+(\w+)"#).unwrap();
    for cap in re_fn.captures_iter(content) {
        let name = cap[1].to_string();
        let start = content[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
        
        // 简单估算函数结束行
        let remaining = &content[cap.get(0).unwrap().end()..];
        let brace_count = remaining.chars().take_while(|&c| c != '{').count();
        let end = start + remaining[brace_count..].lines().take(50).count();
        
        chunks.push(Chunk {
            id: format!("{}:fn:{}", file_path, name),
            file_path: file_path.to_string(),
            language: "javascript".to_string(),
            chunk_type: ChunkType::Function,
            name,
            content: content[start..end.min(content.len())].to_string(),
            start_line: start,
            end_line: end,
            metadata: ChunkMetadata::default(),
        });
    }
    
    // 匹配 export const/let/var
    let re_export = regex::Regex::new(r#"export\s+(?:const|let|var)\s+(\w+)"#).unwrap();
    for cap in re_export.captures_iter(content) {
        let name = cap[1].to_string();
        let start = content[..cap.get(0).unwrap().start()].matches('\n').count() + 1;
        
        chunks.push(Chunk {
            id: format!("{}:export:{}", file_path, name),
            file_path: file_path.to_string(),
            language: "javascript".to_string(),
            chunk_type: ChunkType::Function,
            name,
            content: cap[0].to_string(),
            start_line: start,
            end_line: start,
            metadata: ChunkMetadata::default(),
        });
    }
    
    Ok(chunks)
}

/// 通用分块（按行）
fn chunk_generic(file_path: &str, content: &str) -> anyhow::Result<Vec<Chunk>> {
    let lines: Vec<&str> = content.lines().collect();
    let chunk_size = 50;
    let mut chunks = Vec::new();
    
    for (i, chunk_lines) in lines.chunks(chunk_size).enumerate() {
        let start = i * chunk_size + 1;
        let end = start + chunk_lines.len() - 1;
        
        chunks.push(Chunk {
            id: format!("{}:chunk:{}", file_path, i),
            file_path: file_path.to_string(),
            language: "text".to_string(),
            chunk_type: ChunkType::Module,
            name: format!("chunk_{}", i),
            content: chunk_lines.join("\n"),
            start_line: start,
            end_line: end,
            metadata: ChunkMetadata::default(),
        });
    }
    
    Ok(chunks)
}
