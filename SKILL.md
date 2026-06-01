---
name: impact-vue
description: 使用 impact CLI 做 Vue 项目静态 may-affect 影响面图分析。排查 debug、定位某个组件/函数改动会影响哪里、分析调用链/依赖链、生成影响面报告、评估改动风险时使用。支持 .vue 入口，分析 Options API 和 Composition API。用户提到"impact 一下""影响面""依赖链""调用链""改这个会影响哪里""may-affect graph""排查 debug 影响范围"等场景时触发。
---
# Impact Vue Skill

## 目标

使用 `impact` CLI 辅助 Vue 项目排查 debug 和改动影响面。`impact` 是静态 may-affect graph analyzer，支持 `.vue` 文件分析，适合在修改或排查某个组件、函数前后，快速确认它可能影响哪些代码路径、依赖链路和风险区域。

## 优先使用场景

- 排查 debug：用户想知道某个问题可能从哪里传导、某个组件异常可能影响哪些模块。
- 改动影响面：用户问"改这个文件/函数/组件会影响哪里""有没有下游影响"。
- 依赖/调用链：用户想看 entry 到 target 的链路，或反向查谁会影响某个目标。
- 跨组件分析：用户想看父子组件之间的 props 下发和 emit 事件链路。
- 代码评审前：用户需要补充影响面说明、风险点、验证建议。
- 重构/迁移前：用户需要确认入口文件与目标模块之间的关系。
- 报告产出：用户需要 `md`、`json`、`mermaid` 等影响面报告。

## 基本命令

```bash
impact analyze --framework vue --entry <path> --watch <target> [options]
```

## 参数说明

### --entry（必填）

分析入口文件，支持 `.vue` 单文件组件。

### --watch（推荐）

格式：

| 表达式 | 说明 |
|---|---|
| `init` | 分析组件初始化影响面 |
| `method:handleClick` | 以 method 为目标，分析调用链路 |
| `data:count` | 以 data 字段为目标 |
| `computed:double` | 以 computed 属性为目标 |
| `prop:title` | 以 props 属性为目标 |

### --direction

| 值 | 说明 |
|---|---|
| `both` | 双向分析（默认） |
| `up` / `upstream` | 只看谁影响目标 |
| `down` / `downstream` | 只看目标影响谁 |

### --output-mode

| 值 | 说明 |
|---|---|
| `cli` | 仅终端输出（默认） |
| `report` | 仅生成本地报告 |
| `both` | 终端 + 本地报告 |

### --cross-module

启用跨模块分析，需配合 `--project-root` 使用。

```bash
impact analyze --framework vue --entry src/App.vue --watch data:count \
  --cross-module --project-root src
```

## 支持的分析能力

- **数据流分析**：`this.xxx` 读写、`ref()` / `reactive()` 响应式数据
- **方法调用链**：`this.method()` 调用关系
- **计算属性依赖**：computed 对 data/prop 的依赖
- **模板绑定**：`{{ }}`、`v-if`、`v-for`、`v-model`、`:prop`、`@event`
- **跨组件链路**：props 下发 (BindsProp)、emit 事件 (EmitsEvent)
- **异步链路**：`await`、`.then()`、`.catch()` 数据写入
- **动态 data path**：`` `param[${index}]` `` → `param[*]`
- **生命周期**：created、mounted 等钩子中的数据读写

## 工作流

1. 确认分析目标：从用户选中文件、当前焦点文件、报错栈中选择 `--entry`。
2. 默认使用 `--output-mode cli`，不写本地文件。
3. 如果用户关心某个 method/data/computed 的链路，用 `--watch method:xxx` 格式。
4. 如果分析跨组件影响，加 `--cross-module --project-root <dir>`。
5. 结论要服务于 debug：说明可能链路、关键节点、风险点。

## 推荐命令模板

### 分析 data 字段影响

```bash
impact analyze --framework vue --entry src/Component.vue --watch data:count --output-mode cli
```

### 分析方法调用链

```bash
impact analyze --framework vue --entry src/Component.vue --watch method:handleClick --output-mode cli
```

### 跨组件分析

```bash
impact analyze --framework vue --entry src/App.vue --watch data:user \
  --cross-module --project-root src --output-mode cli
```

### 生成报告文件

```bash
impact analyze --framework vue --entry src/Component.vue --watch data:count \
  --output impact-output/my-analysis --output-mode both
```

### 上游分析（谁影响目标）

```bash
impact analyze --framework vue --entry src/Component.vue --watch data:count \
  --direction up --output-mode cli
```

## 输出总结格式

默认用中文简洁总结：

```
**Impact 结果**
- 入口：`<entry>`
- 目标：`<target>`
- 关键链路：列出 2-5 条最值得关注的链路
- Debug 关注点：列出最可能相关的节点、状态流、事件流
- 建议验证：列出需要手动验证或跑测试的位置
```

## 注意事项

- `impact` 结果仅作为辅助排查使用，不对最终结果负责。
- 不要臆造 `entry` 或 `target`。缺少信息时先推断，不明确再问用户。
- 它是静态 may-affect 分析，需要结合代码阅读和测试验证。
- `<script setup>` 的 `defineProps` 目前支持有限，ref/reactive 代理访问需要通过 `this.xxx` 形式识别。
- 跨组件分析需要 `--cross-module --project-root` 才能识别父子组件链路。
- 如果 CLI 报错，先运行 `impact analyze --help` 检查参数。
