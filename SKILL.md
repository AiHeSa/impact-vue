---
name: impact-vue
description: 使用 impact CLI 做 Vue 项目静态 may-affect 影响面图分析。排查 debug、定位某个组件/函数改动会影响哪里、分析调用链/依赖链、生成影响面报告、评估改动风险时使用。支持 Options API、Composition API、Pinia store。用户提到"impact 一下""影响面""依赖链""调用链""改这个会影响哪里""may-affect graph""排查 debug 影响范围"等场景时触发。
---
# Impact Vue Skill

## 目标

使用 `impact` CLI 辅助 Vue 项目排查 debug 和改动影响面。`impact` 是静态 may-affect graph analyzer，支持 `.vue`、`.ts`、`.js` 文件分析，适合在修改或排查某个组件、函数前后，快速确认它可能影响哪些代码路径、依赖链路和风险区域。

## 优先使用场景

- 排查 debug：用户想知道某个问题可能从哪里传导、某个组件异常可能影响哪些模块。
- 改动影响面：用户问"改这个文件/函数/组件会影响哪里""有没有下游影响"。
- 依赖/调用链：用户想看 entry 到 target 的链路，或反向查谁会影响某个目标。
- 跨组件分析：用户想看父子组件之间的 props 下发和 emit 事件链路。
- 跨文件分析：用户想追踪 import 依赖链路、store 调用链。
- Pinia store 分析：用户想看 store 的 state/getters/actions 被哪些组件使用。
- 路径查询：用户想知道从 A 到 B 有哪些路径。
- 代码评审前：用户需要补充影响面说明、风险点、验证建议。
- 重构/迁移前：用户需要确认入口文件与目标模块之间的关系。
- 报告产出：用户需要 `md`、`json`、`mermaid` 等影响面报告。

## 基本命令

```bash
impact analyze --framework vue --entry <path> --watch <target> [options]
```

## 参数说明

### --entry（必填）

分析入口文件，支持 `.vue`、`.ts`、`.js` 文件。

### --watch（推荐）

格式：

| 表达式 | 说明 |
|---|---|
| `init` | 分析组件初始化影响面 |
| `method:handleClick` | 以 method 为目标，分析调用链路 |
| `data:count` | 以 data 字段 / ref / reactive 为目标 |
| `computed:double` | 以 computed 属性为目标 |
| `prop:title` | 以 props 属性为目标 |

### --direction

| 值 | 说明 |
|---|---|
| `both` | 双向分析（默认） |
| `up` / `upstream` | 只看谁影响目标 |
| `down` / `downstream` | 只看目标影响谁 |

### --from / --to（路径查询）

指定起点和终点，输出从 A 到 B 的所有路径。适合精简分析范围，只关注关键链路。

**注意**：必须同时指定 `--watch`，且 `--watch` 的目标应覆盖 `--from` 节点。

```bash
# ✅ 正确
impact analyze --framework vue --entry src/App.vue --watch method:handleClick \
  --from method:handleClick --to data:count --output-mode cli

# ❌ 会失败（缺少 --watch）
impact analyze --framework vue --entry src/App.vue \
  --from method:handleClick --to data:count
```

路径查询报告格式：

```
# 路径查询报告

起点: method:handleClick
终点: data:count
找到路径数: 3
检测到环数: 1

## 检测到的环
环 1:
  A → B → C → A

## 路径
路径 1: A → D → E
路径 2: A → B → E

## 节点详情
- A [Method] (src/App.vue)
- D [DataField] (src/store.ts)

## 边详情
- A --Calls/Medium--> D
- D --Writes/High--> E
```

### --alias

路径别名解析，支持 `@/` 等配置。

```bash
impact analyze --framework vue --entry src/App.vue --watch data:count \
  --alias @/=src/ --output-mode cli
```

### --cross-module

启用跨模块分析，需配合 `--project-root` 使用。

```bash
impact analyze --framework vue --entry src/App.vue --watch data:count \
  --cross-module --project-root src
```

### --output-mode

| 值 | 说明 |
|---|---|
| `cli` | 仅终端输出（推荐，不写文件） |
| `report` | 仅生成本地报告 |
| `both` | 终端 + 本地报告 |

首次探索建议用 `cli`，确认结果后再用 `report` 生成正式报告。

## 支持的分析能力

