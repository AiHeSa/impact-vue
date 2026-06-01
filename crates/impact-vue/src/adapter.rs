use std::path::Path;

use impact_core::model::{
    ExecutableIr, FrameworkAnalysisResult, SourceFileIr, Target,
};
use impact_framework::FrameworkAdapter;

use crate::sfc_parser::SfcParser;
use crate::script_analyzer::ScriptAnalyzer;
use crate::template_analyzer::TemplateAnalyzer;

pub struct VueAdapter;

impl VueAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl FrameworkAdapter for VueAdapter {
    fn name(&self) -> &'static str {
        "vue"
    }

    fn detect(&self, file: &Path, _content: &str) -> bool {
        file.extension()
            .map(|ext| ext == "vue")
            .unwrap_or(false)
    }

    fn parse_file(
        &self,
        file: &Path,
        content: &str,
    ) -> anyhow::Result<SourceFileIr> {
        let file_path = file.to_string_lossy().to_string();
        let sfc = SfcParser::parse(content)?;

        let component_name = file
            .file_stem()
            .map(|s| s.to_string_lossy().to_string());

        let script_content = &sfc.script.content;
        let data_fields = ScriptAnalyzer::extract_data_fields(script_content);
        let computed_fields = ScriptAnalyzer::extract_computed(script_content);
        let props = ScriptAnalyzer::extract_props(script_content);
        let imports = ScriptAnalyzer::extract_imports(script_content);
        let events = ScriptAnalyzer::extract_events(script_content);

        let mut executables: Vec<ExecutableIr> = Vec::new();
        executables.extend(ScriptAnalyzer::extract_methods(script_content));
        executables.extend(ScriptAnalyzer::extract_lifecycle_hooks(script_content));
        executables.extend(ScriptAnalyzer::extract_computed_executables(script_content));

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
}
