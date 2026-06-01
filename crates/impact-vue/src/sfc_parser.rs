use regex::Regex;

#[allow(dead_code)]
pub struct SfcBlock {
    pub content: String,
    pub start_line: usize,
}

#[allow(dead_code)]
pub struct SfcParseResult {
    pub template: SfcBlock,
    pub script: SfcBlock,
    pub script_setup: Option<SfcBlock>,
    pub style: Option<SfcBlock>,
}

pub struct SfcParser;

impl SfcParser {
    pub fn parse(source: &str) -> anyhow::Result<SfcParseResult> {
        let template = Self::extract_block(source, "template")?;
        let script_setup = Self::extract_script_setup(source)?;
        let style = Self::extract_block(source, "style")?;

        let script = if script_setup.is_some() {
            Self::extract_script_excluding_setup(source)?
        } else {
            Self::extract_block(source, "script")?
        };

        Ok(SfcParseResult {
            template: template.unwrap_or_else(|| SfcBlock {
                content: String::new(),
                start_line: 0,
            }),
            script: script.unwrap_or_else(|| SfcBlock {
                content: String::new(),
                start_line: 0,
            }),
            script_setup,
            style,
        })
    }

    fn extract_block(source: &str, tag: &str) -> anyhow::Result<Option<SfcBlock>> {
        let escaped = regex::escape(tag);
        let pattern = format!(r"(?s)<{escaped}[^>]*>(.*?)</{escaped}>");
        let re = Regex::new(&pattern)?;

        Ok(re.captures(source).map(|c| {
            let content = c[1].to_string();
            let start_line = source[..c.get(1).unwrap().start()].matches('\n').count() + 1;
            SfcBlock { content, start_line }
        }))
    }

    fn extract_script_setup(source: &str) -> anyhow::Result<Option<SfcBlock>> {
        let re = Regex::new(r"(?s)<script\s+setup\b[^>]*>(.*?)</script>")?;
        Ok(re.captures(source).map(|c| {
            let content = c[1].to_string();
            let start_line = source[..c.get(1).unwrap().start()].matches('\n').count() + 1;
            SfcBlock { content, start_line }
        }))
    }

    fn extract_script_excluding_setup(source: &str) -> anyhow::Result<Option<SfcBlock>> {
        let re = Regex::new(r"(?s)<script\b([^>]*)>(.*?)</script>")?;

        for cap in re.captures_iter(source) {
            let attrs = &cap[1];
            if !attrs.contains("setup") {
                let content = cap[2].to_string();
                let start_line = source[..cap.get(2).unwrap().start()].matches('\n').count() + 1;
                return Ok(Some(SfcBlock { content, start_line }));
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_sfc() {
        let source = r#"
<template>
  <div>{{ message }}</div>
</template>

<script>
export default {
  data() {
    return { message: 'hello' }
  }
}
</script>

<style scoped>
div { color: red; }
</style>
"#;
        let result = SfcParser::parse(source).unwrap();
        assert!(result.template.content.contains("{{ message }}"));
        assert!(result.script.content.contains("message: 'hello'"));
        assert!(result.style.is_some());
        assert!(result.script_setup.is_none());
    }

    #[test]
    fn test_parse_script_setup() {
        let source = r#"
<template>
  <div>{{ count }}</div>
</template>

<script setup>
import { ref } from 'vue'
const count = ref(0)
</script>
"#;
        let result = SfcParser::parse(source).unwrap();
        assert!(result.script_setup.is_some());
        let setup = result.script_setup.unwrap();
        assert!(setup.content.contains("ref(0)"));
    }

    #[test]
    fn test_parse_no_template() {
        let source = "<script>export default {}</script>";
        let result = SfcParser::parse(source).unwrap();
        assert!(result.template.content.is_empty());
    }

    #[test]
    fn test_setup_excludes_regular_script() {
        let source = r#"
<script setup>
const count = ref(0)
</script>
<script>
export default {}
</script>
"#;
        let result = SfcParser::parse(source).unwrap();
        assert!(result.script_setup.is_some());
        assert!(result.script.content.contains("export default"));
    }
}
