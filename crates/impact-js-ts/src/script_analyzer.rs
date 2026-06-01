//! JS/TS 脚本分析器
//!
//! 提供通用的 JavaScript/TypeScript 代码分析能力。

use impact_core::model::{
    DataFieldIr, EventIr, ExecutableIr, ExecutableKind, ImportIr,
};
use regex::Regex;

/// JS/TS 脚本分析器
pub struct JsTsAnalyzer;

impl JsTsAnalyzer {
    /// 提取 ref/reactive 响应式数据字段
    pub fn extract_reactive_fields(script: &str) -> Vec<DataFieldIr> {
        if script.is_empty() {
            return Vec::new();
        }

        let mut fields = Vec::new();

        // 匹配 ref() 模式: const count = ref(0)
        let re_ref = Regex::new(r#"(\w+)\s*=\s*ref\s*\(\s*"#).unwrap();
        for cap in re_ref.captures_iter(script) {
            fields.push(DataFieldIr {
                name: cap[1].to_string(),
                default_value: None,
                line: 0,
            });
        }

        // 匹配 reactive() 模式: const state = reactive({ ... })
        let re_reactive = Regex::new(r#"(\w+)\s*=\s*reactive\s*\(\s*\{([^}]*)\}\s*\)"#).unwrap();
        for cap in re_reactive.captures_iter(script) {
            let body = &cap[2];
            let re_inner = Regex::new(r#"(\w+)\s*:"#).unwrap();
            for inner in re_inner.captures_iter(body) {
                fields.push(DataFieldIr {
                    name: inner[1].to_string(),
                    default_value: None,
                    line: 0,
                });
            }
        }

        fields
    }

    /// 提取函数/方法
    pub fn extract_functions(script: &str) -> Vec<ExecutableIr> {
        if script.is_empty() {
            return Vec::new();
        }

        let mut methods = Vec::new();

        // 匹配箭头函数: const fn = () => { ... }
        let re_arrow = Regex::new(r#"(?:const|let|var)\s+(\w+)\s*=\s*\([^)]*\)\s*=>\s*\{"#).unwrap();
        for cap in re_arrow.captures_iter(script) {
            let name = cap[1].to_string();
            let fn_start = cap.get(0).unwrap().end();
            let fn_body = extract_balanced_brace(&script[fn_start..]);
            methods.push(ExecutableIr {
                kind: ExecutableKind::Method,
                name,
                body: fn_body,
                line: 0,
            });
        }

        // 匹配函数声明: function fn() { ... }
        let re_fn = Regex::new(r#"\bfunction\s+(\w+)\s*\("#).unwrap();
        for cap in re_fn.captures_iter(script) {
            let name = cap[1].to_string();
            let fn_start = cap.get(0).unwrap().end();
            let fn_body = extract_balanced_brace(&script[fn_start..]);
            methods.push(ExecutableIr {
                kind: ExecutableKind::Method,
                name,
                body: fn_body,
                line: 0,
            });
        }

        methods
    }

    /// 提取 import 语句
    pub fn extract_imports(script: &str) -> Vec<ImportIr> {
        if script.is_empty() {
            return Vec::new();
        }

        let mut imports = Vec::new();

        let re_import = Regex::new(
            r#"import\s+(?:\{\s*([^}]*)\s*\}|\*\s+as\s+(\w+)|(\w+))\s+from\s+['"]([^'"]+)['"]"#,
        )
        .unwrap();

        for cap in re_import.captures_iter(script) {
            let source = cap.get(4).map(|m| m.as_str().to_string()).unwrap_or_default();

            if let Some(name) = cap.get(3) {
                imports.push(ImportIr {
                    source,
                    imported_name: Some(name.as_str().to_string()),
                    is_default: true,
                    line: 0,
                });
            } else if let Some(name) = cap.get(2) {
                imports.push(ImportIr {
                    source,
                    imported_name: Some(name.as_str().to_string()),
                    is_default: false,
                    line: 0,
                });
            } else if let Some(names) = cap.get(1) {
                let re_name = Regex::new(r#"(\w+)"#).unwrap();
                for n in re_name.captures_iter(names.as_str()) {
                    imports.push(ImportIr {
                        source: source.clone(),
                        imported_name: Some(n[1].to_string()),
                        is_default: false,
                        line: 0,
                    });
                }
            }
        }

        imports
    }

    /// 提取 emit 事件
    pub fn extract_events(script: &str) -> Vec<EventIr> {
        if script.is_empty() {
            return Vec::new();
        }

        let mut events = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // 匹配所有 emit('xxx') 模式
        let re = Regex::new(r#"(\$?)emit\s*\(\s*['"]([^'"]+)['"]"#).unwrap();
        for cap in re.captures_iter(script) {
            let name = cap[2].to_string();
            if seen.insert(name.clone()) {
                events.push(EventIr {
                    event_name: name,
                    handler_name: String::new(),
                    line: 0,
                });
            }
        }

        events
    }

    /// 提取 this.xxx 数据访问
    pub fn extract_this_refs(body: &str) -> Vec<String> {
        let re = Regex::new(r#"this\.(\w+)"#).unwrap();
        let exclude = [
            "data", "methods", "computed", "props", "emit", "$emit", "$refs",
            "$el", "$options", "$parent", "$root", "$children", "$slots",
            "$scopedSlots", "$attrs", "$listeners", "$watch", "$set",
            "$delete", "$nextTick", "$on", "$once", "$off", "$mount",
            "$forceUpdate", "$destroy",
        ];
        let mut refs = Vec::new();
        for cap in re.captures_iter(body) {
            let name = cap[1].to_string();
            if !exclude.contains(&name.as_str()) && !refs.contains(&name) {
                refs.push(name);
            }
        }
        refs
    }

    /// 提取数据读写访问
    pub fn extract_data_access(body: &str) -> (Vec<String>, Vec<String>) {
        let mut reads = Vec::new();
        let mut writes = Vec::new();

        // this.data.get('field')
        let re_read = Regex::new(r#"this\.data\.get\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap();
        for cap in re_read.captures_iter(body) {
            reads.push(cap[1].to_string());
        }

        // this.data.set('field', value)
        let re_write = Regex::new(r#"this\.data\.set\(\s*['"]([^'"]+)['"]\s*,"#).unwrap();
        for cap in re_write.captures_iter(body) {
            writes.push(cap[1].to_string());
        }

        // 直接 this.xxx 访问（Vue 2 代理）
        let re_read_direct = Regex::new(r#"this\.(\w+)"#).unwrap();
        for cap in re_read_direct.captures_iter(body) {
            let field = cap[1].to_string();
            if !is_excluded_field(&field) {
                reads.push(field);
            }
        }

        // this.xxx = value / this.xxx++ / this.xxx--
        let re_write_direct = Regex::new(r#"this\.(\w+)\s*(?:=|\+\+|--)"#).unwrap();
        for cap in re_write_direct.captures_iter(body) {
            let field = cap[1].to_string();
            if !is_excluded_field(&field) {
                writes.push(field);
            }
        }

        (reads, writes)
    }

    /// 提取方法调用
    pub fn extract_method_calls(body: &str) -> Vec<String> {
        let mut calls = Vec::new();
        let re = Regex::new(r#"this\.(\w+)\(\s*\)"#).unwrap();
        for cap in re.captures_iter(body) {
            calls.push(cap[1].to_string());
        }
        calls
    }

    /// 提取 await API 调用
    pub fn extract_await_calls(body: &str) -> Vec<String> {
        let mut api_calls = Vec::new();
        let re = Regex::new(r#"await\s+(?:[\w.]+\.)?(\w+)\s*\("#).unwrap();
        for cap in re.captures_iter(body) {
            api_calls.push(cap[1].to_string());
        }
        api_calls
    }

    /// 检测是否是 async 方法
    pub fn is_async_method(body: &str) -> bool {
        body.contains("await ")
    }

    /// 提取 .then() 链中的写入
    pub fn extract_then_writes(body: &str) -> Vec<String> {
        let mut writes = Vec::new();

        let re = Regex::new(r#"\.then\s*\([^)]*\)\s*\{[^}]*this\.(\w+)\s*="#).unwrap();
        for cap in re.captures_iter(body) {
            writes.push(cap[1].to_string());
        }

        let re2 = Regex::new(r#"\.then\s*\([^)]*\)\s*=>\s*this\.(\w+)\s*="#).unwrap();
        for cap in re2.captures_iter(body) {
            writes.push(cap[1].to_string());
        }

        writes
    }

    /// 提取 .catch() 链中的写入
    pub fn extract_catch_writes(body: &str) -> Vec<String> {
        let mut writes = Vec::new();
        let re = Regex::new(r#"\.catch\s*\([^)]*\)\s*\{[^}]*this\.(\w+)\s*="#).unwrap();
        for cap in re.captures_iter(body) {
            writes.push(cap[1].to_string());
        }
        writes
    }
}

/// 提取平衡括号内容
pub fn extract_balanced_brace(s: &str) -> String {
    let mut depth = 1;
    for (i, ch) in s.char_indices() {
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth -= 1;
            if depth == 0 {
                return s[..i].to_string();
            }
        }
    }
    String::new()
}

/// 检查是否是排除的字段
fn is_excluded_field(field: &str) -> bool {
    const EXCLUDED: &[&str] = &[
        "data", "methods", "computed", "props", "emit", "$emit", "$refs",
        "$el", "$options", "$parent", "$root", "$children", "$slots",
        "$scopedSlots", "$attrs", "$listeners", "$watch", "$set",
        "$delete", "$nextTick", "$on", "$once", "$off", "$mount",
        "$forceUpdate", "$destroy",
    ];
    EXCLUDED.contains(&field)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_reactive_fields_ref() {
        let script = r#"
import { ref } from 'vue'
const count = ref(0)
const name = ref('hello')
"#;
        let fields = JsTsAnalyzer::extract_reactive_fields(script);
        assert_eq!(fields.len(), 2);
        assert!(fields.iter().any(|f| f.name == "count"));
        assert!(fields.iter().any(|f| f.name == "name"));
    }

    #[test]
    fn test_extract_functions() {
        let script = r#"
function increment() {}
const reset = () => {}
"#;
        let methods = JsTsAnalyzer::extract_functions(script);
        assert_eq!(methods.len(), 2);
        assert!(methods.iter().any(|m| m.name == "increment"));
        assert!(methods.iter().any(|m| m.name == "reset"));
    }

    #[test]
    fn test_extract_imports() {
        let script = r#"
import { ref, computed } from 'vue'
import DefaultComp from './Comp.vue'
"#;
        let imports = JsTsAnalyzer::extract_imports(script);
        assert_eq!(imports.len(), 3);
    }

    #[test]
    fn test_extract_events() {
        let script = r#"this.$emit('update', value)"#;
        let events = JsTsAnalyzer::extract_events(script);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_name, "update");
    }

    #[test]
    fn test_extract_data_access() {
        let body = "this.data.set('count', this.data.get('count') + 1)";
        let (reads, writes) = JsTsAnalyzer::extract_data_access(body);
        assert!(reads.contains(&"count".to_string()));
        assert!(writes.contains(&"count".to_string()));
    }

    #[test]
    fn test_extract_method_calls() {
        let body = "this.increment()";
        let calls = JsTsAnalyzer::extract_method_calls(body);
        assert_eq!(calls, vec!["increment"]);
    }

    #[test]
    fn test_extract_await_calls() {
        let body = "const res = await api.getData()";
        let calls = JsTsAnalyzer::extract_await_calls(body);
        assert_eq!(calls, vec!["getData"]);
    }

    #[test]
    fn test_extract_this_refs() {
        let body = "this.count + this.name";
        let refs = JsTsAnalyzer::extract_this_refs(body);
        assert!(refs.contains(&"count".to_string()));
        assert!(refs.contains(&"name".to_string()));
    }
}