- **Options API**：data()、methods、computed、props、lifecycle
- **Composition API**：ref()、reactive()、computed()、顶层函数、xxx.value 读写
- **Pinia**：defineStore 的 state/getters/actions，useXxxStore() 调用追踪
- **数据流**：`this.xxx` 读写、`xxx.value++` 响应式数据
- **方法调用链**：`this.method()`、直接函数调用、带参数调用
- **计算属性依赖**：computed → data/prop 依赖
- **模板绑定**：`{{ }}`、`v-if`、`v-for`、`v-model`、`:prop`、`@event`
- **跨组件链路**：props 下发 (BindsProp)、emit 事件 (EmitsEvent)
- **跨文件链路**：import 依赖递归解析、跨文件方法调用追踪
- **异步链路**：`await`、`.then()`、`.catch()` 数据写入
- **路径查询**：从 A 到 B 的所有路径（DFS，环检测）
- **生命周期**：created、mounted 等钩子中的数据读写

## 工作流

1. 确认分析目标：从用户选中文件、当前焦点文件、报错栈中选择 `--entry`。
2. 默认使用 `--output-mode cli`，不写本地文件。
3. 如果用户关心某个 method/data/computed 的链路，用 `--watch method:xxx` 格式。
4. 如果分析跨组件影响，加 `--cross-module --project-root <dir>`。
5. 如果需要精简路径，用 `--from A --to B`（必须同时带 `--watch`）。
6. 如果项目用 `@/` 别名，加 `--alias @/=src/`。
7. 结论要服务于 debug：说明可能链路、关键节点、风险点。

## 推荐命令模板

### 分析 data 字段影响

```bash
impact analyze --framework vue --entry src/Component.vue --watch data:count --output-mode cli
```

### 分析方法调用链

```bash
impact analyze --framework vue --entry src/Component.vue --watch method:handleClick --output-mode cli
```

### 分析 Pinia store

```bash
impact analyze --framework vue --entry src/views/ProductList.vue --watch data:count --output-mode cli
```

### 跨组件分析

```bash
impact analyze --framework vue --entry src/App.vue --watch data:user \
  --cross-module --project-root src --output-mode cli
```

### 路径查询（精简链路）

```bash
impact analyze --framework vue --entry src/App.vue --watch method:handleClick \
  --from method:handleClick --to data:count --output-mode cli
```

### 使用 alias 解析

```bash
impact analyze --framework vue --entry src/App.vue --watch data:count \
  --alias @/=src/ --output-mode cli
```

### 生成报告文件

```bash
impact analyze --framework vue --entry src/Component.vue --watch data:count \
  --output impact-output/my-analysis --output-mode both
```

## 输出总结格式

默认用中文简洁总结：

```
**Impact 结果**
- 入口：`src/Component.vue`
- 目标：`data:count`
- 关键链路：
  - `Component:method:increment --Writes--> Component:data:count`
  - `Component:computed:double --DependsOn--> Component:data:count`
  - `Component:template:text:1 --Renders--> Component:data:count`
- Debug 关注点：count 被 increment/decrement 修改，影响 computed:double 和模板渲染
- 建议验证：跑单元测试验证 increment 逻辑，检查模板渲染是否正确
```

## 常见问题

| 问题 | 原因 | 解决 |
|------|------|------|
| "Could not find nodes for --from or --to" | `--watch` 未指定或目标不匹配 | 加上 `--watch` 参数，确保覆盖 `--from` 节点 |
| 跨模块分析结果为空 | `--project-root` 路径不对 | 用绝对路径或确认相对路径正确 |
| Composition API store 无结果 | 文件未被 import | 确保 entry 文件直接或间接引用了该 store |
| 路径查询路径数为 0 | 目标不在可达范围内 | 检查图结构，确认起点到终点是否有路 |
| entry 文件不存在 | 路径错误 | 检查文件路径，用绝对路径 |

## 注意事项

- `impact` 结果仅作为辅助排查使用，不对最终结果负责。
- 不要臆造 `entry` 或 `target`。缺少信息时先推断，不明确再问用户。
- 它是静态 may-affect 分析，需要结合代码阅读和测试验证。
- 跨组件分析需要 `--cross-module --project-root` 才能识别父子组件链路。
- Pinia store 分析会自动识别 `defineStore` 模式，无需特殊参数。
- 路径查询用 `--from`/`--to`，必须同时带 `--watch`，输出精简路径报告，适合交给模型分析。
- 如果 CLI 报错，先运行 `impact analyze --help` 检查参数。
