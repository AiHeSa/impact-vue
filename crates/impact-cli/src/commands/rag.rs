use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};

use impact_rag::chunker::{Chunker, SmartChunker};
use impact_rag::embedding::{self, EmbeddingConfig, EmbeddingProvider};
use impact_rag::store::{RagStore, StoreConfig};

#[derive(Args)]
pub struct RagArgs {
    #[command(subcommand)]
    pub command: RagCommand,
}

#[derive(Subcommand)]
pub enum RagCommand {
    /// 索引项目文件
    Index(IndexArgs),
    /// 搜索代码
    Search(SearchArgs),
}

#[derive(Args)]
pub struct IndexArgs {
    /// 要索引的目录或文件
    pub path: String,

    /// 嵌入提供者 (openai, ollama)
    #[arg(long, default_value = "ollama")]
    pub provider: String,

    /// 嵌入模型
    #[arg(long, default_value = "nomic-embed-text")]
    pub model: String,

    /// Ollama URL
    #[arg(long, default_value = "http://localhost:11434")]
    pub ollama_url: String,

    /// OpenAI base URL
    #[arg(long)]
    pub openai_url: Option<String>,

    /// 向量维度
    #[arg(long, default_value = "768")]
    pub dimension: usize,

    /// 数据库路径
    #[arg(long, default_value = "~/.impact/rag.db")]
    pub db: String,
}

#[derive(Args)]
pub struct SearchArgs {
    /// 搜索查询
    pub query: String,

    /// 返回结果数量
    #[arg(long, default_value = "10")]
    pub top_k: usize,

    /// 最小分数
    #[arg(long, default_value = "0.1")]
    pub min_score: f64,

    /// 向量搜索权重
    #[arg(long, default_value = "0.7")]
    pub vector_weight: f64,

    /// 词项搜索权重
    #[arg(long, default_value = "0.3")]
    pub term_weight: f64,

    /// 嵌入提供者 (openai, ollama)
    #[arg(long, default_value = "ollama")]
    pub provider: String,

    /// 嵌入模型
    #[arg(long, default_value = "nomic-embed-text")]
    pub model: String,

    /// Ollama URL
    #[arg(long, default_value = "http://localhost:11434")]
    pub ollama_url: String,

    /// OpenAI base URL
    #[arg(long)]
    pub openai_url: Option<String>,

    /// 向量维度
    #[arg(long, default_value = "768")]
    pub dimension: usize,

    /// 数据库路径
    #[arg(long, default_value = "~/.impact/rag.db")]
    pub db: String,

    /// 输出格式 (text, json)
    #[arg(long, default_value = "text")]
    pub format: String,
}

pub fn run(args: &RagArgs) -> anyhow::Result<()> {
    match &args.command {
        RagCommand::Index(index_args) => run_index(index_args),
        RagCommand::Search(search_args) => run_search(search_args),
    }
}

fn run_index(args: &IndexArgs) -> anyhow::Result<()> {
    let path = Path::new(&args.path);
    if !path.exists() {
        anyhow::bail!("Path not found: {}", args.path);
    }

    // 创建存储
    let db_path = shellexpand::tilde(&args.db).to_string();
    let store_config = StoreConfig {
        db_path: db_path.clone(),
        dimension: args.dimension,
    };
    let store = RagStore::new(&store_config)?;

    // 创建分块器
    let chunker = SmartChunker::new();

    // 收集文件
    let files = collect_files(path)?;
    println!("Found {} files to index", files.len());

    // 分块并存储
    let mut total_chunks = 0;
    for file in &files {
        let content = match std::fs::read_to_string(file) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let file_path = file.to_string_lossy().to_string();
        match chunker.chunk(&file_path, &content) {
            Ok(chunks) => {
                if !chunks.is_empty() {
                    store.store_chunks(&chunks)?;
                    total_chunks += chunks.len();
                }
            }
            Err(e) => {
                tracing::warn!("Failed to chunk {}: {}", file_path, e);
            }
        }
    }

    println!("Indexed {} chunks from {} files", total_chunks, files.len());
    println!("Database: {}", db_path);

    Ok(())
}

fn run_search(args: &SearchArgs) -> anyhow::Result<()> {
    let db_path = shellexpand::tilde(&args.db).to_string();
    
    // 检查数据库是否存在
    if !Path::new(&db_path).exists() {
        anyhow::bail!("Database not found: {}. Run 'impact rag index' first.", db_path);
    }

    // 创建存储
    let store_config = StoreConfig {
        db_path,
        dimension: args.dimension,
    };
    let store = RagStore::new(&store_config)?;

    // 创建嵌入器
    let api_key = embedding::load_api_key();
    let provider = match args.provider.as_str() {
        "openai" => EmbeddingProvider::OpenAI,
        "ollama" => EmbeddingProvider::Ollama,
        _ => anyhow::bail!("Unknown provider: {}", args.provider),
    };

    let base_url = if provider == EmbeddingProvider::Ollama {
        Some(args.ollama_url.clone())
    } else {
        args.openai_url.clone()
    };

    let embedder = embedding::create_embedder(EmbeddingConfig {
        provider,
        model: args.model.clone(),
        api_key,
        base_url,
        dimension: args.dimension,
    });

    // 搜索
    let searcher = impact_rag::searcher::Searcher::new(store);
    let options = impact_rag::searcher::SearchOptions {
        top_k: args.top_k,
        min_score: args.min_score,
        vector_weight: args.vector_weight,
        term_weight: args.term_weight,
    };

    let results = if args.vector_weight > 0.0 {
        // 需要向量搜索
        let query_vector = embedder.embed(&args.query)?;
        searcher.search_hybrid(&args.query, &query_vector, &options)?
    } else {
        // 只用词项搜索
        searcher.search_term(&args.query, &options)?
    };

    // 输出结果
    if args.format == "json" {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        println!("# RAG Search Results\n");
        println!("**Query**: `{}`", args.query);
        println!("**Results**: {}\n", results.len());

        for (i, result) in results.iter().enumerate() {
            println!("## Result {}\n", i + 1);
            println!("- **File**: `{}`", result.chunk.file_path);
            println!("- **Name**: `{}`", result.chunk.name);
            println!("- **Type**: `{:?}`", result.chunk.chunk_type);
            println!("- **Score**: {:.3}", result.score);
            println!("- **Vector Sim**: {:.3}", result.vector_sim);
            println!("- **Term Sim**: {:.3}", result.term_sim);
            println!("```{}", result.chunk.language);
            println!("{}", truncate(&result.chunk.content, 500));
            println!("```\n");
        }
    }

    Ok(())
}

fn collect_files(path: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let extensions = ["vue", "ts", "js", "tsx", "jsx", "json"];

    if path.is_file() {
        files.push(path.to_path_buf());
    } else {
        for entry in walkdir::WalkDir::new(path)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.') && name != "node_modules" && name != "target"
            })
        {
            let entry = entry?;
            if entry.file_type().is_file() {
                if let Some(ext) = entry.path().extension() {
                    if extensions.contains(&ext.to_string_lossy().as_ref()) {
                        files.push(entry.path().to_path_buf());
                    }
                }
            }
        }
    }

    Ok(files)
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}
