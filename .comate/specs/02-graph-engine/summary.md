# 第二阶段总结

第二阶段的核心目标是实现图构建引擎和 Fixpoint 调度算法，将静态 IR 转换为 may-affect graph。

## 主要工作

1. 重构 graph_builder 模块，实现 Vue IR 到 ImpactGraph 的转换
2. 实现调度引擎，包括 AnalysisStep、FactSet、run_fixpoint
3. 实现 Vue 特定分析器（方法、计算属性、模板、生命周期）
4. 实现目标解析和方向控制
5. 编写单元测试，验证分析结果

## 预期成果

完成第二阶段后，项目将能够：
- 分析 Vue 组件的数据流、方法调用、计算属性依赖
- 生成正确的 may-affect graph
- 支持 data:、method:、computed: 目标类型
- 生成 Markdown 报告，展示影响链路