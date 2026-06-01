# 第二阶段任务清单

## 已完成

- [x] 创建第二阶段设计文档
- [x] 重构 graph_builder 模块，实现 Vue IR 到 ImpactGraph 的转换
- [x] 实现数据字段节点生成
- [x] 实现可执行文件节点生成（方法、生命周期、计算属性）
- [x] 实现模板绑定节点生成
- [x] 实现边生成逻辑（读写、调用、模板绑定）
- [x] 实现基于目标的图裁剪
- [x] 实现数据读写提取（this.data.get/set + this.xxx 代理）
- [x] 实现方法调用提取（this.method()）
- [x] 实现模板绑定边生成
- [x] 实现 Vue 2 代理访问识别（this.xxx++）
- [x] 实现 Vue 3 Composition API 计算属性识别
- [x] 编写图构建单元测试（25 个测试全部通过）
- [x] 验证 Counter.vue 分析结果（data/method/computed 三种目标）
- [x] 生成正确的 Markdown 报告

## 待完成

### 调度引擎

- [ ] 完善 AnalysisStep 结构
- [ ] 完善 FactSet 逻辑
- [ ] 完善 run_fixpoint 循环
- [ ] 实现步骤前置事实检查
- [ ] 实现事实合并逻辑

### 分析器增强

- [ ] 实现 computed 依赖自动提取
- [ ] 实现 LifecycleAnalyzer：分析生命周期钩子的副作用
- [ ] 实现 async/Promise 链路
- [ ] 实现跨组件 props/emit 链路

### 目标解析

- [ ] 实现 up/down/both 方向控制
- [ ] 实现主链路裁剪

### 测试验证

- [ ] 编写调度引擎单元测试
- [ ] 编写跨组件分析集成测试
