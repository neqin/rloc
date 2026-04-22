use std::fmt;

use camino::Utf8Path;
use rloc_core::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Rust,
    Python,
    React,
    Monorepo,
}

impl Preset {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::React => "react",
            Self::Monorepo => "monorepo",
        }
    }
}

impl fmt::Display for Preset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

pub fn detect_presets(root: &Utf8Path, languages: &[Language]) -> Vec<Preset> {
    let mut presets = Vec::new();

    if root.join("Cargo.toml").exists() || has_language(languages, Language::Rust) {
        presets.push(Preset::Rust);
    }
    if root.join("pyproject.toml").exists()
        || root.join("requirements.txt").exists()
        || root.join("setup.py").exists()
        || has_language(languages, Language::Python)
    {
        presets.push(Preset::Python);
    }
    if root.join("package.json").exists()
        && has_any_language(languages, &[Language::Jsx, Language::Tsx])
    {
        presets.push(Preset::React);
    }
    if root.join("pnpm-workspace.yaml").exists()
        || root.join("turbo.json").exists()
        || root.join("nx.json").exists()
        || root.join("lerna.json").exists()
        || root.join("packages").is_dir()
        || root.join("crates").is_dir()
    {
        presets.push(Preset::Monorepo);
    }

    presets
}

fn has_language(languages: &[Language], expected: Language) -> bool {
    languages.contains(&expected)
}

fn has_any_language(languages: &[Language], expected: &[Language]) -> bool {
    expected.iter().any(|language| languages.contains(language))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use camino::{Utf8Path, Utf8PathBuf};
    use rloc_core::Language;

    use super::{Preset, detect_presets};

    #[test]
    fn detects_language_driven_presets() {
        let root = temp_dir("detects_language_driven_presets");
        fs::write(
            root.join("Cargo.toml").as_std_path(),
            "[package]\nname = \"demo\"\n",
        )
        .unwrap();
        fs::write(
            root.join("pyproject.toml").as_std_path(),
            "[project]\nname = \"demo\"\n",
        )
        .unwrap();
        fs::write(root.join("package.json").as_std_path(), "{}\n").unwrap();
        fs::create_dir_all(root.join("packages").as_std_path()).unwrap();

        let presets = detect_presets(&root, &[Language::Rust, Language::Python, Language::Tsx]);
        assert_eq!(
            presets,
            vec![
                Preset::Rust,
                Preset::Python,
                Preset::React,
                Preset::Monorepo
            ]
        );

        cleanup(&root);
    }

    #[test]
    fn skips_react_without_jsx_or_tsx_sources() {
        let root = temp_dir("skips_react_without_jsx_or_tsx_sources");
        fs::write(root.join("package.json").as_std_path(), "{}\n").unwrap();

        let presets = detect_presets(&root, &[Language::JavaScript, Language::TypeScript]);
        assert!(presets.is_empty());

        cleanup(&root);
    }

    fn temp_dir(test_name: &str) -> Utf8PathBuf {
        let unique = format!(
            "rloc-presets-{test_name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        fs::create_dir_all(&path).unwrap();
        Utf8PathBuf::from_path_buf(path).unwrap()
    }

    fn cleanup(root: &Utf8Path) {
        if root.exists() {
            fs::remove_dir_all(root.as_std_path()).unwrap();
        }
    }
}
