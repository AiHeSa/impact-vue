use std::path::{Path, PathBuf};

pub fn resolve_relative_file(
    current_file: &Path,
    import_path: &str,
    extensions: &[&str],
) -> Option<PathBuf> {
    let parent = current_file.parent()?;
    let candidate = parent.join(import_path);

    if candidate.exists() {
        return Some(candidate);
    }

    for ext in extensions {
        let with_ext = candidate.with_extension(ext);
        if with_ext.exists() {
            return Some(with_ext);
        }
    }

    let index_candidate = candidate.join("index");
    for ext in extensions {
        let with_ext = index_candidate.with_extension(ext);
        if with_ext.exists() {
            return Some(with_ext);
        }
    }

    None
}
