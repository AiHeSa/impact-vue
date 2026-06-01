# 项目初始化 Spec

## 背景

本项目是对 `san-impack-analyzer` 的复刻，从多框架（San/Vue/React/HTML/Rust）通用影响链路分析工具，转型为**专注 Vue 生态**的静态 may-affect graph 构建 CLI。

## 目标

1. 搭建 Rust workspace 项目骨架
2. 定义核心数据模型（IR、Graph、Confidence、Evidence）
3. 实现 FrameworkAdapter 特征与注册机制
4. 实现 Vue 适配器骨架（SFC 解析、Options API、Composition API、Template 分析）
5. CLI 入口：`impact analyze --framework vue --entry App.vue --watch data:xxx`

## 架构

```text
impact-vue/
├── Cargo.toml                    # workspace 根
├── crates/
│   ├── impact-cli/               # CLI 入口，clap 参数解析
│   ├── impact-core/              # 核心引擎：IR、Graph、Analyzer、Reporter
│   ├── impact-framework/         # FrameworkAdapter 特征
│   └── impact-vue/               # Vue 适配器
├── fixtures/                     # 测试用例
├── tests/                        # 集成测试
└── .comate/specs/                # 设计文档
```

## 核心数据模型

- **NodeType**: Component、DataField、Prop、Computed、Method、Lifecycle、TemplateNode、Event、AsyncTask
- **EdgeType**: Reads、Writes、Calls、MaybeCalls、Renders、DependsOn、Initializes、EmitsEvent、HandlesEvent
- **Confidence**: High / Medium / Low
- **Target**: Data / Method / Computed / Prop / Init
- **Fact / FactSet / AnalysisStep**: Fixpoint 调度核心

## Vue 支持范围

### Options API
- `data()` 字段提取
- `methods: {}` 函数提取
- `computed: {}` 依赖识别
- Lifecycle hooks（created、mounted 等）
- `props: []` / `props: {}` 声明提取

### Composition API (`<script setup>`)
- `ref()` / `reactive()` 响应式数据
- `computed()` 计算属性
- `watch()` / `watchEffect()`
- `onMounted` / `onUnmounted` 等生命周期
- `defineProps()` / `defineEmits()`

### Template
- `{{ expr }}` mustache 插值
- `v-if` / `v-else-if` / `v-show`
- `v-for`
- `:prop` / `v-bind:prop`
- `@event` / `v-on:event`
- `v-model`

## 技术选型

- Rust Edition 2021
- `clap` CLI 参数
- `serde` / `serde_json` 序列化
- `regex` 轻量扫描
- `walkdir` 文件遍历

## 不做范围

- 不实现完整 JS AST 解析（V1 为 regex-based）
- 不实现 TypeScript 类型推断
- 不处理运行时动态组件
- 不展开外部库函数实现
