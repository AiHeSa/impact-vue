//! 路径查询模块
//!
//! 从完整影响图中提取从起点到终点的所有路径。

use std::collections::{HashMap, HashSet, VecDeque};

use crate::model::{Edge, ImpactGraph, Node};

/// 路径查询结果
#[derive(Debug, Clone)]
pub struct PathQueryResult {
    /// 找到的路径
    pub paths: Vec<Vec<String>>,
    /// 路径上的节点
    pub nodes: Vec<Node>,
    /// 路径上的边
    pub edges: Vec<Edge>,
}

/// 在图中搜索从 start 到 end 的所有简单路径
pub fn find_paths(graph: &ImpactGraph, start: &str, end: &str) -> PathQueryResult {
    // 构建邻接表
    let adj = build_adjacency_list(&graph.edges);
    
    // BFS 找所有路径
    let paths = bfs_all_paths(&adj, start, end);
    
    // 收集路径上的节点和边
    let path_node_ids: HashSet<String> = paths.iter()
        .flat_map(|p| p.iter().cloned())
        .collect();
    
    let path_edge_keys: HashSet<String> = paths.iter()
        .flat_map(|p| {
            p.windows(2).map(|w| format!("{}->{}", w[0], w[1])).collect::<Vec<_>>()
        })
        .collect();
    
    let nodes: Vec<Node> = graph.nodes.iter()
        .filter(|n| path_node_ids.contains(&n.id))
        .cloned()
        .collect();
    
    let edges: Vec<Edge> = graph.edges.iter()
        .filter(|e| {
            let key = format!("{}->{}", e.source, e.target);
            path_edge_keys.contains(&key)
        })
        .cloned()
        .collect();
    
    PathQueryResult { paths, nodes, edges }
}

/// 构建邻接表
fn build_adjacency_list(edges: &[Edge]) -> HashMap<String, Vec<String>> {
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    
    for edge in edges {
        adj.entry(edge.source.clone())
            .or_default()
            .push(edge.target.clone());
    }
    
    adj
}

/// BFS 找从 start 到 end 的所有最短路径
fn bfs_all_paths(
    adj: &HashMap<String, Vec<String>>,
    start: &str,
    end: &str,
) -> Vec<Vec<String>> {
    let mut result = Vec::new();
    let mut queue: VecDeque<Vec<String>> = VecDeque::new();
    let mut min_len = usize::MAX;
    
    queue.push_back(vec![start.to_string()]);
    
    while let Some(path) = queue.pop_front() {
        let current = path.last().unwrap();
        
        // 超过最短路径长度，停止
        if path.len() > min_len {
            continue;
        }
        
        if current == end {
            if path.len() < min_len {
                min_len = path.len();
                result.clear();
            }
            result.push(path);
            continue;
        }
        
        if let Some(neighbors) = adj.get(current) {
            for neighbor in neighbors {
                // 避免环
                if !path.contains(neighbor) {
                    let mut new_path = path.clone();
                    new_path.push(neighbor.clone());
                    queue.push_back(new_path);
                }
            }
        }
    }
    
    result
}

/// 将路径查询结果转换为精简的 ImpactGraph
pub fn paths_to_graph(result: &PathQueryResult, original_target: &crate::model::Target) -> ImpactGraph {
    ImpactGraph {
        target: original_target.clone(),
        nodes: result.nodes.clone(),
        edges: result.edges.clone(),
        evidences: Vec::new(),
        unknowns: Vec::new(),
    }
}

