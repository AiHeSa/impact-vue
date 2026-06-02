//! 图构建器模块
//! 
//! 负责将 Vue 组件的中间表示 (IR) 转换为影响图 (ImpactGraph)。
//! 
//! # 功能
//! 
//! - 创建节点：数据字段、计算属性、方法、生命周期、模板、事件、异步任务
//! - 创建边：读写、调用、渲染、属性绑定、事件触发、依赖关系
//! - 跨组件分析：props 绑定、emit 事件
//! - 异步链路：await、Promise then/catch
//! - 动态数据路径：模板字符串、字符串拼接
//! - 图裁剪：基于目标的多跳传递闭包

use crate::model::{
    Direction, Edge, EdgeType, ExecutableKind, ImpactGraph, Node, NodeType,
    SourceFileIr, Target, TargetKind,
};
use indexmap::IndexSet;
use std::collections::HashMap;

/// 构建完整的 ImpactGraph
/// 
/// # 参数
/// 
/// - `all_irs`: 所有源文件的中间表示
/// - `target`: 分析目标
/// - `direction`: 分析方向（上游/下游/双向）
/// 
/// # 返回
/// 
/// 返回裁剪后的影响图，只包含与目标相关的节点和边。
pub fn build_full_graph(all_irs: &[SourceFileIr], target: &Target, direction: &Direction) -> ImpactGraph {
    let mut graph = ImpactGraph {
        target: target.clone(),
        nodes: Vec::new(),
        edges: Vec::new(),
        evidences: Vec::new(),
        unknowns: Vec::new(),
    };
    
    // 1. 创建所有节点
    let mut node_map: HashMap<String, Node> = HashMap::new();
    
    for ir in all_irs {
        let component = ir.component_name.clone().unwrap_or_default();
        
        // 数据字段节点
        for field in &ir.data_fields {
            let node_id = format!("{}:data:{}", component, field.name);
            let node = Node {
                id: node_id.clone(),
                component: component.clone(),
                name: field.name.clone(),
                node_type: NodeType::DataField,
                file: Some(ir.file_path.clone()),
                line: Some(field.line),
            };
            node_map.insert(node_id, node);
        }
        
        // 可执行文件节点
        for exec in &ir.executables {
            let node_type = match exec.kind {
                ExecutableKind::Method => NodeType::Method,
                ExecutableKind::Computed => NodeType::Computed,
                ExecutableKind::Lifecycle => NodeType::Lifecycle,
                ExecutableKind::AsyncCallback => NodeType::AsyncTask,
                ExecutableKind::InitPhase => NodeType::InitPhase,
            };
            
            let node_id = format!("{}:{}:{}", component, node_type_prefix(&node_type), exec.name);
            let node = Node {
                id: node_id.clone(),
                component: component.clone(),
                name: exec.name.clone(),
                node_type,
                file: Some(ir.file_path.clone()),
                line: Some(exec.line),
            };
            node_map.insert(node_id, node);
        }
        
        // 计算属性节点
        for computed in &ir.computed_fields {
            let node_id = format!("{}:computed:{}", component, computed.name);
            let node = Node {
                id: node_id.clone(),
                component: component.clone(),
                name: computed.name.clone(),
                node_type: NodeType::Computed,
                file: Some(ir.file_path.clone()),
                line: Some(computed.line),
            };
            node_map.insert(node_id, node);
        }
        
        // Props 节点
        for prop in &ir.props {
            let node_id = format!("{}:prop:{}", component, prop.name);
            let node = Node {
                id: node_id.clone(),
                component: component.clone(),
                name: prop.name.clone(),
                node_type: NodeType::Prop,
                file: Some(ir.file_path.clone()),
                line: Some(prop.line),
            };
            node_map.insert(node_id, node);
        }
        
        // 事件节点
        for event in &ir.events {
            let node_id = format!("{}:event:{}", component, event.event_name);
            let node = Node {
                id: node_id.clone(),
                component: component.clone(),
                name: event.event_name.clone(),
                node_type: NodeType::Event,
                file: Some(ir.file_path.clone()),
                line: Some(event.line),
            };
            node_map.insert(node_id, node);
        }
        
        // 模板绑定节点
        for binding in &ir.template_bindings {
            let node_id = format!("{}:template:{}", component, binding.node_id);
            let node = Node {
                id: node_id.clone(),
                component: component.clone(),
                name: binding.node_id.clone(),
                node_type: NodeType::TemplateNode,
                file: Some(ir.file_path.clone()),
                line: Some(binding.line),
            };
            node_map.insert(node_id, node);
        }
    }
    
    // 2. 创建边
    for ir in all_irs {
        let component = ir.component_name.clone().unwrap_or_default();
        
        // 分析可执行文件中的数据读写
        for exec in &ir.executables {
            let exec_id = format!("{}:{}:{}", component, node_type_prefix(&node_type_from_kind(&exec.kind)), exec.name);
            
            // 从执行体中提取数据读写
            let (reads, writes) = extract_data_access(&exec.body);
            
            for read in reads {
                if let Some(tid) = find_data_or_prop_node(&node_map, &component, &read) {
                    graph.edges.push(Edge {
                        source: exec_id.clone(),
                        target: tid,
                        edge_type: EdgeType::Reads,
                        confidence: crate::model::confidence::Confidence::High,
                        evidence_id: None,
                    });
                }
            }
            
            for write in writes {
                if let Some(tid) = find_data_or_prop_node(&node_map, &component, &write) {
                    graph.edges.push(Edge {
                        source: exec_id.clone(),
                        target: tid,
                        edge_type: EdgeType::Writes,
                        confidence: crate::model::confidence::Confidence::High,
                        evidence_id: None,
                    });
                }
            }
            
            // 分析方法调用
            let calls = extract_method_calls(&exec.body);
            for call in calls {
                // 先在当前组件中查找
                let call_id = format!("{}:method:{}", component, call);
                if node_map.contains_key(&call_id) {
                    graph.edges.push(Edge {
                        source: exec_id.clone(),
                        target: call_id,
                        edge_type: EdgeType::Calls,
                        confidence: crate::model::confidence::Confidence::High,
                        evidence_id: None,
                    });
                } else {
                    // 在所有组件中查找（跨文件调用）
                    for (node_id, node) in &node_map {
                        if node.node_type == NodeType::Method && node.name == call {
                            graph.edges.push(Edge {
                                source: exec_id.clone(),
                                target: node_id.clone(),
                                edge_type: EdgeType::Calls,
                                confidence: crate::model::confidence::Confidence::Medium,
                                evidence_id: None,
                            });
                            break;
                        }
                    }
                }
            }
            
            // 分析 async/await 链路
            if is_async_method(&exec.body) {
                let api_calls = extract_await_calls(&exec.body);
                for api in api_calls {
                    let api_id = format!("{}:async:{}", component, api);
                    // 创建 AsyncTask 节点
                    node_map.entry(api_id.clone()).or_insert_with(|| Node {
                        id: api_id.clone(),
                        component: component.clone(),
                        name: api.clone(),
                        node_type: NodeType::AsyncTask,
                        file: Some(ir.file_path.clone()),
                        line: Some(exec.line),
                    });
                    graph.edges.push(Edge {
                        source: exec_id.clone(),
                        target: api_id,
                        edge_type: EdgeType::Awaits,
                        confidence: crate::model::confidence::Confidence::High,
                        evidence_id: None,
                    });
                }
            }
            
            // 分析 .then() 链中的写入
            let then_writes = extract_then_writes(&exec.body);
            for write in then_writes {
                let data_id = format!("{}:data:{}", component, write);
                if node_map.contains_key(&data_id) {
                    graph.edges.push(Edge {
                        source: exec_id.clone(),
                        target: data_id,
                        edge_type: EdgeType::ThenCalls,
                        confidence: crate::model::confidence::Confidence::Medium,
                        evidence_id: None,
                    });
                }
            }
            
            // 分析 .catch() 链中的写入
            let catch_writes = extract_catch_writes(&exec.body);
            for write in catch_writes {
                let data_id = format!("{}:data:{}", component, write);
                if node_map.contains_key(&data_id) {
                    graph.edges.push(Edge {
                        source: exec_id.clone(),
                        target: data_id,
                        edge_type: EdgeType::MaybeCalls,
                        confidence: crate::model::confidence::Confidence::Low,
                        evidence_id: None,
                    });
                }
            }
        }
        
        // 计算属性依赖
        for computed in &ir.computed_fields {
            let computed_id = format!("{}:computed:{}", component, computed.name);
            for dep in &computed.deps {
                if let Some(tid) = find_data_or_prop_node(&node_map, &component, dep) {
                    graph.edges.push(Edge {
                        source: computed_id.clone(),
                        target: tid,
                        edge_type: EdgeType::DependsOn,
                        confidence: crate::model::confidence::Confidence::High,
                        evidence_id: None,
                    });
                }
            }
        }
        
        // 模板绑定
        for binding in &ir.template_bindings {
            let template_id = format!("{}:template:{}", component, binding.node_id);
            for data_path in &binding.data_paths {
                if let Some(tid) = find_data_or_prop_node(&node_map, &component, data_path) {
                    graph.edges.push(Edge {
                        source: template_id.clone(),
                        target: tid,
                        edge_type: EdgeType::Renders,
                        confidence: crate::model::confidence::Confidence::High,
                        evidence_id: None,
                    });
                }
            }
        }
        
        // Props 依赖：子组件 props 读取父组件数据
        for prop in &ir.props {
            let prop_id = format!("{}:prop:{}", component, prop.name);
            // 查找父组件中绑定到该 prop 的数据
            for parent_ir in all_irs {
                let parent_component = parent_ir.component_name.clone().unwrap_or_default();
                if parent_component != component {
                    for binding in &parent_ir.template_bindings {
                        if binding.kind == "bind" && binding.data_paths.contains(&prop.name) {
                            // 父组件数据绑定到子组件 prop
                            let parent_data_id = format!("{}:data:{}", parent_component, binding.data_paths[0]);
                            if node_map.contains_key(&parent_data_id) {
                                graph.edges.push(Edge {
                                    source: parent_data_id.clone(),
                                    target: prop_id.clone(),
                                    edge_type: EdgeType::BindsProp,
                                    confidence: crate::model::confidence::Confidence::High,
                                    evidence_id: None,
                                });
                            }
                        }
                    }
                }
            }
        }
        
        // Emit 事件：子组件 emit 事件绑定到父组件 handler
        for event in &ir.events {
            let event_id = format!("{}:event:{}", component, event.event_name);
            // 查找父组件中处理该事件的 handler
            for parent_ir in all_irs {
                let parent_component = parent_ir.component_name.clone().unwrap_or_default();
                if parent_component != component {
                    for binding in &parent_ir.template_bindings {
                        // binding.kind 是 "@update" 格式，需要提取事件名
                        let binding_event = binding.kind.trim_start_matches('@');
                        if binding.kind.starts_with('@') && binding_event == event.event_name {
                            // 父组件监听子组件事件
                            let handler_id = format!("{}:method:{}", parent_component, binding.data_paths[0]);
                            if node_map.contains_key(&handler_id) {
                                graph.edges.push(Edge {
                                    source: event_id.clone(),
                                    target: handler_id.clone(),
                                    edge_type: EdgeType::EmitsEvent,
                                    confidence: crate::model::confidence::Confidence::High,
                                    evidence_id: None,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    
    // 3. 根据目标裁剪图（多跳传递闭包）
    let target_nodes = find_target_nodes(&node_map, target);
    
    // BFS: 从目标节点出发，沿边双向遍历，收集所有相关节点和边
    let mut visited_nodes: IndexSet<String> = IndexSet::new();
    let mut visited_edges: IndexSet<String> = IndexSet::new();
    let mut queue: Vec<String> = target_nodes.iter().cloned().collect();
    let mut related_edges = Vec::new();
    let mut related_node_ids: IndexSet<String> = IndexSet::new();
    
    // 目标节点本身必须包含在结果中
    for id in &target_nodes {
        related_node_ids.insert(id.clone());
    }
    
    while let Some(node_id) = queue.pop() {
        if visited_nodes.contains(&node_id) {
            continue;
        }
        visited_nodes.insert(node_id.clone());
        
        for edge in &graph.edges {
            let edge_key = format!("{}->{}:{:?}", edge.source, edge.target, edge.edge_type);
            if (edge.source == node_id || edge.target == node_id) && !visited_edges.contains(&edge_key) {
                visited_edges.insert(edge_key);
                related_edges.push(edge.clone());
                related_node_ids.insert(edge.source.clone());
                related_node_ids.insert(edge.target.clone());
                if !visited_nodes.contains(&edge.source) {
                    queue.push(edge.source.clone());
                }
                if !visited_nodes.contains(&edge.target) {
                    queue.push(edge.target.clone());
                }
            }
        }
    }
    
    // 收集相关的节点
    let mut filtered_nodes = Vec::new();
    for node_id in &related_node_ids {
        if let Some(node) = node_map.get(node_id) {
            filtered_nodes.push(node.clone());
        }
    }
    
    graph.nodes = filtered_nodes;
    graph.edges = related_edges;
    
    // 4. 根据方向过滤边
    if *direction != Direction::Both {
        let mut direction_edges = Vec::new();
        for edge in &graph.edges {
            let is_upstream = matches!(
                edge.edge_type,
                EdgeType::Reads | EdgeType::DependsOn | EdgeType::BindsProp | EdgeType::HandlesEvent
            );
            let is_downstream = matches!(
                edge.edge_type,
                EdgeType::Writes | EdgeType::Calls | EdgeType::Renders | EdgeType::EmitsEvent | EdgeType::MaybeCalls
            );
            
            match direction {
                Direction::Upstream => {
                    if is_upstream {
                        direction_edges.push(edge.clone());
                    }
                }
                Direction::Downstream => {
                    if is_downstream {
                        direction_edges.push(edge.clone());
                    }
                }
                Direction::Both => {
                    direction_edges.push(edge.clone());
                }
            }
        }
        graph.edges = direction_edges;
    }
    
    graph
}

/// 查找 data 或 prop 节点 ID
fn find_data_or_prop_node(node_map: &HashMap<String, Node>, component: &str, name: &str) -> Option<String> {
    let data_id = format!("{}:data:{}", component, name);
    let prop_id = format!("{}:prop:{}", component, name);
    if node_map.contains_key(&data_id) {
        Some(data_id)
    } else if node_map.contains_key(&prop_id) {
        Some(prop_id)
    } else {
        None
    }
}

/// 从执行体中提取数据读写
fn extract_data_access(body: &str) -> (Vec<String>, Vec<String>) {
    let mut reads = Vec::new();
    let mut writes = Vec::new();
    
    // 匹配 this.data.get('field') 和 this.data.set('field', value)
    let re_read = regex::Regex::new(r#"this\.data\.get\(\s*['"]([^'"]+)['"]\s*\)"#).unwrap();
    let re_write = regex::Regex::new(r#"this\.data\.set\(\s*['"]([^'"]+)['"]\s*,"#).unwrap();
    
    for cap in re_read.captures_iter(body) {
        reads.push(cap[1].to_string());
    }
    
    for cap in re_write.captures_iter(body) {
        writes.push(cap[1].to_string());
    }
    
    // 匹配动态 data path: this.data.set(`param[${xxx}]`, value) → param[*]
    let re_dynamic = regex::Regex::new(r#"this\.data\.set\(\s*`([^`]+)`\s*,"#).unwrap();
    for cap in re_dynamic.captures_iter(body) {
        let path = cap[1].to_string();
        // 将 ${xxx} 替换为 *
        let normalized = regex::Regex::new(r#"\$\{[^}]+\}"#)
            .unwrap()
            .replace_all(&path, "*")
            .to_string();
        writes.push(normalized);
    }
    
    // 匹配动态 data path: this.data.get(`param[${xxx}]`)
    let re_dynamic_get = regex::Regex::new(r#"this\.data\.get\(\s*`([^`]+)`\s*\)"#).unwrap();
    for cap in re_dynamic_get.captures_iter(body) {
        let path = cap[1].to_string();
        let normalized = regex::Regex::new(r#"\$\{[^}]+\}"#)
            .unwrap()
            .replace_all(&path, "*")
            .to_string();
        reads.push(normalized);
    }
    
    // 匹配动态 data path: this.data.set('param.' + xxx, value) → param.*
    let re_concat = regex::Regex::new(r#"this\.data\.set\(\s*['"]([^'"]+)['"]\s*\+\s*"#).unwrap();
    for cap in re_concat.captures_iter(body) {
        let prefix = cap[1].to_string();
        writes.push(format!("{}.*", prefix));
    }
    
    // 匹配直接访问 this.xxx 的模式（Vue 2 代理）
    // 读取：this.xxx（不在赋值左侧）
    let re_read_direct = regex::Regex::new(r#"this\.(\w+)"#).unwrap();
    for cap in re_read_direct.captures_iter(body) {
        let field = cap[1].to_string();
        // 排除常见非数据字段
        if !["data", "methods", "computed", "props", "emit", "$emit", "$refs", "$el", "$options", "$parent", "$root", "$children", "$slots", "$scopedSlots", "$attrs", "$listeners", "$watch", "$set", "$delete", "$nextTick", "$on", "$once", "$off", "$mount", "$forceUpdate", "$destroy"].contains(&field.as_str()) {
            reads.push(field);
        }
    }
    
    // 写入：this.xxx = value, this.xxx++, this.xxx--
    let re_write_direct = regex::Regex::new(r#"this\.(\w+)\s*(?:=|\+\+|--)"#).unwrap();
    for cap in re_write_direct.captures_iter(body) {
        let field = cap[1].to_string();
        if !["data", "methods", "computed", "props", "emit", "$emit", "$refs", "$el", "$options", "$parent", "$root", "$children", "$slots", "$scopedSlots", "$attrs", "$listeners", "$watch", "$set", "$delete", "$nextTick", "$on", "$once", "$off", "$mount", "$forceUpdate", "$destroy"].contains(&field.as_str()) {
            writes.push(field);
        }
    }
    
    // Composition API: xxx.value++ / xxx.value-- / xxx.value = ...
    let re_ref_write = regex::Regex::new(r#"(\w+)\.value\s*(?:=|\+\+|--)"#).unwrap();
    for cap in re_ref_write.captures_iter(body) {
        let field = cap[1].to_string();
        writes.push(field);
    }
    
    // Composition API: xxx.value（读取）
    let re_ref_read = regex::Regex::new(r#"(\w+)\.value\b"#).unwrap();
    for cap in re_ref_read.captures_iter(body) {
        let field = cap[1].to_string();
        reads.push(field);
    }
    
    (reads, writes)
}

/// 从执行体中提取方法调用
fn extract_method_calls(body: &str) -> Vec<String> {
    let mut calls = Vec::new();
    let mut seen = std::collections::HashSet::new();
    
    // 匹配 this.method() 调用（Options API）
    let re_this = regex::Regex::new(r#"this\.(\w+)\(\s*\)"#).unwrap();
    for cap in re_this.captures_iter(body) {
        let name = cap[1].to_string();
        if seen.insert(name.clone()) {
            calls.push(name);
        }
    }
    
    // 匹配直接函数调用 method()（Composition API）
    // 排除常见全局函数
    let excluded = [
        "console", "log", "error", "warn", "info", "debug",
        "setTimeout", "setInterval", "clearTimeout", "clearInterval",
        "parseInt", "parseFloat", "isNaN", "isFinite",
        "require", "import", "export",
        "if", "else", "for", "while", "switch", "case", "return",
        "new", "typeof", "instanceof", "void", "delete",
        "ref", "reactive", "computed", "watch", "watchEffect",
        "onMounted", "onUnmounted", "onUpdated", "onBeforeMount", "onBeforeUnmount",
        "defineProps", "defineEmits", "defineExpose",
        "nextTick", "toRef", "toRefs", "unref", "isRef",
        "push", "pop", "shift", "unshift", "splice", "slice",
        "map", "filter", "reduce", "forEach", "find", "findIndex",
        "then", "catch", "finally", "resolve", "reject",
        "addEventListener", "removeEventListener",
        "querySelector", "querySelectorAll", "getElementById",
        "toString", "valueOf", "hasOwnProperty",
    ];
    
    let re_direct = regex::Regex::new(r#"\b(\w+)\(\s*\)"#).unwrap();
    for cap in re_direct.captures_iter(body) {
        let name = cap[1].to_string();
        if !excluded.contains(&name.as_str()) && !name.starts_with('$') && seen.insert(name.clone()) {
            calls.push(name);
        }
    }
    
    calls
}

/// 从执行体中提取 await API 调用
fn extract_await_calls(body: &str) -> Vec<String> {
    let mut api_calls = Vec::new();
    
    // 匹配 await xxx.xxx() 或 await xxx()
    let re = regex::Regex::new(r#"await\s+(?:[\w.]+\.)?(\w+)\s*\("#).unwrap();
    
    for cap in re.captures_iter(body) {
        api_calls.push(cap[1].to_string());
    }
    
    api_calls
}

/// 从执行体中提取 .then() 链中的数据写入
fn extract_then_writes(body: &str) -> Vec<String> {
    let mut writes = Vec::new();
    
    // 匹配 .then(res => { this.xxx = ... }) 或 .then(res => { this.xxx = res.xxx })
    let re = regex::Regex::new(r#"\.then\s*\([^)]*\)\s*\{[^}]*this\.(\w+)\s*="#).unwrap();
    
    for cap in re.captures_iter(body) {
        writes.push(cap[1].to_string());
    }
    
    // 匹配 .then(res => this.xxx = ...)
    let re2 = regex::Regex::new(r#"\.then\s*\([^)]*\)\s*=>\s*this\.(\w+)\s*="#).unwrap();
    
    for cap in re2.captures_iter(body) {
        writes.push(cap[1].to_string());
    }
    
    writes
}

/// 从执行体中提取 .catch() 链中的数据写入
fn extract_catch_writes(body: &str) -> Vec<String> {
    let mut writes = Vec::new();
    
    // 匹配 .catch(e => { this.xxx = ... })
    let re = regex::Regex::new(r#"\.catch\s*\([^)]*\)\s*\{[^}]*this\.(\w+)\s*="#).unwrap();
    
    for cap in re.captures_iter(body) {
        writes.push(cap[1].to_string());
    }
    
    writes
}

/// 检测方法是否是 async
fn is_async_method(body: &str) -> bool {
    body.contains("await ")
}

/// 查找目标相关的节点
fn find_target_nodes(node_map: &HashMap<String, Node>, target: &Target) -> IndexSet<String> {
    let mut result = IndexSet::new();
    
    for (id, node) in node_map {
        match &target.kind {
            TargetKind::Data => {
                if node.node_type == NodeType::DataField {
                    if let Some(name) = &target.name {
                        if node.name == *name {
                            result.insert(id.clone());
                        }
                    }
                }
            }
            TargetKind::Method => {
                if node.node_type == NodeType::Method {
                    if let Some(name) = &target.name {
                        if node.name == *name {
                            result.insert(id.clone());
                        }
                    }
                }
            }
            TargetKind::Computed => {
                if node.node_type == NodeType::Computed {
                    if let Some(name) = &target.name {
                        if node.name == *name {
                            result.insert(id.clone());
                        }
                    }
                }
            }
            TargetKind::Init => {
                if node.node_type == NodeType::InitPhase {
                    result.insert(id.clone());
                }
            }
            _ => {}
        }
    }
    
    result
}

/// 节点类型前缀
fn node_type_prefix(node_type: &NodeType) -> &'static str {
    match node_type {
        NodeType::DataField => "data",
        NodeType::Method => "method",
        NodeType::Computed => "computed",
        NodeType::Lifecycle => "lifecycle",
        NodeType::AsyncTask => "async",
        NodeType::InitPhase => "init",
        _ => "other",
    }
}

/// 从可执行文件类型获取节点类型
fn node_type_from_kind(kind: &ExecutableKind) -> NodeType {
    match kind {
        ExecutableKind::Method => NodeType::Method,
        ExecutableKind::Computed => NodeType::Computed,
        ExecutableKind::Lifecycle => NodeType::Lifecycle,
        ExecutableKind::AsyncCallback => NodeType::AsyncTask,
        ExecutableKind::InitPhase => NodeType::InitPhase,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        DataFieldIr, ExecutableIr, ExecutableKind, SourceFileIr, TemplateBindingIr,
    };
    
    fn create_test_ir() -> SourceFileIr {
        SourceFileIr {
            file_path: "test.vue".to_string(),
            framework: "vue".to_string(),
            component_name: Some("Counter".to_string()),
            executables: vec![
                ExecutableIr {
                    kind: ExecutableKind::Method,
                    name: "increment".to_string(),
                    body: "this.data.set('count', this.data.get('count') + 1)".to_string(),
                    line: 10,
                },
            ],
            data_fields: vec![
                DataFieldIr {
                    name: "count".to_string(),
                    default_value: Some("0".to_string()),
                    line: 5,
                },
            ],
            computed_fields: vec![],
            props: vec![],
            template_bindings: vec![
                TemplateBindingIr {
                    node_id: "1".to_string(),
                    kind: "text".to_string(),
                    expression: "count".to_string(),
                    data_paths: vec!["count".to_string()],
                    line: 1,
                },
            ],
            imports: vec![],
            events: vec![],
        }
    }
    
    #[test]
    fn test_build_graph_for_data_target() {
        let ir = create_test_ir();
        let target = Target {
            kind: TargetKind::Data,
            name: Some("count".to_string()),
        };
        
        let graph = build_full_graph(&[ir], &target, &Direction::Both);
        
        // 应该有节点：data:count, method:increment, template:1
        assert!(!graph.nodes.is_empty());
        
        // 应该有边：increment 读写 count，template 渲染 count
        let reads_count = graph.edges.iter().any(|e| 
            e.edge_type == EdgeType::Reads && e.target.ends_with(":data:count")
        );
        let writes_count = graph.edges.iter().any(|e| 
            e.edge_type == EdgeType::Writes && e.target.ends_with(":data:count")
        );
        let renders_count = graph.edges.iter().any(|e| 
            e.edge_type == EdgeType::Renders && e.target.ends_with(":data:count")
        );
        
        assert!(reads_count, "应该有读取 count 的边");
        assert!(writes_count, "应该有写入 count 的边");
        assert!(renders_count, "应该有渲染 count 的边");
    }
    
    #[test]
    fn test_extract_data_access() {
        let body = "this.data.set('count', this.data.get('count') + 1)";
        let (reads, writes) = extract_data_access(body);
        
        assert_eq!(reads, vec!["count"]);
        assert_eq!(writes, vec!["count"]);
    }
    
    #[test]
    fn test_extract_method_calls() {
        let body = "this.increment()";
        let calls = extract_method_calls(body);
        
        assert_eq!(calls, vec!["increment"]);
    }
}