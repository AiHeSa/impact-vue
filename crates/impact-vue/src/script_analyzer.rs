//! Vue 脚本分析器
//!
//! 基于 impact-js-ts 的通用能力，提供 Vue 特定的分析：
//! - Options API: data(), methods, computed, props, lifecycle
//! - Composition API: ref(), reactive(), computed(), 生命周期钩子

use impact_core::model::{
    ComputedIr, DataFieldIr, EventIr, ExecutableIr, ExecutableKind, ImportIr, PropIr,
};
use impact_js_ts::{JsTsAnalyzer, extract_balanced_brace};
use regex::Regex;

/// Vue 脚本分析器
pub struct ScriptAnalyzer;

impl ScriptAnalyzer {
    /// 提取数据字段（Options API data() + Composition API ref/reactive）
    pub fn extract_data_fields(script: &str) -> Vec<DataFieldIr> {
        if script.is_empty() {
            return Vec::new();
        }

        let mut fields = Vec::new();

        // Options API: data() { return { ... } }
        let re_data = Regex::new(r"data\s*\(\s*\)\s*\{").unwrap();
        if let Some(data_match) = re_data.find(script) {
            let rest = &script[data_match.end()..];
            let body = extract_balanced_brace(rest);
            let re_field = Regex::new(r#"(\w+)\s*:"#).unwrap();
            for cap in re_field.captures_iter(&body) {
                fields.push(DataFieldIr {
                    name: cap[1].to_string(),
                    default_value: None,
                    line: 0,
                });
            }
        }

        // Composition API: ref() / reactive()
        fields.extend(JsTsAnalyzer::extract_reactive_fields(script));

        fields
    }

    /// 提取计算属性
    pub fn extract_computed(script: &str) -> Vec<ComputedIr> {
        if script.is_empty() {
            return Vec::new();
        }

        let mut computed = Vec::new();

        // Options API: computed: { ... }
        let re_computed_block = Regex::new(r"computed\s*:\s*\{").unwrap();
        if let Some(m) = re_computed_block.find(script) {
            let rest = &script[m.end()..];
            let body = extract_balanced_brace(rest);
            let re_method = Regex::new(r#"(\w+)\s*\(\s*\)\s*\{"#).unwrap();
            for cap in re_method.captures_iter(&body) {
                let name = cap[1].to_string();
                let method_start = cap.get(0).unwrap().end();
                let method_body = extract_balanced_brace(&body[method_start..]);
                let deps = JsTsAnalyzer::extract_this_refs(&method_body);
                computed.push(ComputedIr {
                    name,
                    deps,
                    line: 0,
                });
            }
        }

        // Composition API: computed(() => ...)
        let re_computed_api = Regex::new(r#"(\w+)\s*=\s*computed\s*\(\s*\(\s*\)\s*=>"#).unwrap();
        for cap in re_computed_api.captures_iter(script) {
            let name = cap[1].to_string();
            let fn_start = cap.get(0).unwrap().end();
            let fn_body = extract_balanced_brace(&script[fn_start..]);
            let deps = JsTsAnalyzer::extract_this_refs(&fn_body);
            computed.push(ComputedIr {
                name,
                deps,
                line: 0,
            });
        }

        computed
    }

    /// 提取属性
    pub fn extract_props(script: &str) -> Vec<PropIr> {
        if script.is_empty() {
            return Vec::new();
        }

        let mut props = Vec::new();

        // Options API: props: ['a', 'b']
        let re_props = Regex::new(r"props\s*:\s*\[([^\]]*)\]").unwrap();
        if let Some(cap) = re_props.captures(script) {
            let body = &cap[1];
            let re_str = Regex::new(r#""([^"]+)"|'([^']+)'"#).unwrap();
            for cap in re_str.captures_iter(body) {
                let name = cap.get(1).or_else(|| cap.get(2)).unwrap().as_str().to_string();
                props.push(PropIr {
                    name,
                    prop_type: None,
                    line: 0,
                });
            }
        }

        props
    }

    /// 提取方法
    pub fn extract_methods(script: &str) -> Vec<ExecutableIr> {
        if script.is_empty() {
            return Vec::new();
        }

        let mut methods = Vec::new();

        // Options API: methods: { ... }
        let re_methods_block = Regex::new(r"methods\s*:\s*\{").unwrap();
        if let Some(m) = re_methods_block.find(script) {
            let rest = &script[m.end()..];
            let body = extract_balanced_brace(rest);
            let re_method = Regex::new(r#"(\w+)\s*\(\s*[^)]*\)\s*\{"#).unwrap();
            for cap in re_method.captures_iter(&body) {
                let name = cap[1].to_string();
                let method_start = cap.get(0).unwrap().end();
                let method_body = extract_balanced_brace(&body[method_start..]);
                methods.push(ExecutableIr {
                    kind: ExecutableKind::Method,
                    name,
                    body: method_body,
                    line: 0,
                });
            }
        }

        // Composition API: 顶层函数
        methods.extend(JsTsAnalyzer::extract_functions(script));

        methods
    }

    /// 提取生命周期钩子
    pub fn extract_lifecycle_hooks(script: &str) -> Vec<ExecutableIr> {
        if script.is_empty() {
            return Vec::new();
        }

        let hooks = [
            "created", "mounted", "beforeMount", "beforeUpdate",
            "updated", "beforeUnmount", "unmounted", "activated",
            "deactivated", "beforeCreate",
        ];

        let mut executables = Vec::new();

        for hook in &hooks {
            let pattern = format!(r#"{}\s*\(\s*\)\s*\{{"#, regex::escape(hook));
            if let Ok(re) = Regex::new(&pattern) {
                if let Some(m) = re.find(script) {
                    let rest = &script[m.end()..];
                    let body = extract_balanced_brace(rest);
                    executables.push(ExecutableIr {
                        kind: ExecutableKind::Lifecycle,
                        name: hook.to_string(),
                        body,
                        line: 0,
                    });
                }
            }
        }

        let composition_hooks = ["onMounted", "onUnmounted", "onUpdated", "onBeforeMount", "onBeforeUnmount"];
        for hook in &composition_hooks {
            if script.contains(hook) {
                executables.push(ExecutableIr {
                    kind: ExecutableKind::Lifecycle,
                    name: hook.to_string(),
                    body: String::new(),
                    line: 0,
                });
            }
        }

        executables
    }

    /// 提取计算属性可执行体
    pub fn extract_computed_executables(script: &str) -> Vec<ExecutableIr> {
        let computed = Self::extract_computed(script);
        computed
            .into_iter()
            .map(|c| ExecutableIr {
                kind: ExecutableKind::Computed,
                name: c.name,
                body: String::new(),
                line: 0,
            })
            .collect()
    }

    /// 提取导入
    pub fn extract_imports(script: &str) -> Vec<ImportIr> {
        JsTsAnalyzer::extract_imports(script)
    }

    /// 提取事件
    pub fn extract_events(script: &str) -> Vec<EventIr> {
        JsTsAnalyzer::extract_events(script)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_options_data() {
        let script = r#"
export default {
  data() {
    return {
      count: 0,
      message: 'hello'
    }
  }
}"#;
        let fields = ScriptAnalyzer::extract_data_fields(script);
        assert_eq!(fields.len(), 2);
        assert!(fields.iter().any(|f| f.name == "count"));
        assert!(fields.iter().any(|f| f.name == "message"));
    }

    #[test]
    fn test_extract_ref_fields() {
        let script = r#"
import { ref } from 'vue'
const count = ref(0)
const name = ref('hello')
"#;
        let fields = ScriptAnalyzer::extract_data_fields(script);
        assert!(fields.iter().any(|f| f.name == "count"));
        assert!(fields.iter().any(|f| f.name == "name"));
    }

    #[test]
    fn test_extract_computed() {
        let script = r#"
export default {
  computed: {
    double() {
      return this.count * 2
    }
  }
}"#;
        let computed = ScriptAnalyzer::extract_computed(script);
        assert_eq!(computed.len(), 1);
        assert_eq!(computed[0].name, "double");
    }

    #[test]
    fn test_extract_computed_multiple() {
        let script = r#"
export default {
  computed: {
    double() {
      return this.count * 2
    },
    triple() {
      return this.count * 3
    }
  }
}"#;
        let computed = ScriptAnalyzer::extract_computed(script);
        assert_eq!(computed.len(), 2);
        assert!(computed.iter().any(|c| c.name == "double"));
        assert!(computed.iter().any(|c| c.name == "triple"));
    }

    #[test]
    fn test_extract_computed_from_full_script() {
        let script = r#"
export default {
  data() {
    return { count: 0 }
  },
  computed: {
    double() {
      return this.count * 2
    }
  },
  methods: {
    increment() {
      this.count++
    }
  }
}"#;
        let computed = ScriptAnalyzer::extract_computed(script);
        assert_eq!(computed.len(), 1);
        assert_eq!(computed[0].name, "double");
    }

    #[test]
    fn test_extract_methods() {
        let script = r#"
export default {
  methods: {
    handleClick() {},
    fetchData() {}
  }
}"#;
        let methods = ScriptAnalyzer::extract_methods(script);
        assert_eq!(methods.len(), 2);
    }

    #[test]
    fn test_extract_imports() {
        let script = r#"
import { ref, computed } from 'vue'
import DefaultComp from './Comp.vue'
import * as Utils from './utils'
"#;
        let imports = ScriptAnalyzer::extract_imports(script);
        assert_eq!(imports.len(), 4);
    }

    #[test]
    fn test_extract_lifecycle() {
        let script = r#"
export default {
  created() {},
  mounted() {}
}"#;
        let hooks = ScriptAnalyzer::extract_lifecycle_hooks(script);
        assert!(hooks.iter().any(|h| h.name == "created"));
        assert!(hooks.iter().any(|h| h.name == "mounted"));
    }

    #[test]
    fn test_extract_props() {
        let script = r#"
export default {
  props: ['title', 'count']
}"#;
        let props = ScriptAnalyzer::extract_props(script);
        assert_eq!(props.len(), 2);
    }

    #[test]
    fn test_extract_composition_functions() {
        let script = r#"
import { ref } from 'vue'
const count = ref(0)

function increment() {
  count.value++
}

const reset = () => {
  count.value = 0
}
"#;
        let methods = ScriptAnalyzer::extract_methods(script);
        assert!(methods.iter().any(|m| m.name == "increment"));
        assert!(methods.iter().any(|m| m.name == "reset"));
    }
}
