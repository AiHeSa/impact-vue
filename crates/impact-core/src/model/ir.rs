//! 中间表示 (IR) 模块
//! 
//! 定义了 Vue 组件的中间表示结构，用于在解析和图构建之间传递数据。

use serde::{Deserialize, Serialize};

/// 节点类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NodeType {
    /// 组件
    Component,
    /// 数据字段 (data)
    DataField,
    /// 属性 (props)
    Prop,
    /// 计算属性 (computed)
    Computed,
    /// 方法 (methods)
    Method,
    /// 生命周期钩子
    Lifecycle,
    /// 模板节点
    TemplateNode,
    /// 事件
    Event,
    /// 异步任务
    AsyncTask,
    /// API 调用
    ApiCall,
    /// 模块
    Module,
    /// 初始化阶段
    InitPhase,
}

/// 边类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EdgeType {
    /// 读取数据
    Reads,
    /// 写入数据
    Writes,
    /// 调用方法
    Calls,
    /// 可能调用
    MaybeCalls,
    /// 等待异步操作
    Awaits,
    /// Promise then 回调
    ThenCalls,
    /// 渲染模板
    Renders,
    /// 绑定属性
    BindsProp,
    /// 触发事件
    EmitsEvent,
    /// 处理事件
    HandlesEvent,
    /// 依赖于
    DependsOn,
    /// 影响计算属性
    AffectsComputed,
    /// 初始化
    Initializes,
    /// 可能影响
    MaybeAffects,
    /// 导入
    Imports,
}

/// 分析方向
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Direction {
    /// 上游：谁可能影响目标
    Upstream,
    /// 下游：目标可能影响谁
    Downstream,
    /// 双向
    Both,
}

/// 源文件中间表示
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFileIr {
    /// 文件路径
    pub file_path: String,
    /// 框架名称
    pub framework: String,
    /// 组件名称
    pub component_name: Option<String>,
    /// 可执行文件列表
    pub executables: Vec<ExecutableIr>,
    /// 数据字段列表
    pub data_fields: Vec<DataFieldIr>,
    /// 计算属性列表
    pub computed_fields: Vec<ComputedIr>,
    /// 属性列表
    pub props: Vec<PropIr>,
    /// 模板绑定列表
    pub template_bindings: Vec<TemplateBindingIr>,
    /// 导入列表
    pub imports: Vec<ImportIr>,
    /// 事件列表
    pub events: Vec<EventIr>,
}

/// 可执行文件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutableKind {
    /// 方法
    Method,
    /// 计算属性
    Computed,
    /// 生命周期钩子
    Lifecycle,
    /// 异步回调
    AsyncCallback,
    /// 初始化阶段
    InitPhase,
}

/// 可执行文件中间表示
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutableIr {
    /// 类型
    pub kind: ExecutableKind,
    /// 名称
    pub name: String,
    /// 函数体
    pub body: String,
    /// 行号
    pub line: usize,
}

/// 数据字段中间表示
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFieldIr {
    /// 字段名
    pub name: String,
    /// 默认值
    pub default_value: Option<String>,
    /// 行号
    pub line: usize,
}

/// 计算属性中间表示
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputedIr {
    /// 属性名
    pub name: String,
    /// 依赖列表
    pub deps: Vec<String>,
    /// 行号
    pub line: usize,
}

/// 属性中间表示
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropIr {
    /// 属性名
    pub name: String,
    /// 属性类型
    pub prop_type: Option<String>,
    /// 行号
    pub line: usize,
}

/// 模板绑定中间表示
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateBindingIr {
    /// 节点 ID
    pub node_id: String,
    /// 绑定类型
    pub kind: String,
    /// 表达式
    pub expression: String,
    /// 数据路径列表
    pub data_paths: Vec<String>,
    /// 行号
    pub line: usize,
}

/// 导入中间表示
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportIr {
    /// 导入源
    pub source: String,
    /// 导入名称
    pub imported_name: Option<String>,
    /// 是否默认导入
    pub is_default: bool,
    /// 行号
    pub line: usize,
}

/// 事件中间表示
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventIr {
    /// 事件名
    pub event_name: String,
    /// 处理函数名
    pub handler_name: String,
    /// 行号
    pub line: usize,
}

/// 框架分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkAnalysisResult {
    /// 文件列表
    pub files: Vec<SourceFileIr>,
    /// 错误列表
    pub errors: Vec<String>,
}
