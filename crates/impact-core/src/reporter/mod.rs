//! 报告生成器模块
//! 
//! 负责将影响图转换为各种格式的报告。
//! 
//! # 支持的格式
//! 
//! - Markdown 报告 (`report.md`)
//! - JSON 图结构 (`graph.json`)
//! - Mermaid 图 (`graph.mmd`)
//! - 证据 JSON (`evidence.json`)
//! - 摘要 JSON (`summary.json`)

use crate::model::ImpactGraph;

/// 生成 Markdown 格式的影响分析报告
/// 
/// # 参数
/// 
/// - `graph`: 影响图
/// 
/// # 返回
/// 
/// 返回 Markdown 格式的报告字符串。
pub fn markdown_report(graph: &ImpactGraph) -> String {
    let mut report = String::new();
    report.push_str("# 静态影响分析报告\n\n");
    report.push_str("## 目标\n\n");
    report.push_str(&format!("- 类型: {:?}\n", graph.target.kind));
    if let Some(name) = &graph.target.name {
        report.push_str(&format!("- 名称: {}\n", name));
    }
    report.push_str("\n");
    report.push_str(&format!("## 节点 ({})\n\n", graph.nodes.len()));
    for node in &graph.nodes {
        report.push_str(&format!("- `{}` [{:?}]\n", node.id, node.node_type));
    }
    report.push_str("\n");
    report.push_str(&format!("## 边 ({})\n\n", graph.edges.len()));
    for edge in &graph.edges {
        report.push_str(&format!(
            "- `{}` --{:?}/{:?}--> `{}`\n",
            edge.source, edge.edge_type, edge.confidence, edge.target
        ));
    }
    report
}

pub fn write_outputs(
    graph: &ImpactGraph,
    output_dir: &std::path::Path,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(output_dir)?;

    let json = serde_json::to_string_pretty(graph)?;
    std::fs::write(output_dir.join("graph.json"), &json)?;

    let md = markdown_report(graph);
    std::fs::write(output_dir.join("report.md"), &md)?;

    let mmd = mermaid_graph(graph);
    std::fs::write(output_dir.join("graph.mmd"), &mmd)?;

    let evidence = evidence_json(graph);
    std::fs::write(output_dir.join("evidence.json"), &evidence)?;

    let summary = serde_json::json!({
        "nodes": graph.nodes.len(),
        "edges": graph.edges.len(),
        "evidences": graph.evidences.len(),
        "unknowns": graph.unknowns.len(),
        "target": graph.target,
    });
    std::fs::write(
        output_dir.join("summary.json"),
        serde_json::to_string_pretty(&summary)?,
    )?;

    Ok(())
}

pub fn mermaid_graph(graph: &ImpactGraph) -> String {
    let mut mmd = String::from("graph TD\n");
    
    // 添加节点定义
    for node in &graph.nodes {
        let id = sanitize_mermaid_id(&node.id);
        mmd.push_str(&format!("    {}[\"{}\"]:::node{}\n", id, node.id, id));
    }
    
    // 添加边
    for edge in &graph.edges {
        let src = sanitize_mermaid_id(&edge.source);
        let tgt = sanitize_mermaid_id(&edge.target);
        let label = format!("{:?}", edge.edge_type);
        mmd.push_str(&format!("    {} -->|{}| {}\n", src, label, tgt));
    }
    
    // 添加样式类
    mmd.push_str("\n");
    for node in &graph.nodes {
        let id = sanitize_mermaid_id(&node.id);
        let style = match node.node_type {
            crate::model::NodeType::DataField => "fill:#4CAF50,color:white",
            crate::model::NodeType::Prop => "fill:#8BC34A,color:white",
            crate::model::NodeType::Method => "fill:#2196F3,color:white",
            crate::model::NodeType::Computed => "fill:#9C27B0,color:white",
            crate::model::NodeType::Lifecycle => "fill:#FF9800,color:white",
            crate::model::NodeType::TemplateNode => "fill:#607D8B,color:white",
            crate::model::NodeType::Event => "fill:#F44336,color:white",
            crate::model::NodeType::AsyncTask => "fill:#00BCD4,color:white",
            _ => "fill:#9E9E9E,color:white",
        };
        mmd.push_str(&format!("    classDef node{} {}\n", id, style));
    }
    
    mmd
}

pub fn evidence_json(graph: &ImpactGraph) -> String {
    let evidence = serde_json::json!({
        "edges": graph.edges.iter().map(|e| {
            serde_json::json!({
                "source": e.source,
                "target": e.target,
                "edge_type": format!("{:?}", e.edge_type),
                "confidence": format!("{:?}", e.confidence),
                "evidence_id": e.evidence_id,
            })
        }).collect::<Vec<_>>(),
        "evidences": graph.evidences,
        "unknowns": graph.unknowns,
    });
    serde_json::to_string_pretty(&evidence).unwrap_or_default()
}

fn sanitize_mermaid_id(id: &str) -> String {
    id.replace(['(', ')', '[', ']', '{', '}', ',', '"', ':', '.'], "_")
        .replace(['-', ' '], "_")
}
