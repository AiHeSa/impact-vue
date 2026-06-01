# 第三阶段：跨组件链路与分析增强

## 背景

第二阶段实现了单组件的图构建引擎，能够分析 data/method/computed 的影响链路。第三阶段需要：

1. 自动提取 computed 依赖
2. 跨组件 props/emit 链路
3. lifecycle 副作用分析
4. up/down 方向控制

## 目标

### 3.1 Computed 依赖自动提取

从计算属性体中识别 `this.xxx`，自动建立 computed → data 依赖边。

### 3.2 跨组件链路

支持父子组件之间的数据流：
- 父组件 `:prop="data"` → 子组件 `props` → 子组件 `this.prop`
- 子组件 `this.$emit('event')` → 父组件 `@event="handler"`

### 3.3 Lifecycle 副作用分析

识别 lifecycle 钩子中的数据读写和外部副作用。

### 3.4 Up/Down 方向控制

- `--direction up`：谁可能影响目标（upstream）
- `--direction down`：目标可能影响谁（downstream）
- `--direction both`：双向分析

## 验证场景

```
Parent.vue
├── data: count
├── @update → handleUpdate
└── <Child :count="count" @update="handleUpdate" />

Child.vue
├── props: ['count']
├── computed: double (依赖 this.count)
├── methods: increment → this.$emit('update')
└── template: {{ double }}
```

分析 `data:count` 应该找到：
- Parent:data:count → Child:prop:count (BindsProp)
- Child:prop:count → Child:computed:double (DependsOn)
- Child:method:increment → Child:event:update (EmitsEvent)
- Child:event:update → Parent:method:handleUpdate (HandlesEvent)
