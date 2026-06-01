//! JS/TS 静态分析模块
//!
//! 提供通用的 JavaScript/TypeScript 代码分析能力，包括：
//! - 函数/方法提取
//! - import 语句解析
//! - 数据字段识别（ref/reactive 模式）
//! - 事件识别（emit 模式）
//! - 平衡括号提取

pub mod script_analyzer;

pub use script_analyzer::{JsTsAnalyzer, extract_balanced_brace};
