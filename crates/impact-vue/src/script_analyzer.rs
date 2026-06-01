use regex::Regex;

use impact_core::model::{
    ComputedIr, DataFieldIr, EventIr, ExecutableIr, ExecutableKind, ImportIr, PropIr,
};

pub struct ScriptAnalyzer;

impl ScriptAnalyzer {
    pub fn extract_data_fields(script: &str) -> Vec<DataFieldIr> {
        if script.is_empty() {
            return Vec::new();
        }

        let mut fields = Vec::new();

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

        let re_ref = Regex::new(r#"(\w+)\s*=\s*ref\s*\(\s*"#).unwrap();
        for cap in re_ref.captures_iter(script) {
            fields.push(DataFieldIr {
                name: cap[1].to_string(),
                default_value: None,
                line: 0,
            });
        }

        let re_reactive = Regex::new(r#"(\w+)\s*=\s*reactive\s*\(\s*\{([^}]*)\}\s*\)"#).unwrap();
        for cap in re_reactive.captures_iter(script) {
            let body = &cap[2];
            let re_inner = Regex::new(r#"(\w+)\s*:"#).unwrap();
            for inner in re_inner.captures_iter(body) {
                fields.push(DataFieldIr {
                    name: inner[1].to_string(),
                    default_value: None,
                    line: 0,
                });
            }
        }

        fields
    }

    pub fn extract_computed(script: &str) -> Vec<ComputedIr> {
        if script.is_empty() {
            return Vec::new();
        }

        let mut computed = Vec::new();

        let re_computed_block = Regex::new(r"computed\s*:\s*\{").unwrap();
        if let Some(m) = re_computed_block.find(script) {
            let rest = &script[m.end()..];
            let body = extract_balanced_brace(rest);
            let re_method = Regex::new(r#"(\w+)\s*\(\s*\)\s*\{"#).unwrap();
            for cap in re_method.captures_iter(&body) {
                let name = cap[1].to_string();
                let method_start = cap.get(0).unwrap().end();
                let method_body = extract_balanced_brace(&body[method_start..]);
                let deps = Self::extract_this_refs(&method_body);
                computed.push(ComputedIr {
                    name,
                    deps,
                    line: 0,
                });
            }
        }

        let re_computed_api = Regex::new(r#"(\w+)\s*=\s*computed\s*\(\s*\(\s*\)\s*=>"#).unwrap();
        for cap in re_computed_api.captures_iter(script) {
            let name = cap[1].to_string();
            let fn_start = cap.get(0).unwrap().end();
            let fn_body = extract_balanced_brace(&script[fn_start..]);
            let deps = Self::extract_this_refs(&fn_body);
            computed.push(ComputedIr {
                name,
                deps,
                line: 0,
            });
        }

        computed
    }

    fn extract_this_refs(body: &str) -> Vec<String> {
        let re = Regex::new(r#"this\.(\w+)"#).unwrap();
        let exclude = [
            "data", "methods", "computed", "props", "emit", "$emit", "$refs",
            "$el", "$options", "$parent", "$root", "$children", "$slots",
            "$scopedSlots", "$attrs", "$listeners", "$watch", "$set",
            "$delete", "$nextTick", "$on", "$once", "$off", "$mount",
            "$forceUpdate", "$destroy",
        ];
        let mut refs = Vec::new();
        for cap in re.captures_iter(body) {
            let name = cap[1].to_string();
            if !exclude.contains(&name.as_str()) && !refs.contains(&name) {
                refs.push(name);
            }
        }
        refs
    }

    pub fn extract_props(script: &str) -> Vec<PropIr> {
        if script.is_empty() {
            return Vec::new();
        }

        let mut props = Vec::new();

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

    pub fn extract_methods(script: &str) -> Vec<ExecutableIr> {
        if script.is_empty() {
            return Vec::new();
        }

        let mut methods = Vec::new();

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

        let re_fn = Regex::new(r#"(?:const|let|var)\s+(\w+)\s*=\s*\([^)]*\)\s*=>\s*\{|\bfunction\s+(\w+)\s*\("#).unwrap();
        for cap in re_fn.captures_iter(script) {
            let name = cap.get(1).or_else(|| cap.get(2)).unwrap().as_str().to_string();
            let fn_start = cap.get(0).unwrap().end();
            let fn_body = extract_balanced_brace(&script[fn_start..]);
            methods.push(ExecutableIr {
                kind: ExecutableKind::Method,
                name,
                body: fn_body,
                line: 0,
            });
        }

        methods
    }

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

    pub fn extract_imports(script: &str) -> Vec<ImportIr> {
        if script.is_empty() {
            return Vec::new();
        }

        let mut imports = Vec::new();

        let re_import = Regex::new(
            r#"import\s+(?:\{\s*([^}]*)\s*\}|\*\s+as\s+(\w+)|(\w+))\s+from\s+['"]([^'"]+)['"]"#,
        )
        .unwrap();

        for cap in re_import.captures_iter(script) {
            let source = cap.get(4).map(|m| m.as_str().to_string()).unwrap_or_default();

            if let Some(name) = cap.get(3) {
                imports.push(ImportIr {
                    source,
                    imported_name: Some(name.as_str().to_string()),
                    is_default: true,
                    line: 0,
                });
            } else if let Some(name) = cap.get(2) {
                imports.push(ImportIr {
                    source,
                    imported_name: Some(name.as_str().to_string()),
                    is_default: false,
                    line: 0,
                });
            } else if let Some(names) = cap.get(1) {
                let re_name = Regex::new(r#"(\w+)"#).unwrap();
                for n in re_name.captures_iter(names.as_str()) {
                    imports.push(ImportIr {
                        source: source.clone(),
                        imported_name: Some(n[1].to_string()),
                        is_default: false,
                        line: 0,
                    });
                }
            }
        }

        imports
    }

    pub fn extract_events(script: &str) -> Vec<EventIr> {
        if script.is_empty() {
            return Vec::new();
        }

        let mut events = Vec::new();

        let re_emit = Regex::new(r#"emit\s*\(\s*['"]([^'"]+)['"]"#).unwrap();
        for cap in re_emit.captures_iter(script) {
            events.push(EventIr {
                event_name: cap[1].to_string(),
                handler_name: String::new(),
                line: 0,
            });
        }

        let re_emit_alt = Regex::new(r#"\$emit\s*\(\s*['"]([^'"]+)['"]"#).unwrap();
        for cap in re_emit_alt.captures_iter(script) {
            events.push(EventIr {
                event_name: cap[1].to_string(),
                handler_name: String::new(),
                line: 0,
            });
        }

        events
    }
}

fn extract_balanced_brace(s: &str) -> String {
    let mut depth = 1;

    for (i, ch) in s.char_indices() {
        if ch == '{' {
            depth += 1;
        } else if ch == '}' {
            depth -= 1;
            if depth == 0 {
                return s[..i].to_string();
            }
        }
    }

    String::new()
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
        // 注意：当前实现提取了 this.count 作为依赖
        assert!(computed[0].deps.contains(&"count".to_string()));
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
}
