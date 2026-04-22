use camino::Utf8Path;

use crate::types::ScanOptions;

pub const BUILTIN_IGNORE_RULES: [&str; 14] = [
    ".git/",
    "target/",
    "node_modules/",
    "dist/",
    "build/",
    "coverage/",
    ".next/",
    ".turbo/",
    ".venv/",
    "venv/",
    "__pycache__/",
    ".pytest_cache/",
    ".mypy_cache/",
    ".ruff_cache/",
];

pub fn is_ignored(path: &Utf8Path) -> bool {
    path.components().any(|component| {
        BUILTIN_IGNORE_RULES.iter().any(|rule| {
            let component_name = rule.trim_end_matches('/');
            component.as_str() == component_name
        })
    })
}

pub fn active_ignore_rules(options: &ScanOptions) -> Vec<String> {
    let mut rules = BUILTIN_IGNORE_RULES
        .iter()
        .map(|rule| (*rule).to_owned())
        .collect::<Vec<_>>();

    if options.respect_gitignore {
        rules.push(".gitignore".to_owned());
    }

    rules
}

#[cfg(test)]
mod tests {
    use camino::Utf8Path;

    use super::{active_ignore_rules, is_ignored};
    use crate::ScanOptions;

    #[test]
    fn ignores_root_level_noise_directories() {
        assert!(is_ignored(Utf8Path::new("target/debug/rloc")));
        assert!(is_ignored(Utf8Path::new(".venv/bin/python")));
    }

    #[test]
    fn keeps_regular_source_paths() {
        assert!(!is_ignored(Utf8Path::new("src/main.rs")));
    }

    #[test]
    fn advertises_gitignore_when_enabled() {
        let rules = active_ignore_rules(&ScanOptions::default());
        assert!(rules.iter().any(|rule| rule == ".gitignore"));
    }
}
