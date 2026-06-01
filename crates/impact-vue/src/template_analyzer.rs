use regex::Regex;

use impact_core::model::TemplateBindingIr;

pub struct TemplateAnalyzer;

impl TemplateAnalyzer {
    pub fn extract_bindings(template: &str) -> Vec<TemplateBindingIr> {
        if template.is_empty() {
            return Vec::new();
        }

        let mut bindings = Vec::new();
        let mut node_counter = 0;

        Self::extract_mustache(template, &mut bindings, &mut node_counter);
        Self::extract_directives(template, &mut bindings, &mut node_counter);
        Self::extract_event_bindings(template, &mut bindings, &mut node_counter);

        bindings
    }

    fn extract_mustache(
        template: &str,
        bindings: &mut Vec<TemplateBindingIr>,
        counter: &mut usize,
    ) {
        let re = Regex::new(r"\{\{\s*([^}]+)\s*\}\}").unwrap();
        for cap in re.captures_iter(template) {
            let expr = cap[1].to_string();
            let paths = Self::extract_data_paths(&expr);
            for path in paths {
                *counter += 1;
                bindings.push(TemplateBindingIr {
                    node_id: format!("text:{}", *counter),
                    kind: "text".to_string(),
                    expression: expr.clone(),
                    data_paths: vec![path],
                    line: 0,
                });
            }
        }
    }

    fn extract_directives(
        template: &str,
        bindings: &mut Vec<TemplateBindingIr>,
        counter: &mut usize,
    ) {
        let dirs = [
            (r#"v-if\s*=\s*["']([^"']+)["']"#, "v-if"),
            (r#"v-else-if\s*=\s*["']([^"']+)["']"#, "v-else-if"),
            (r#"v-show\s*=\s*["']([^"']+)["']"#, "v-show"),
            (r#"v-for\s*=\s*["']([^"']+)["']"#, "v-for"),
            (r#":([\w-]+)\s*=\s*["']([^"']+)["']"#, "bind"),
            (r#"v-bind:([\w-]+)\s*=\s*["']([^"']+)["']"#, "bind"),
            (r#"v-model\s*=\s*["']([^"']+)["']"#, "v-model"),
        ];

        for (pattern, kind) in &dirs {
            if let Ok(re) = Regex::new(pattern) {
                for cap in re.captures_iter(template) {
                    let expr = if *kind == "bind" {
                        cap[2].to_string()
                    } else {
                        cap[1].to_string()
                    };
                    let paths = Self::extract_data_paths(&expr);
                    for path in paths {
                        *counter += 1;
                        bindings.push(TemplateBindingIr {
                            node_id: format!("{}:{}", kind, *counter),
                            kind: kind.to_string(),
                            expression: expr.clone(),
                            data_paths: vec![path],
                            line: 0,
                        });
                    }
                }
            }
        }
    }

    fn extract_event_bindings(
        template: &str,
        bindings: &mut Vec<TemplateBindingIr>,
        counter: &mut usize,
    ) {
        let re = Regex::new(r#"@([\w-]+)\s*=\s*["']([^"']+)["']"#).unwrap();
        for cap in re.captures_iter(template) {
            let event = cap[1].to_string();
            let handler = cap[2].to_string();
            *counter += 1;
            bindings.push(TemplateBindingIr {
                node_id: format!("event:{}", *counter),
                kind: format!("@{}", event),
                expression: handler.clone(),
                data_paths: vec![handler],
                line: 0,
            });
        }
    }

    fn extract_data_paths(expr: &str) -> Vec<String> {
        let mut paths = Vec::new();
        let re = Regex::new(r"\b([a-zA-Z_$][\w$]*)\b").unwrap();
        let keywords = [
            "true", "false", "null", "undefined", "this", "typeof", "instanceof",
            "if", "else", "return", "for", "while", "in", "of",
            "item", "index", "key", "value",
        ];

        for cap in re.captures_iter(expr) {
            let name = cap[1].to_string();
            if !keywords.contains(&name.as_str()) {
                paths.push(name);
            }
        }

        paths
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_mustache() {
        let tmpl = "<div>{{ count }}</div>";
        let bindings = TemplateAnalyzer::extract_bindings(tmpl);
        assert!(bindings.iter().any(|b| b.kind == "text" && b.data_paths.contains(&"count".to_string())));
    }

    #[test]
    fn test_extract_v_if() {
        let tmpl = "<div v-if=\"show\">content</div>";
        let bindings = TemplateAnalyzer::extract_bindings(tmpl);
        assert!(bindings.iter().any(|b| b.kind == "v-if" && b.data_paths.contains(&"show".to_string())));
    }

    #[test]
    fn test_extract_v_for() {
        let tmpl = "<li v-for=\"item in items\">{{ item.name }}</li>";
        let bindings = TemplateAnalyzer::extract_bindings(tmpl);
        assert!(bindings.iter().any(|b| b.kind == "v-for" && b.data_paths.contains(&"items".to_string())));
    }

    #[test]
    fn test_extract_event() {
        let tmpl = "<button @click=\"handleClick\">click</button>";
        let bindings = TemplateAnalyzer::extract_bindings(tmpl);
        assert!(bindings.iter().any(|b| b.kind == "@click" && b.data_paths.contains(&"handleClick".to_string())));
    }

    #[test]
    fn test_extract_bind() {
        let tmpl = "<div :class=\"activeClass\">content</div>";
        let bindings = TemplateAnalyzer::extract_bindings(tmpl);
        assert!(bindings.iter().any(|b| b.kind == "bind" && b.data_paths.contains(&"activeClass".to_string())));
    }
}
