use crate::model::{Target, TargetKind};

pub fn resolve_watch(watch: &str) -> Option<Target> {
    let watch = watch.trim();
    if watch == "init" {
        return Some(Target { kind: TargetKind::Init, name: None });
    }

    if let Some((kind_str, name)) = watch.split_once(':') {
        let name = name.trim().to_string();
        match kind_str.trim() {
            "data" => Some(Target { kind: TargetKind::Data, name: Some(name) }),
            "method" => Some(Target { kind: TargetKind::Method, name: Some(name) }),
            "computed" => Some(Target { kind: TargetKind::Computed, name: Some(name) }),
            "prop" => Some(Target { kind: TargetKind::Prop, name: Some(name) }),
            _ => None,
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_data() {
        let t = resolve_watch("data:activeTab").unwrap();
        assert_eq!(t.kind, TargetKind::Data);
        assert_eq!(t.name.unwrap(), "activeTab");
    }

    #[test]
    fn test_resolve_method() {
        let t = resolve_watch("method:handleClick").unwrap();
        assert_eq!(t.kind, TargetKind::Method);
        assert_eq!(t.name.unwrap(), "handleClick");
    }

    #[test]
    fn test_resolve_init() {
        let t = resolve_watch("init").unwrap();
        assert_eq!(t.kind, TargetKind::Init);
        assert!(t.name.is_none());
    }

    #[test]
    fn test_resolve_invalid() {
        assert!(resolve_watch("foo:bar:baz").is_none());
        assert!(resolve_watch("").is_none());
    }
}
