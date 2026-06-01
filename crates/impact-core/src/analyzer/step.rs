use crate::model::graph::{Fact, FactSet};
use crate::model::{Edge, Unknown};
use crate::model::{Target, TargetKind};

/// 分析步骤类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepKind {
    Method,
    Computed,
    Lifecycle,
    TemplateBinding,
    AsyncCallback,
    InitPhase,
}

/// 分析步骤
#[derive(Debug, Clone)]
pub struct AnalysisStep {
    pub key: String,
    pub kind: StepKind,
    pub required_facts: Vec<FactPattern>,
    pub evidence_id: Option<String>,
}

/// 事实模式
#[derive(Debug, Clone)]
pub enum FactPattern {
    Exact(Fact),
    ExternalAllowed { source: String },
    UnknownAllowed { reason: String },
}

/// 步骤执行结果
#[derive(Debug, Clone)]
pub struct StepResult {
    pub produced_facts: Vec<Fact>,
    pub produced_edges: Vec<Edge>,
    pub unknowns: Vec<Unknown>,
}

/// 准备状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadyState {
    Ready,
    ReadyWithUnknowns,
    Blocked,
}

/// 检查步骤是否准备就绪
pub fn is_step_ready(step: &AnalysisStep, facts: &FactSet) -> ReadyState {
    if facts.satisfy_all(&step.required_facts) {
        return ReadyState::Ready;
    }
    
    if facts.can_downgrade_missing_to_unknown(&step.required_facts) {
        return ReadyState::ReadyWithUnknowns;
    }
    
    ReadyState::Blocked
}

/// 运行 fixpoint 调度
pub fn run_fixpoint(
    steps: &[AnalysisStep],
    facts: &mut FactSet,
    execute: impl Fn(&AnalysisStep, &FactSet) -> StepResult,
) -> Vec<StepResult> {
    let mut results = Vec::new();
    let mut changed = true;
    
    while changed {
        changed = false;
        
        for step in steps {
            let ready = is_step_ready(step, facts);
            if ready == ReadyState::Ready || ready == ReadyState::ReadyWithUnknowns {
                let result = execute(step, facts);
                
                // 合并新事实
                for fact in &result.produced_facts {
                    if facts.insert(fact.clone()) {
                        changed = true;
                    }
                }
                
                results.push(result);
            }
        }
    }
    
    results
}

/// 创建目标到节点的映射
pub fn create_target_steps(target: &Target) -> Vec<AnalysisStep> {
    let mut steps = Vec::new();
    
    match &target.kind {
        TargetKind::Data => {
            if let Some(name) = &target.name {
                steps.push(AnalysisStep {
                    key: format!("data:{}", name),
                    kind: StepKind::Method,
                    required_facts: vec![],
                    evidence_id: None,
                });
            }
        }
        TargetKind::Method => {
            if let Some(name) = &target.name {
                steps.push(AnalysisStep {
                    key: format!("method:{}", name),
                    kind: StepKind::Method,
                    required_facts: vec![],
                    evidence_id: None,
                });
            }
        }
        TargetKind::Computed => {
            if let Some(name) = &target.name {
                steps.push(AnalysisStep {
                    key: format!("computed:{}", name),
                    kind: StepKind::Computed,
                    required_facts: vec![],
                    evidence_id: None,
                });
            }
        }
        TargetKind::Init => {
            steps.push(AnalysisStep {
                key: "init".to_string(),
                kind: StepKind::InitPhase,
                required_facts: vec![],
                evidence_id: None,
            });
        }
        _ => {}
    }
    
    steps
}