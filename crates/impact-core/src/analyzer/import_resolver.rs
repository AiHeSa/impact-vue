//! 跨文件 import 解析
//!
//! 解析 import 语句，找到引用的实际文件路径。

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// 解析 import 路径，返回实际文件路径
pub fn resolve_import(current_file: &Path, import_source: &str) -> Option<PathBuf> {
    resolve_import_with_aliases(current_file, import_source, &HashMap::new())
}

/// 解析 import 路径（支持 alias）
pub fn resolve_import_with_aliases(
    current_file: &Path,
    import_source: &str,
    aliases: &HashMap<String, String>,
) -> Option<PathBuf> {
    let parent = current_file.parent()?;
    
    // 处理 alias 路径（如 @/stores/xxx）
    let resolved_source = resolve_alias(import_source, aliases);
    
    // 如果是绝对路径（alias 解析后的结果）
    if let Some(absolute) = &resolved_source {
        let candidate = PathBuf::from(absolute);
        if candidate.exists() && candidate.is_file() {
            return Some(candidate);
        }
        // 尝试添加扩展名
        for ext in &["ts", "js", "vue", "tsx", "jsx"] {
            let with_ext = candidate.with_extension(ext);
            if with_ext.exists() && with_ext.is_file() {
                return Some(with_ext);
            }
        }
        // 尝试 index 文件
        for ext in &["ts", "js", "vue"] {
            let index = candidate.join(format!("index.{}", ext));
            if index.exists() && index.is_file() {
                return Some(index);
            }
        }
    }
    
    // 只处理相对路径
    if !import_source.starts_with('.') {
        return None;
    }
    
    let candidate = parent.join(import_source);
    
    // 尝试直接路径
    if candidate.exists() && candidate.is_file() {
        return Some(candidate);
    }
    
    // 尝试添加扩展名
    for ext in &["ts", "js", "vue", "tsx", "jsx"] {
        let with_ext = candidate.with_extension(ext);
        if with_ext.exists() && with_ext.is_file() {
            return Some(with_ext);
        }
    }
    
    // 尝试 index 文件
    for ext in &["ts", "js", "vue"] {
        let index = candidate.join(format!("index.{}", ext));
        if index.exists() && index.is_file() {
            return Some(index);
        }
    }
    
    None
}

/// 解析 alias 路径
fn resolve_alias(import_source: &str, aliases: &HashMap<String, String>) -> Option<String> {
    for (alias, target) in aliases {
        let alias_prefix = if alias.ends_with('/') {
            alias.clone()
        } else {
            format!("{}/", alias)
        };
        
        if import_source == alias.trim_end_matches('/') || import_source.starts_with(&alias_prefix) {
            let rest = if import_source.starts_with(&alias_prefix) {
                &import_source[alias_prefix.len()..]
            } else {
                ""
            };
            
            let resolved = if target.ends_with('/') {
                format!("{}{}", target, rest)
            } else {
                format!("{}/{}", target, rest)
            };
            
            return Some(resolved);
        }
    }
    None
}

/// 从文件内容中提取所有 import 源
pub fn extract_import_sources(content: &str) -> Vec<String> {
    let mut sources = Vec::new();
    
    // import ... from 'xxx'
    let re = regex::Regex::new(r#"from\s+['"]([^'"]+)['"]"#).unwrap();
    for cap in re.captures_iter(content) {
        sources.push(cap[1].to_string());
    }
    
    // import 'xxx'
    let re_import = regex::Regex::new(r#"import\s+['"]([^'"]+)['"]"#).unwrap();
    for cap in re_import.captures_iter(content) {
        sources.push(cap[1].to_string());
    }
    
    // require('xxx')
    let re_require = regex::Regex::new(r#"require\s*\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap();
    for cap in re_require.captures_iter(content) {
        sources.push(cap[1].to_string());
    }
    
    sources
}

/// 递归收集所有依赖文件
pub fn collect_dependencies(
    entry: &Path,
    max_depth: usize,
) -> Vec<PathBuf> {
    let mut result = Vec::new();
    let mut visited = std::collections::HashSet::new();
    collect_recursive(entry, max_depth, &mut visited, &mut result);
    result
}

fn collect_recursive(
    file: &Path,
    depth: usize,
    visited: &mut std::collections::HashSet<PathBuf>,
    result: &mut Vec<PathBuf>,
) {
    if depth == 0 || !visited.insert(file.to_path_buf()) {
        return;
    }
    
    let content = match std::fs::read_to_string(file) {
        Ok(c) => c,
        Err(_) => return,
    };
    
    let sources = extract_import_sources(&content);
    
    for source in sources {
        if let Some(resolved) = resolve_import(file, &source) {
            result.push(resolved.clone());
            collect_recursive(&resolved, depth - 1, visited, result);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    
    #[test]
    fn test_extract_import_sources() {
        let content = r#"
import { ref } from 'vue'
import MyComponent from './MyComponent.vue'
import './styles.css'
const utils = require('./utils')
"#;
        let sources = extract_import_sources(content);
        assert_eq!(sources.len(), 4);
        assert!(sources.contains(&"vue".to_string()));
        assert!(sources.contains(&"./MyComponent.vue".to_string()));
        assert!(sources.contains(&"./styles.css".to_string()));
        assert!(sources.contains(&"./utils".to_string()));
    }
}
