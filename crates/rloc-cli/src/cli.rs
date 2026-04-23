use std::ffi::OsString;

use camino::Utf8PathBuf;
use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "rloc",
    version,
    about = "Repository line-of-code analyzer",
    override_usage = "rloc [SCAN_OPTIONS] [PATH]\n       rloc <COMMAND>",
    after_help = "Default command:\n  rloc [SCAN_OPTIONS] [PATH]\n\nExamples:\n  rloc\n  rloc .\n  rloc --format json .\n  rloc detect ."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

impl Cli {
    pub fn parse_args() -> Self {
        <Self as Parser>::parse_from(Self::normalize_args(std::env::args_os()))
    }

    fn normalize_args<I, T>(args: I) -> Vec<OsString>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString>,
    {
        let mut args = args.into_iter().map(Into::into).collect::<Vec<_>>();
        if args.is_empty() {
            return args;
        }

        if Self::should_inject_default_scan(&args[1..]) {
            args.insert(1, OsString::from("scan"));
        }

        args
    }

    fn should_inject_default_scan(args: &[OsString]) -> bool {
        let Some(first) = args.first() else {
            return true;
        };

        !matches!(
            first.to_str(),
            Some("scan" | "detect" | "explain" | "config" | "help")
                | Some("-h" | "--help" | "-V" | "--version")
        )
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Scan a repository and print aggregated metrics.
    Scan(ScanArgs),
    /// Explain how a single file is classified.
    Explain(ExplainArgs),
    /// Detect supported languages and project presets.
    Detect(DetectArgs),
    /// Print effective configuration from the current directory and its parents, or bootstrap a local config.
    Config(ConfigArgs),
}

#[derive(Debug, Clone, Args)]
pub struct ScanArgs {
    #[arg(default_value = ".")]
    pub path: Utf8PathBuf,

    #[arg(long, value_enum)]
    pub format: Option<OutputFormat>,

    #[arg(long = "group-by", value_enum)]
    pub group_by: Vec<GroupBy>,

    /// Show top files; defaults to 10 when passed without a value.
    #[arg(
        long = "top-files",
        num_args = 0..=1,
        default_missing_value = "10",
        conflicts_with = "no_top_files"
    )]
    pub top_files: Option<usize>,

    #[arg(long = "no-top-files")]
    pub no_top_files: bool,

    /// Show top directories; defaults to 10 when passed without a value.
    #[arg(
        long = "top-dirs",
        num_args = 0..=1,
        default_missing_value = "10",
        conflicts_with = "no_top_dirs"
    )]
    pub top_dirs: Option<usize>,

    #[arg(long = "no-top-dirs")]
    pub no_top_dirs: bool,

    /// List example unsupported files; defaults to 5 when passed without a value.
    #[arg(
        long = "list-unsupported",
        num_args = 0..=1,
        default_missing_value = "5"
    )]
    pub list_unsupported: Option<usize>,

    /// Comma-separated language list; aliases: rs, py, js, ts, md, cfg, sh; names: sql, go, html, css.
    #[arg(long = "languages", value_delimiter = ',')]
    pub languages: Vec<LanguageArg>,

    /// Comma-separated exclude globs; may be passed multiple times and are appended to config excludes.
    #[arg(long = "exclude", value_delimiter = ',')]
    pub exclude: Vec<String>,

    #[arg(long = "no-tests")]
    pub no_tests: bool,

    /// Include generated files in the report.
    #[arg(long = "generated")]
    pub generated: bool,

    #[arg(long = "no-vendor")]
    pub no_vendor: bool,

    /// Path to .rloc.toml; when omitted, search the scan path and its parents.
    #[arg(long = "config")]
    pub config: Option<Utf8PathBuf>,

    #[arg(long = "no-gitignore")]
    pub no_gitignore: bool,

    #[arg(long = "hidden")]
    pub hidden: bool,
}

#[derive(Debug, Clone, Args)]
pub struct ExplainArgs {
    pub file: Utf8PathBuf,

    #[arg(long, value_enum, default_value_t = OutputFormat::Table)]
    pub format: OutputFormat,

    /// Path to .rloc.toml; when omitted, search the current directory and its parents.
    #[arg(long = "config")]
    pub config: Option<Utf8PathBuf>,
}

#[derive(Debug, Clone, Args)]
pub struct DetectArgs {
    #[arg(default_value = ".")]
    pub path: Utf8PathBuf,

    /// List example unsupported files; defaults to 5 when passed without a value.
    #[arg(
        long = "list-unsupported",
        num_args = 0..=1,
        default_missing_value = "5"
    )]
    pub list_unsupported: Option<usize>,
}

