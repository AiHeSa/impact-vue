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

    #[arg(long, help = "Path aliases, e.g. @/=src/,~=/src/ (format: alias=path)")]
    pub alias: Vec<String>,

    #[arg(long, help = "Path query: start node, e.g. method:handleClick")]
    pub from: Option<String>,

    #[arg(long, help = "Path query: end node, e.g. data:count")]
    pub to: Option<String>,
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

    // 路径查询模式
    if let (Some(from_str), Some(to_str)) = (&args.from, &args.to) {
        let from_target = analyzer::resolve_watch(from_str)
            .ok_or_else(|| anyhow::anyhow!("Invalid --from expression: {}", from_str))?;
        let to_target = analyzer::resolve_watch(to_str)
            .ok_or_else(|| anyhow::anyhow!("Invalid --to expression: {}", to_str))?;
        
        // 找到 from 和 to 的节点 ID
        let from_id = find_node_id(&graph, &from_target);
        let to_id = find_node_id(&graph, &to_target);
        
        match (from_id, to_id) {
            (Some(from), Some(to)) => {
                let path_result = impact_core::analyzer::path_finder::find_paths(&graph, &from, &to);
                let report = impact_core::analyzer::path_finder::path_report(&path_result, from_str, to_str);
                
                let output_mode = args.output_mode.as_str();
                match output_mode {
                    "cli" | "both" => {
                        println!("{}", report);
                    }
                    _ => {}
                }
                
                if output_mode == "report" || output_mode == "both" {
                    let output_path = PathBuf::from(&args.output);
                    let dir = auto_report_dir(&output_path, &target)?;
                    std::fs::create_dir_all(&dir)?;
                    std::fs::write(dir.join("path-report.md"), &report)?;
                    let path_graph = impact_core::analyzer::path_finder::paths_to_graph(&path_result, &target);
                    reporter::write_outputs(&path_graph, &dir)?;
                }
                
                return Ok(());
            }
            _ => {
                eprintln!("Warning: Could not find nodes for --from or --to, falling back to normal analysis");
            }
        }
    }

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

/// 根据 target 查找图中的节点 ID
fn find_node_id(graph: &impact_core::model::ImpactGraph, target: &Target) -> Option<String> {
    use impact_core::model::{NodeType, TargetKind};
    
    for node in &graph.nodes {
        let matches = match (&target.kind, &node.node_type) {
            (TargetKind::Data, NodeType::DataField) => true,
            (TargetKind::Method, NodeType::Method) => true,
            (TargetKind::Computed, NodeType::Computed) => true,
            (TargetKind::Prop, NodeType::Prop) => true,
            (TargetKind::Init, NodeType::InitPhase) => true,
            _ => false,
        };
        
        if matches {
            if let Some(name) = &target.name {
                if node.name == *name {
                    return Some(node.id.clone());
                }
            } else {
                return Some(node.id.clone());
            }
        }
    }
    
    None
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
