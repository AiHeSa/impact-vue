use std::path::{Path, PathBuf};

use clap::Args;
use impact_core::analyzer;
use impact_core::model::Target;
use impact_core::reporter;
use impact_framework::AdapterRegistry;

#[derive(Args, Clone)]
pub struct AnalyzeArgs {
    #[arg(long, help = "Framework adapter (vue)")]
    pub framework: Option<String>,

    #[arg(long, help = "Entry file path")]
    pub entry: String,

    #[arg(
        long,
        default_value = "init",
        help = "Watch target, e.g. method:handleClick, data:count, computed:list, init"
    )]
    pub watch: String,

    #[arg(long, default_value = "both", help = "Output direction: upstream, downstream, both")]
    pub direction: String,

    #[arg(long, default_value = "impact-output", help = "Output directory")]
    pub output: String,

    #[arg(
        long,
        default_value = "both",
        help = "Output mode: cli, report, both"
    )]
    pub output_mode: String,

    #[arg(long, help = "Project root for cross-module analysis")]
    pub project_root: Option<String>,

    #[arg(long, default_value = "false", help = "Enable cross-module analysis")]
    pub cross_module: bool,
}

use impact_core::model::Direction;

pub fn run(args: &AnalyzeArgs, registry: &AdapterRegistry) -> anyhow::Result<()> {
    let entry_path = Path::new(&args.entry);
    if !entry_path.exists() {
        anyhow::bail!("Entry file not found: {}", args.entry);
    }

    let content = std::fs::read_to_string(entry_path)?;

    let adapter = registry
        .select(args.framework.as_deref(), entry_path, &content)
        .ok_or_else(|| anyhow::anyhow!("No suitable framework adapter found"))?;

    let target = analyzer::resolve_watch(&args.watch)
        .ok_or_else(|| anyhow::anyhow!("Invalid watch expression: {}", args.watch))?;

    let direction = match args.direction.as_str() {
        "up" | "upstream" => Direction::Upstream,
        "down" | "downstream" => Direction::Downstream,
        _ => Direction::Both,
    };

    // 使用 parse_file_with_deps 解析文件及其依赖
    let all_irs = adapter.parse_file_with_deps(entry_path, &content, 10)?;
    
    let mut result = adapter.analyze(all_irs, &target)?;

    if args.cross_module {
        if let Some(root) = &args.project_root {
            let cross = impact_core::analyzer::cross_module::CrossModuleAnalysis::new(Path::new(root));
            let files = cross.collect_files()?;
            let import_graph = cross.build_import_graph(&files)?;
            let orphans = cross.detect_orphans(&files, &import_graph);

            let mut all_irs = Vec::new();
            for file in &files {
                if let Ok(c) = std::fs::read_to_string(file) {
                    if let Ok(ir) = adapter.parse_file(file, &c) {
                        all_irs.push(ir);
                    }
                }
            }
            result = adapter.analyze(all_irs, &target)?;

            let nodes = impact_core::analyzer::cross_module::build_report_import_graph(
                &files, &import_graph, &orphans,
            );
            let md_section = impact_core::analyzer::cross_module::cross_module_markdown_section(&nodes);
            let mmd_section = impact_core::analyzer::cross_module::cross_module_mermaid(&nodes);

            let output_path = PathBuf::from(&args.output);
            std::fs::create_dir_all(&output_path)?;
            std::fs::write(output_path.join("cross-module-imports.md"), &md_section)?;
            std::fs::write(output_path.join("cross-module-imports.mmd"), &mmd_section)?;
        }
    }

    let graph = analyzer::build_graph(&result.files, &target, &direction);

    let output_mode = args.output_mode.as_str();
    match output_mode {
        "cli" | "both" => {
            let report = reporter::markdown_report(&graph);
            println!("{}", report);
        }
        _ => {}
    }

    if output_mode == "report" || output_mode == "both" {
        let output_path = PathBuf::from(&args.output);
        let dir = auto_report_dir(&output_path, &target)?;
        reporter::write_outputs(&graph, &dir)?;
        prune_report_cache(&output_path, 10)?;
    }

    Ok(())
}

fn auto_report_dir(base: &Path, target: &Target) -> anyhow::Result<PathBuf> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let target_part = match &target.name {
        Some(name) => sanitize_path_part(name),
        None => "init".to_string(),
    };

    let dir_name = format!("impact-run-{}-{}", timestamp, target_part);
    let dir = base.join(&dir_name);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn prune_report_cache(base: &Path, keep: usize) -> anyhow::Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(base)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("impact-run-"))
        .collect();

    entries.sort_by_key(|e| e.path());

    if entries.len() > keep {
        for entry in entries.iter().take(entries.len() - keep) {
            std::fs::remove_dir_all(entry.path())?;
        }
    }

    Ok(())
}

fn sanitize_path_part(value: &str) -> String {
    value.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect()
}
