use std::path::Path;

use impact_core::model::{FrameworkAnalysisResult, SourceFileIr, Target};

pub trait FrameworkAdapter: Send + Sync {
    fn name(&self) -> &'static str;

    fn detect(&self, file: &Path, content: &str) -> bool;

    fn parse_file(
        &self,
        file: &Path,
        content: &str,
    ) -> anyhow::Result<SourceFileIr>;

    /// 解析文件及其所有依赖
    /// 默认实现只解析单个文件
    fn parse_file_with_deps(
        &self,
        file: &Path,
        content: &str,
        _max_depth: usize,
    ) -> anyhow::Result<Vec<SourceFileIr>> {
        let ir = self.parse_file(file, content)?;
        Ok(vec![ir])
    }

    fn analyze(
        &self,
        files: Vec<SourceFileIr>,
        target: &Target,
    ) -> anyhow::Result<FrameworkAnalysisResult>;
}

pub enum Direction {
    Upstream,
    Downstream,
    Both,
}

impl Direction {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_lowercase().as_str() {
            "upstream" => Some(Self::Upstream),
            "downstream" => Some(Self::Downstream),
            "both" => Some(Self::Both),
            _ => None,
        }
    }
}

pub struct AdapterRegistry {
    adapters: Vec<Box<dyn FrameworkAdapter>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self { adapters: Vec::new() }
    }

    pub fn register(&mut self, adapter: Box<dyn FrameworkAdapter>) {
        self.adapters.push(adapter);
    }

    pub fn select(
        &self,
        framework: Option<&str>,
        file: &Path,
        content: &str,
    ) -> Option<&dyn FrameworkAdapter> {
        if let Some(name) = framework {
            self.adapters.iter().find(|a| a.name() == name).map(|a| a.as_ref())
        } else {
            self.adapters.iter().find(|a| a.detect(file, content)).map(|a| a.as_ref())
        }
    }
}
