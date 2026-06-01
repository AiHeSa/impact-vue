use std::collections::HashMap;
use std::path::{Path, PathBuf};

use regex::Regex;
use walkdir::WalkDir;

use crate::analyzer::path_utils::resolve_relative_file;

pub struct CrossModuleAnalysis {
    pub project_root: PathBuf,
    pub max_depth: Option<usize>,
}

impl CrossModuleAnalysis {
    pub fn new(project_root: &Path) -> Self {
        Self {
            project_root: project_root.to_path_buf(),
            max_depth: None,
        }
    }

    pub fn collect_files(&self) -> anyhow::Result<Vec<PathBuf>> {
        let extensions = [
            "vue", "ts", "js", "tsx", "jsx",
            "html", "htm", "rs",
        ];

        let mut files = Vec::new();

        for entry in WalkDir::new(&self.project_root)
            .max_depth(self.max_depth.unwrap_or(usize::MAX))
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.')
                    && name != "node_modules"
                    && name != "target"
                    && name != "dist"
                    && name != "build"
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

        files.sort();
        Ok(files)
    }

    pub fn build_import_graph(
        &self,
        files: &[PathBuf],
    ) -> anyhow::Result<HashMap<String, Vec<String>>> {
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();

        let re_import = Regex::new(r#"(?:import|require)\s*\(?['"]([^'"]+)['"]"#)?;

        for file in files {
            let content = std::fs::read_to_string(file)?;
            let relative = file
                .strip_prefix(&self.project_root)
                .unwrap_or(file)
                .to_string_lossy()
                .to_string();

            let mut imports = Vec::new();

            for cap in re_import.captures_iter(&content) {
                let raw = cap[1].to_string();
                if raw.starts_with('.') {
                    if let Some(resolved) =
                        resolve_relative_file(file, &raw, &["vue", "ts", "js", "tsx", "jsx"])
                    {
                        let resolved_rel = resolved
                            .strip_prefix(&self.project_root)
                            .unwrap_or(&resolved)
                            .to_string_lossy()
                            .to_string();
                        imports.push(resolved_rel);
                    }
                } else {
                    imports.push(raw);
                }
            }

            graph.insert(relative, imports);
        }

        Ok(graph)
    }

    pub fn detect_orphans(
        &self,
        files: &[PathBuf],
        import_graph: &HashMap<String, Vec<String>>,
    ) -> Vec<String> {
        let mut imported_by: HashMap<String, Vec<String>> = HashMap::new();

        for (source, targets) in import_graph {
            for target in targets {
                imported_by
                    .entry(target.clone())
                    .or_default()
                    .push(source.clone());
            }
        }

        let entry_stems = ["main", "app", "index", "lib", "mod"];

        let mut orphans = Vec::new();
        for file in files {
            let relative = file
                .strip_prefix(&self.project_root)
                .unwrap_or(file)
                .to_string_lossy()
                .to_string();

            if !imported_by.contains_key(&relative) {
                let stem = file
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();

                if !entry_stems.contains(&stem.as_str()) {
                    orphans.push(relative);
                }
            }
        }

        orphans.sort();
        orphans
    }
}

pub fn build_report_import_graph(
    files: &[PathBuf],
    import_graph: &HashMap<String, Vec<String>>,
    orphans: &[String],
) -> Vec<ImportGraphNode> {
    let mut imported_by: HashMap<String, Vec<String>> = HashMap::new();

    for (source, targets) in import_graph {
        for target in targets {
            imported_by
                .entry(target.clone())
                .or_default()
                .push(source.clone());
        }
    }

    let mut nodes = Vec::new();
    for file in files {
        let relative = file
            .strip_prefix("/")
                .unwrap_or(file)
                .to_string_lossy()
                .to_string();

        let file_imports = import_graph.get(&relative).cloned().unwrap_or_default();
        let file_imported_by = imported_by.get(&relative).cloned().unwrap_or_default();
        let is_orphan = orphans.contains(&relative);

        nodes.push(ImportGraphNode {
            file: relative,
            imports: file_imports,
            imported_by: file_imported_by,
            is_orphan,
        });
    }

    nodes
}

pub struct ImportGraphNode {
    pub file: String,
    pub imports: Vec<String>,
    pub imported_by: Vec<String>,
    pub is_orphan: bool,
}

pub fn cross_module_markdown_section(nodes: &[ImportGraphNode]) -> String {
    let mut md = String::from("## Cross-Module Import Graph\n\n");
    md.push_str("| File | Imports | Imported By | Orphan |\n");
    md.push_str("|------|---------|-------------|--------|\n");

    for node in nodes {
        let imports = node.imports.join(", ");
        let imported_by = node.imported_by.join(", ");
        let orphan = if node.is_orphan { "Yes" } else { "No" };
        md.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            node.file, imports, imported_by, orphan
        ));
    }

    md
}

pub fn cross_module_mermaid(nodes: &[ImportGraphNode]) -> String {
    let mut mmd = String::from("graph LR\n");

    for node in nodes {
        let src = node
            .file
            .replace(['/', '.', '-', '\\'], "_")
            .replace("__", "_");

        for imp in &node.imports {
            let tgt = imp
                .replace(['/', '.', '-', '\\'], "_")
                .replace("__", "_");

            if imp.starts_with('.') {
                mmd.push_str(&format!("    {} --> {}\n", src, tgt));
            } else {
                mmd.push_str(&format!("    {} -.-> {}\n", src, tgt));
            }
        }
    }

    mmd
}
