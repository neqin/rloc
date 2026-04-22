pub mod config;
pub mod detect;
pub mod explain;
pub mod scan;

use rloc_core::LanguageBackendRegistry;

use crate::cli::{Cli, Command};
use crate::error::AppError;

pub(crate) fn default_registry() -> LanguageBackendRegistry {
    LanguageBackendRegistry::new()
        .with_backend(rloc_lang_rust::backend())
        .with_backend(rloc_lang_python::backend())
        .with_backend(rloc_lang_js::javascript_backend())
        .with_backend(rloc_lang_js::typescript_backend())
        .with_backend(rloc_lang_js::jsx_backend())
        .with_backend(rloc_lang_js::tsx_backend())
        .with_backend(rloc_core::markdown_backend())
        .with_backend(rloc_core::config_backend())
}

pub fn run(cli: Cli) -> Result<(), AppError> {
    match cli.command {
        Command::Scan(args) => scan::run(args),
        Command::Explain(args) => explain::run(args),
        Command::Detect(args) => detect::run(args),
        Command::Config(args) => config::run(args),
    }
}

#[cfg(test)]
mod tests {
    use super::default_registry;
    use rloc_core::Language;

    #[test]
    fn default_registry_wires_all_workspace_backends() {
        let languages = default_registry().supported_languages();

        assert_eq!(languages.len(), 8);
        assert!(languages.contains(&Language::Rust));
        assert!(languages.contains(&Language::Python));
        assert!(languages.contains(&Language::JavaScript));
        assert!(languages.contains(&Language::TypeScript));
        assert!(languages.contains(&Language::Jsx));
        assert!(languages.contains(&Language::Tsx));
        assert!(languages.contains(&Language::Markdown));
        assert!(languages.contains(&Language::Config));
    }
}
