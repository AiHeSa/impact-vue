# 项目初始化 — 总结

已完成 impact-vue 项目的基础骨架搭建，包括：

- 4 crate 的 Rust workspace
- Core 数据模型（IR、Graph、Confidence、Evidence、Target）
- FrameworkAdapter 特征定义与注册
- Vue SFC 解析器（Options API + Composition API）
- Vue Script 分析器（data/methods/computed/props/imports/lifecycle）
- Vue Template 分析器（mustache/directives/events）
- CLI 入口与 analyze 命令
- Cross-module 分析骨架
- Markdown/Mermaid/JSON 输出
- 测试用例与 fixtures

项目可编译运行：`cargo build`。
