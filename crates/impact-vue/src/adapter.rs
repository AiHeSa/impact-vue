use std::collections::HashMap;
use std::path::{Path, PathBuf};

use impact_core::analyzer::import_resolver;
use impact_core::model::{
    ExecutableIr, FrameworkAnalysisResult, SourceFileIr, Target,
};
use impact_framework::FrameworkAdapter;

use crate::sfc_parser::SfcParser;
use crate::script_analyzer::ScriptAnalyzer;
use crate::template_analyzer::TemplateAnalyzer;

pub struct VueAdapter {
    aliases: HashMap<String, String>,
}

impl VueAdapter {
    pub fn new() -> Self {
        Self {
            aliases: HashMap::new(),
        }
    }

    pub fn with_aliases(aliases: HashMap<String, String>) -> Self {
        Self { aliases }
    }

    fn parse_recursive(
        &self,
        file: &Path,
        content: &str,
        depth: usize,
        visited: &mut std::collections::HashSet<PathBuf>,
        all_irs: &mut Vec<SourceFileIr>,
    ) -> anyhow::Result<()> {
        if depth == 0 || !visited.insert(file.to_path_buf()) {
            return Ok(());
        }

        let ir = self.parse_file(file, content)?;

        // 收集 import 源，解析依赖文件
        let import_sources: Vec<String> = ir.imports.iter()
            .map(|i| i.source.clone())
            .collect();

        all_irs.push(ir);

        // 递归解析依赖
        for source in import_sources {
            if let Some(resolved) = import_resolver::resolve_import_with_aliases(file, &source, &self.aliases) {
                if let Ok(dep_content) = std::fs::read_to_string(&resolved) {
                    self.parse_recursive(&resolved, &dep_content, depth - 1, visited, all_irs)?;
                }
            }
        }

        Ok(())
    }
}

impl FrameworkAdapter for VueAdapter {
    fn name(&self) -> &'static str {
        "vue"
    }

    fn detect(&self, file: &Path, _content: &str) -> bool {
        file.extension()
            .map(|ext| ext == "vue" || ext == "js" || ext == "ts")
            .unwrap_or(false)
    }

    fn parse_file(
        &self,
        file: &Path,
        content: &str,
    ) -> anyhow::Result<SourceFileIr> {
        let file_path = file.to_string_lossy().to_string();
        let component_name = file
            .file_stem()
            .map(|s| s.to_string_lossy().to_string());

        let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");

        if ext == "vue" {
            let sfc = SfcParser::parse(content)?;

            // 合并 script 和 script_setup 内容
            let mut script_content = sfc.script.content.clone();
            if let Some(setup) = &sfc.script_setup {
                if !setup.content.is_empty() {
                    script_content = format!("{}\n{}", script_content, setup.content);
                }
            }

            let data_fields = ScriptAnalyzer::extract_data_fields(&script_content);
            let computed_fields = ScriptAnalyzer::extract_computed(&script_content);
            let props = ScriptAnalyzer::extract_props(&script_content);
            let imports = ScriptAnalyzer::extract_imports(&script_content);
            let events = ScriptAnalyzer::extract_events(&script_content);

            let mut executables: Vec<ExecutableIr> = Vec::new();
            executables.extend(ScriptAnalyzer::extract_methods(&script_content));
            executables.extend(ScriptAnalyzer::extract_lifecycle_hooks(&script_content));
            executables.extend(ScriptAnalyzer::extract_computed_executables(&script_content));

            let template_bindings = TemplateAnalyzer::extract_bindings(&sfc.template.content);

            Ok(SourceFileIr {
                file_path,
                framework: "vue".to_string(),
                component_name,
                executables,
                data_fields,
                computed_fields,
                props,
                template_bindings,
                imports,
                events,
            })
        } else {
            // JS/TS file
            let data_fields = ScriptAnalyzer::extract_data_fields(content);
            let computed_fields = ScriptAnalyzer::extract_computed(content);
            let imports = ScriptAnalyzer::extract_imports(content);
            let events = ScriptAnalyzer::extract_events(content);

            let mut executables: Vec<ExecutableIr> = Vec::new();
            executables.extend(ScriptAnalyzer::extract_methods(content));
            executables.extend(ScriptAnalyzer::extract_lifecycle_hooks(content));

            Ok(SourceFileIr {
                file_path,
                framework: "js-ts".to_string(),
                component_name,
                executables,
                data_fields,
                computed_fields,
                props: Vec::new(),
                template_bindings: Vec::new(),
                imports,
                events,
            })
        }
    }

    fn analyze(
        &self,
        _files: Vec<SourceFileIr>,
        _target: &Target,
    ) -> anyhow::Result<FrameworkAnalysisResult> {
        Ok(FrameworkAnalysisResult {
            files: _files,
            errors: Vec::new(),
        })
    }

    fn parse_file_with_deps(
        &self,
        file: &Path,
        content: &str,
        max_depth: usize,
    ) -> anyhow::Result<Vec<SourceFileIr>> {
        let mut all_irs = Vec::new();
        let mut visited = std::collections::HashSet::new();

        self.parse_recursive(file, content, max_depth, &mut visited, &mut all_irs)?;

        Ok(all_irs)
    }
}