#[derive(Debug, Clone, Args)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: Option<ConfigCommand>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Subcommand)]
pub enum ConfigCommand {
    /// Create a default .rloc.toml in the current directory.
    Init,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum GroupBy {
    Language,
    Category,
    Dir,
    File,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum LanguageArg {
    #[value(alias("sh"))]
    Shell,
    Sql,
    Go,
    Html,
    Css,
    #[value(alias("rs"))]
    Rust,
    #[value(alias("py"))]
    Python,
    #[value(alias("js"))]
    Javascript,
    #[value(alias("ts"))]
    Typescript,
    #[value(alias("md"))]
    Markdown,
    #[value(alias("cfg"))]
    Config,
    Jsx,
    Tsx,
}

impl From<LanguageArg> for rloc_core::Language {
    fn from(value: LanguageArg) -> Self {
        match value {
            LanguageArg::Shell => Self::Shell,
            LanguageArg::Sql => Self::Sql,
            LanguageArg::Go => Self::Go,
            LanguageArg::Html => Self::Html,
            LanguageArg::Css => Self::Css,
            LanguageArg::Rust => Self::Rust,
            LanguageArg::Python => Self::Python,
            LanguageArg::Javascript => Self::JavaScript,
            LanguageArg::Typescript => Self::TypeScript,
            LanguageArg::Markdown => Self::Markdown,
            LanguageArg::Config => Self::Config,
            LanguageArg::Jsx => Self::Jsx,
            LanguageArg::Tsx => Self::Tsx,
        }
    }
}

#[cfg(test)]
mod tests {
    use camino::Utf8PathBuf;
    use clap::{CommandFactory, Parser};

    use super::{Cli, Command, ConfigCommand};

    #[test]
    fn parses_config_init_subcommand() {
        let cli = Cli::try_parse_from(["rloc", "config", "init"]).unwrap();

        match cli.command {
            Command::Config(args) => {
                assert_eq!(args.command, Some(ConfigCommand::Init));
            }
            other => panic!("expected config command, got {other:?}"),
        }
    }

    #[test]
    fn parses_config_without_subcommand() {
        let cli = Cli::try_parse_from(["rloc", "config"]).unwrap();

        match cli.command {
            Command::Config(args) => {
                assert!(args.command.is_none());
            }
            other => panic!("expected config command, got {other:?}"),
        }
    }

    #[test]
    fn parses_explain_with_config_override() {
        let cli = Cli::try_parse_from([
            "rloc",
            "explain",
            "src/lib.rs",
            "--format",
            "json",
            "--config",
            ".rloc.toml",
        ])
        .unwrap();

        match cli.command {
            Command::Explain(args) => {
                assert_eq!(args.file, Utf8PathBuf::from("src/lib.rs"));
                assert!(matches!(args.format, super::OutputFormat::Json));
                assert_eq!(args.config, Some(Utf8PathBuf::from(".rloc.toml")));
            }
            other => panic!("expected explain command, got {other:?}"),
        }
    }

    #[test]
    fn parses_scan_language_aliases() {
        let cli = Cli::try_parse_from([
            "rloc",
            "scan",
            ".",
            "--languages",
            "js,ts,md,cfg,tsx,sh,sql,go,html,css",
        ])
        .unwrap();

        match cli.command {
            Command::Scan(args) => {
                assert_eq!(args.languages.len(), 10);
                assert!(matches!(args.languages[0], super::LanguageArg::Javascript));
                assert!(matches!(args.languages[1], super::LanguageArg::Typescript));
                assert!(matches!(args.languages[2], super::LanguageArg::Markdown));
                assert!(matches!(args.languages[3], super::LanguageArg::Config));
                assert!(matches!(args.languages[4], super::LanguageArg::Tsx));
                assert!(matches!(args.languages[5], super::LanguageArg::Shell));
                assert!(matches!(args.languages[6], super::LanguageArg::Sql));
                assert!(matches!(args.languages[7], super::LanguageArg::Go));
                assert!(matches!(args.languages[8], super::LanguageArg::Html));
                assert!(matches!(args.languages[9], super::LanguageArg::Css));
            }
            other => panic!("expected scan command, got {other:?}"),
        }
    }

    #[test]
    fn normalizes_default_scan_without_subcommand() {
        let cli = Cli::try_parse_from(Cli::normalize_args(["rloc", "."])).unwrap();

        match cli.command {
            Command::Scan(args) => {
                assert_eq!(args.path, Utf8PathBuf::from("."));
            }
            other => panic!("expected scan command, got {other:?}"),
        }
    }

    #[test]
    fn normalizes_default_scan_with_scan_flags() {
        let cli = Cli::try_parse_from(Cli::normalize_args([
            "rloc",
            "--format",
            "json",
            "--exclude",
            "**/target/**,**/dist/**",
            ".",
        ]))
        .unwrap();

        match cli.command {
            Command::Scan(args) => {
                assert!(matches!(args.format, Some(super::OutputFormat::Json)));
                assert_eq!(args.exclude, vec!["**/target/**", "**/dist/**"]);
                assert_eq!(args.path, Utf8PathBuf::from("."));
            }
            other => panic!("expected scan command, got {other:?}"),
        }
    }

    #[test]
    fn keeps_named_subcommands_unchanged() {
        let cli = Cli::try_parse_from(Cli::normalize_args(["rloc", "detect", "."])).unwrap();

        match cli.command {
            Command::Detect(args) => {
                assert_eq!(args.path, Utf8PathBuf::from("."));
            }
            other => panic!("expected detect command, got {other:?}"),
        }
    }

    #[test]
    fn parses_bare_top_flags_with_default_limit() {
        let cli = Cli::try_parse_from(["rloc", "scan", ".", "--top-files", "--top-dirs"]).unwrap();

        match cli.command {
            Command::Scan(args) => {
                assert_eq!(args.top_files, Some(10));
                assert_eq!(args.top_dirs, Some(10));
            }
            other => panic!("expected scan command, got {other:?}"),
        }
    }

    #[test]
    fn parses_no_top_flags() {
        let cli =
            Cli::try_parse_from(["rloc", "scan", ".", "--no-top-files", "--no-top-dirs"]).unwrap();

        match cli.command {
            Command::Scan(args) => {
                assert!(args.no_top_files);
                assert!(args.no_top_dirs);
            }
            other => panic!("expected scan command, got {other:?}"),
        }
    }

    #[test]
    fn parses_bare_list_unsupported_flags_with_default_limit() {
        let scan = Cli::try_parse_from(["rloc", "scan", ".", "--list-unsupported"]).unwrap();
        let detect = Cli::try_parse_from(["rloc", "detect", ".", "--list-unsupported"]).unwrap();

        match scan.command {
            Command::Scan(args) => {
                assert_eq!(args.list_unsupported, Some(5));
            }
            other => panic!("expected scan command, got {other:?}"),
        }

        match detect.command {
            Command::Detect(args) => {
                assert_eq!(args.list_unsupported, Some(5));
            }
            other => panic!("expected detect command, got {other:?}"),
        }
    }

    #[test]
    fn parses_generated_flag() {
        let cli = Cli::try_parse_from(["rloc", "scan", ".", "--generated"]).unwrap();

        match cli.command {
            Command::Scan(args) => {
                assert!(args.generated);
            }
            other => panic!("expected scan command, got {other:?}"),
        }
    }

    #[test]
    fn scan_help_mentions_language_aliases() {
        let mut command = Cli::command();
        let scan = command.find_subcommand_mut("scan").unwrap();
        let help = scan.render_long_help().to_string();

        assert!(help.contains("aliases: rs, py, js, ts, md, cfg, sh"));
        assert!(help.contains("sql"));
        assert!(help.contains("go"));
        assert!(help.contains("html"));
        assert!(help.contains("css"));
    }

    #[test]
    fn scan_help_mentions_generated_flag() {
        let mut command = Cli::command();
        let scan = command.find_subcommand_mut("scan").unwrap();
        let help = scan.render_long_help().to_string();

        assert!(help.contains("--generated"));
        assert!(!help.contains("--no-generated"));
    }

    #[test]
    fn scan_and_detect_help_mention_list_unsupported_flag() {
        let mut command = Cli::command();
        let scan = command.find_subcommand_mut("scan").unwrap();
        let scan_help = scan.render_long_help().to_string();
        let detect = command.find_subcommand_mut("detect").unwrap();
        let detect_help = detect.render_long_help().to_string();

        assert!(scan_help.contains("--list-unsupported"));
        assert!(detect_help.contains("--list-unsupported"));
    }

    #[test]
    fn scan_help_mentions_default_config_lookup() {
        let mut command = Cli::command();
        let scan = command.find_subcommand_mut("scan").unwrap();
        let help = scan.render_long_help().to_string();

        assert!(help.contains("search the scan path and its parents"));
    }

    #[test]
    fn explain_help_mentions_default_config_lookup() {
        let mut command = Cli::command();
        let explain = command.find_subcommand_mut("explain").unwrap();
        let help = explain.render_long_help().to_string();

        assert!(help.contains("search the current directory and its parents"));
    }

    #[test]
    fn config_help_mentions_ancestor_discovery() {
        let mut command = Cli::command();
        let config = command.find_subcommand_mut("config").unwrap();
        let help = config.render_long_help().to_string();

        assert!(help.contains("current directory and its parents"));
    }

    #[test]
    fn root_help_mentions_default_scan_shorthand() {
        let help = Cli::command().render_long_help().to_string();

        assert!(help.contains("Usage: rloc [SCAN_OPTIONS] [PATH]"));
        assert!(help.contains("Default command:"));
        assert!(help.contains("rloc [SCAN_OPTIONS] [PATH]"));
    }

    #[test]
    fn rejects_removed_include_comments_flag() {
        let error = Cli::try_parse_from(["rloc", "scan", ".", "--include-comments"]).unwrap_err();
        let rendered = error.to_string();

        assert!(rendered.contains("--include-comments"));
        assert!(rendered.contains("unexpected"));
    }
}