/// 生成路径查询的 Markdown 报告
pub fn path_report(result: &PathQueryResult, from: &str, to: &str) -> String {
    let mut report = String::new();
    
    report.push_str(&format!("# 路径查询报告\n\n"));
    report.push_str(&format!("**起点**: `{}`\n", from));
    report.push_str(&format!("**终点**: `{}`\n", to));
    report.push_str(&format!("**找到路径数**: {}\n\n", result.paths.len()));
    
    if result.paths.is_empty() {
        report.push_str("未找到从起点到终点的路径。\n");
        return report;
    }
    
    report.push_str("## 路径\n\n");
    for (i, path) in result.paths.iter().enumerate() {
        report.push_str(&format!("### 路径 {}\n\n", i + 1));
        report.push_str("```");
        report.push('\n');
        for (j, node) in path.iter().enumerate() {
            if j > 0 {
                report.push_str("    ↓\n");
            }
            report.push_str(&format!("{}\n", node));
        }
        report.push_str("```\n\n");
    }
    
    report.push_str("## 节点详情\n\n");
    for node in &result.nodes {
        report.push_str(&format!("- `{}` [{:?}] {}\n", 
            node.id, 
            node.node_type,
            node.file.as_ref().map(|f| format!("({})", f)).unwrap_or_default()
        ));
    }
    
    report.push_str("\n## 边详情\n\n");
    for edge in &result.edges {
        report.push_str(&format!(
            "- `{}` --{:?}/{:?}--> `{}`\n",
            edge.source, edge.edge_type, edge.confidence, edge.target
        ));
    }
    
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    
    #[test]
    fn test_find_simple_path() {
        let graph = ImpactGraph {
            target: Target { kind: TargetKind::Data, name: None },
            nodes: vec![
                Node { id: "A".into(), component: "".into(), name: "A".into(), node_type: NodeType::Method, file: None, line: None },
                Node { id: "B".into(), component: "".into(), name: "B".into(), node_type: NodeType::DataField, file: None, line: None },
            ],
            edges: vec![
                Edge { source: "A".into(), target: "B".into(), edge_type: EdgeType::Writes, confidence: crate::model::confidence::Confidence::High, evidence_id: None },
            ],
            evidences: vec![],
            unknowns: vec![],
        };
        
        let result = find_paths(&graph, "A", "B");
        assert_eq!(result.paths.len(), 1);
        assert_eq!(result.paths[0], vec!["A", "B"]);
        assert_eq!(result.nodes.len(), 2);
        assert_eq!(result.edges.len(), 1);
    }
    
    #[test]
    fn test_find_path_with_intermediate() {
        let graph = ImpactGraph {
            target: Target { kind: TargetKind::Data, name: None },
            nodes: vec![
                Node { id: "A".into(), component: "".into(), name: "A".into(), node_type: NodeType::Method, file: None, line: None },
                Node { id: "B".into(), component: "".into(), name: "B".into(), node_type: NodeType::Method, file: None, line: None },
                Node { id: "C".into(), component: "".into(), name: "C".into(), node_type: NodeType::DataField, file: None, line: None },
            ],
            edges: vec![
                Edge { source: "A".into(), target: "B".into(), edge_type: EdgeType::Calls, confidence: crate::model::confidence::Confidence::High, evidence_id: None },
                Edge { source: "B".into(), target: "C".into(), edge_type: EdgeType::Writes, confidence: crate::model::confidence::Confidence::High, evidence_id: None },
            ],
            evidences: vec![],
            unknowns: vec![],
        };
        
        let result = find_paths(&graph, "A", "C");
        assert_eq!(result.paths.len(), 1);
        assert_eq!(result.paths[0], vec!["A", "B", "C"]);
    }
    
    #[test]
    fn test_no_path() {
        let graph = ImpactGraph {
            target: Target { kind: TargetKind::Data, name: None },
            nodes: vec![
                Node { id: "A".into(), component: "".into(), name: "A".into(), node_type: NodeType::Method, file: None, line: None },
                Node { id: "B".into(), component: "".into(), name: "B".into(), node_type: NodeType::DataField, file: None, line: None },
            ],
            edges: vec![],
            evidences: vec![],
            unknowns: vec![],
        };
        
        let result = find_paths(&graph, "A", "B");
        assert_eq!(result.paths.len(), 0);
    }
    
    #[test]
    fn test_multiple_paths() {
        let graph = ImpactGraph {
            target: Target { kind: TargetKind::Data, name: None },
            nodes: vec![
                Node { id: "A".into(), component: "".into(), name: "A".into(), node_type: NodeType::Method, file: None, line: None },
                Node { id: "B".into(), component: "".into(), name: "B".into(), node_type: NodeType::Method, file: None, line: None },
                Node { id: "C".into(), component: "".into(), name: "C".into(), node_type: NodeType::Method, file: None, line: None },
                Node { id: "D".into(), component: "".into(), name: "D".into(), node_type: NodeType::DataField, file: None, line: None },
            ],
            edges: vec![
                Edge { source: "A".into(), target: "B".into(), edge_type: EdgeType::Calls, confidence: crate::model::confidence::Confidence::High, evidence_id: None },
                Edge { source: "A".into(), target: "C".into(), edge_type: EdgeType::Calls, confidence: crate::model::confidence::Confidence::High, evidence_id: None },
                Edge { source: "B".into(), target: "D".into(), edge_type: EdgeType::Writes, confidence: crate::model::confidence::Confidence::High, evidence_id: None },
                Edge { source: "C".into(), target: "D".into(), edge_type: EdgeType::Writes, confidence: crate::model::confidence::Confidence::High, evidence_id: None },
            ],
            evidences: vec![],
            unknowns: vec![],
        };
        
        let result = find_paths(&graph, "A", "D");
        assert_eq!(result.paths.len(), 2);
    }
}
