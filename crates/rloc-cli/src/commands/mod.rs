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
        .with_backend(rloc_lang_go::backend())
        .with_backend(rloc_lang_shell::backend())
        .with_backend(rloc_lang_sql::backend())
        .with_backend(rloc_lang_web::html_backend())
        .with_backend(rloc_lang_web::css_backend())
        .with_backend(rloc_lang_web::xml_backend())
        .with_backend(rloc_lang_cfamily::c_backend())
        .with_backend(rloc_lang_cfamily::cpp_backend())
        .with_backend(rloc_lang_cfamily::java_backend())
        .with_backend(rloc_lang_cfamily::swift_backend())
        .with_backend(rloc_lang_cfamily::objective_c_backend())
        .with_backend(rloc_lang_cfamily::zig_backend())
        .with_backend(rloc_lang_powershell::backend())
        .with_backend(rloc_core::markdown_backend())
        .with_backend(rloc_core::text_backend())
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

        assert_eq!(languages.len(), 22);
        assert!(languages.contains(&Language::Go));
        assert!(languages.contains(&Language::Html));
        assert!(languages.contains(&Language::Css));
        assert!(languages.contains(&Language::Shell));
        assert!(languages.contains(&Language::Sql));
        assert!(languages.contains(&Language::Rust));
        assert!(languages.contains(&Language::Python));
        assert!(languages.contains(&Language::JavaScript));
        assert!(languages.contains(&Language::TypeScript));
        assert!(languages.contains(&Language::Jsx));
        assert!(languages.contains(&Language::Tsx));
        assert!(languages.contains(&Language::Markdown));
        assert!(languages.contains(&Language::Config));
        assert!(languages.contains(&Language::C));
        assert!(languages.contains(&Language::Cpp));
        assert!(languages.contains(&Language::Java));
        assert!(languages.contains(&Language::Swift));
        assert!(languages.contains(&Language::ObjectiveC));
        assert!(languages.contains(&Language::Zig));
        assert!(languages.contains(&Language::Xml));
        assert!(languages.contains(&Language::PowerShell));
        assert!(languages.contains(&Language::Text));
    }
}
