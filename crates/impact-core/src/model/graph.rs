use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

use super::confidence::Confidence;
use super::evidence::Evidence;
use super::ir::{EdgeType, NodeType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Target {
    pub kind: TargetKind,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TargetKind {
    Data,
    Method,
    Computed,
    Prop,
    Init,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub component: String,
    pub name: String,
    pub node_type: NodeType,
    pub file: Option<String>,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub source: String,
    pub target: String,
    pub edge_type: EdgeType,
    pub confidence: Confidence,
    pub evidence_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unknown {
    pub id: String,
    pub description: String,
    pub reason: String,
    pub file: Option<String>,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactGraph {
    pub target: Target,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub evidences: Vec<Evidence>,
    pub unknowns: Vec<Unknown>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Fact {
    pub kind: FactKind,
    pub component: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FactKind {
    DataKnown,
    DataRead,
    DataWritten,
    MethodCallable,
    MethodCalled,
    ComputedDependsOn,
    TemplateReads,
    ExternalValue,
    UnknownValue,
}

#[derive(Debug, Clone)]
pub struct FactSet {
    facts: IndexSet<Fact>,
}

impl FactSet {
    pub fn new() -> Self {
        Self {
            facts: IndexSet::new(),
        }
    }
    
    pub fn insert(&mut self, fact: Fact) -> bool {
        self.facts.insert(fact)
    }
    
    pub fn contains(&self, fact: &Fact) -> bool {
        self.facts.contains(fact)
    }
    
    pub fn satisfy_all(&self, patterns: &[crate::analyzer::step::FactPattern]) -> bool {
        for pattern in patterns {
            match pattern {
                crate::analyzer::step::FactPattern::Exact(fact) => {
                    if !self.contains(fact) {
                        return false;
                    }
                }
                crate::analyzer::step::FactPattern::ExternalAllowed { source } => {
                    let external = Fact {
                        kind: FactKind::ExternalValue,
                        component: String::new(),
                        detail: source.clone(),
                    };
                    if !self.contains(&external) {
                        return false;
                    }
                }
                crate::analyzer::step::FactPattern::UnknownAllowed { reason } => {
                    let unknown = Fact {
                        kind: FactKind::UnknownValue,
                        component: String::new(),
                        detail: reason.clone(),
                    };
                    if !self.contains(&unknown) {
                        return false;
                    }
                }
            }
        }
        true
    }
    
    pub fn can_downgrade_missing_to_unknown(&self, patterns: &[crate::analyzer::step::FactPattern]) -> bool {
        for pattern in patterns {
            match pattern {
                crate::analyzer::step::FactPattern::Exact(_) => {
                    // 精确匹配不能降级
                    return false;
                }
                crate::analyzer::step::FactPattern::ExternalAllowed { .. } => {
                    // 外部值可以降级为未知
                    continue;
                }
                crate::analyzer::step::FactPattern::UnknownAllowed { .. } => {
                    // 未知值可以降级
                    continue;
                }
            }
        }
        true
    }
}
