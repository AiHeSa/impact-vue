# impact

Vue 静态影响链路分析 CLI。基于源码静态扫描构建 may-affect graph，分析某个 data、method、computed 或 prop 变化可能影响到哪些方法、模板、异步调用和跨组件链路。

## 安装

```bash
cargo install --path crates/impact-cli --force
```

## 基本用法

```bash
# 分析 data 字段影响
impact analyze --framework vue --entry src/Counter.vue --watch data:count

# 分析方法调用链
impact analyze --framework vue --entry src/Counter.vue --watch method:increment

# 跨组件分析
impact analyze --framework vue --entry src/App.vue --watch data:user \
  --cross-module --project-root src
```

## CLI 参数

| 参数 | 说明 | 默认值 |
|------|------|--------|
| `--framework` | 框架适配器（vue） | 自动识别 |
| `--entry` | 入口文件路径 | 必填 |
| `--watch` | 分析目标：`data:field`、`method:name`、`computed:name`、`prop:name`、`init` | `init` |
| `--direction` | 分析方向：`up`、`down`、`both` | `both` |
| `--output` | 报告输出目录 | `impact-output` |
| `--output-mode` | 输出模式：`cli`、`report`、`both` | `both` |
| `--cross-module` | 启用跨模块分析 | 关闭 |
| `--project-root` | 项目根目录 | entry 所在目录 |

## 支持的分析能力

- **数据流**：`this.xxx` 读写、`ref()` / `reactive()` 响应式数据
- **方法调用链**：`this.method()` 调用关系
- **计算属性依赖**：computed → data/prop 依赖
- **模板绑定**：`{{ }}`、`v-if`、`v-for`、`v-model`、`:prop`、`@event`
- **跨组件**：props 下发、emit 事件
- **异步链路**：`await`、`.then()`、`.catch()`
- **动态 data path**：`` `param[${index}]` `` → `param[*]`
- **生命周期**：created、mounted 等钩子中的数据读写
- **方向控制**：上游（谁影响目标）、下游（目标影响谁）

## 输出

默认生成：

| 文件 | 说明 |
|------|------|
| `report.md` | Markdown 报告 |
| `graph.json` | 完整图结构 |
| `graph.mmd` | Mermaid 图（可粘贴到 mermaid.live 预览） |
| `evidence.json` | 边证据 |
| `summary.json` | 统计摘要 |

## 项目结构

```
crates/
  impact-cli/          CLI 入口 (clap)
  impact-core/         IR、Graph、Analyzer、Reporter
  impact-framework/    FrameworkAdapter 特征
  impact-vue/          Vue SFC 解析、Script/Template 分析
fixtures/              测试用例
```

## 开发

```bash
cargo build
cargo test
cargo run -- analyze --framework vue --entry fixtures/vue/basic/Counter.vue --watch data:count --output-mode cli
```

## 节点类型

| 类型 | 颜色（Mermaid） | 说明 |
|------|----------------|------|
| DataField | 绿色 | data 字段 |
| Prop | 浅绿 | props 属性 |
| Method | 蓝色 | methods 方法 |
| Computed | 紫色 | computed 计算属性 |
| Lifecycle | 橙色 | 生命周期钩子 |
| TemplateNode | 灰色 | 模板节点 |
| Event | 红色 | 事件 |
| AsyncTask | 青色 | 异步任务 |

## 边类型

| 类型 | 说明 |
|------|------|
| Reads | 读取数据 |
| Writes | 写入数据 |
| Calls | 调用方法 |
| DependsOn | 依赖（computed → data） |
| Renders | 模板渲染 |
| BindsProp | 跨组件 props 绑定 |
| EmitsEvent | 跨组件事件触发 |
| Awaits | 等待异步操作 |
| ThenCalls | Promise then 回调 |

## License

MIT
