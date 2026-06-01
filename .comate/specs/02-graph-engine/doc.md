# 第二阶段：图构建引擎与 Fixpoint 调度

## 背景

第一阶段完成了项目骨架搭建，实现了 SFC 解析和基本数据模型。第二阶段需要实现核心的图构建引擎和 Fixpoint 调度算法，将静态 IR 转换为 may-affect graph。

## 目标

1. 实现图构建引擎，将 SourceFileIr 转换为 ImpactGraph（节点和边）
2. 实现 Fixpoint 调度引擎（AnalysisStep、FactSet、run_fixpoint）
3. 实现 Upstream/Downstream 分析方向控制
4. 实现基于目标（target）的主链路裁剪
5. 实现 Vue 特定分析：data 读写、方法调用、计算属性依赖、模板绑定

## 核心数据流

```text
SourceFileIr ──► GraphBuilder ──► AnalysisStep ──► FactSet ──► ImpactGraph
                     │               │               │
                     └── Vue 分析器   └── 调度循环     └── 事实合并
```

## 技术方案

### 图构建器 (GraphBuilder)

将 Vue IR 转换为图结构：
- 数据字段 → NodeType::DataField 节点
- 可执行文件 → NodeType::Method/Lifecycle/Computed 节点
- 模板绑定 → NodeType::TemplateNode 节点
- 边关系基于数据流：
  - data.reads → EdgeType::Reads
  - data.writes → EdgeType::Writes
  - method.calls → EdgeType::Calls
  - template.binds → EdgeType::Renders

### 分析步骤 (AnalysisStep)

每个可分析单元抽象为 Step：
- MethodStep：分析方法体内的数据读写和调用
- ComputedStep：分析计算属性的数据依赖
- TemplateStep：分析模板绑定的数据引用
- LifecycleStep：分析生命周期钩子的副作用
- InitPhaseStep：分析初始化阶段的影响

### 调度循环 (run_fixpoint)

1. 维护全局 FactSet（已知事实集合）
2. 遍历所有 Step，检查前置事实是否满足
3. 执行满足条件的 Step，产出新事实
4. 合并新事实，重新检查所有 Step
5. 直到没有新事实产生，达到 fixpoint

### 目标解析 (TargetResolver)

- `data:count` → 找所有读写 count 的位置
- `method:increment` → 找所有调用 increment 的位置
- `computed:double` → 找所有依赖 double 的位置
- `init` → 找初始化阶段的影响

## 验证场景

### 场景 1：数据流分析

输入：Counter.vue 中 `data:count`
期望：找到所有读写 count 的位置，构建影响图

### 场景 2：方法调用分析

输入：Counter.vue 中 `method:increment`
期望：找到 increment 的调用链，包括数据读写和模板影响

### 场景 3：计算属性分析

输入：Counter.vue 中 `computed:double`
期望：找到 double 的数据依赖和影响链路

## 验收标准

1. `cargo test` 通过
2. 能够分析 Counter.vue 并生成正确的节点和边
3. 支持 `data:`、`method:`、`computed:` 三种目标类型
4. 支持 up/down/both 方向控制
5. 生成正确的 Markdown 报告