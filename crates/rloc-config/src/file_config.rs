use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigFile {
    #[serde(default)]
    pub scan: ScanConfig,
    #[serde(default)]
    pub filters: FiltersConfig,
    #[serde(default)]
    pub classification: ClassificationConfig,
    #[serde(default)]
    pub report: ReportConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanConfig {
    #[serde(default = "default_true")]
    pub respect_gitignore: bool,
    #[serde(default)]
    pub hidden: bool,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            respect_gitignore: true,
            hidden: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FiltersConfig {
    #[serde(default)]
    pub exclude: Vec<String>,
    #[serde(default = "default_true")]
    pub include_tests: bool,
    #[serde(default)]
    pub include_generated: bool,
    #[serde(default)]
    pub include_vendor: bool,
    #[serde(default)]
    pub generated_patterns: Vec<String>,
    #[serde(default)]
    pub vendor_patterns: Vec<String>,
}

impl Default for FiltersConfig {
    fn default() -> Self {
        Self {
            exclude: Vec::new(),
            include_tests: true,
            include_generated: false,
            include_vendor: false,
            generated_patterns: Vec::new(),
            vendor_patterns: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassificationConfig {
    #[serde(default = "default_true")]
    pub count_doc_comments: bool,
    #[serde(default = "default_true")]
    pub count_docstrings_as_comments: bool,
    #[serde(default = "default_true")]
    pub mixed_lines_as_code: bool,
}

impl Default for ClassificationConfig {
    fn default() -> Self {
        Self {
            count_doc_comments: true,
            count_docstrings_as_comments: true,
            mixed_lines_as_code: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportConfig {
    #[serde(default)]
    pub format: ReportFormat,
    #[serde(default = "default_group_by")]
    pub group_by: Vec<ReportGroupBy>,
    #[serde(default = "default_top_limit", skip_serializing_if = "Option::is_none")]
    pub top_files: Option<usize>,
    #[serde(default = "default_top_limit", skip_serializing_if = "Option::is_none")]
    pub top_dirs: Option<usize>,
}

impl Default for ReportConfig {
    fn default() -> Self {
        Self {
            format: ReportFormat::default(),
            group_by: default_group_by(),
            top_files: default_top_limit(),
            top_dirs: default_top_limit(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportFormat {
    #[default]
    Table,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportGroupBy {
    Language,
    Category,
    Dir,
    File,
}

fn default_true() -> bool {
    true
}

fn default_group_by() -> Vec<ReportGroupBy> {
    vec![ReportGroupBy::Language]
}

fn default_top_limit() -> Option<usize> {
    Some(10)
}

#[cfg(test)]
mod tests {
    use super::{FiltersConfig, ReportConfig};

    #[test]
    fn filters_default_excludes_generated_files() {
        let filters = FiltersConfig::default();

        assert!(!filters.include_generated);
    }

    #[test]
    fn report_defaults_enable_top_sections() {
        let report = ReportConfig::default();

        assert_eq!(report.top_files, Some(10));
        assert_eq!(report.top_dirs, Some(10));
    }
}
