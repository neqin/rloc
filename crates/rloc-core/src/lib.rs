pub use camino::{Utf8Path, Utf8PathBuf};

pub mod analyze;
pub mod categories;
pub mod discover;
pub mod filters;
pub mod metrics;
pub mod registry;
pub mod text_backend;
pub mod types;

pub use analyze::{Analyzer, DetectReport, ExplainReport};
pub use metrics::{FileMetrics, MetricsSummary, ScanReport};
pub use registry::{LanguageBackendRegistry, LanguageDescriptor};
pub use text_backend::{config_backend, markdown_backend};
pub use types::{
    AnalysisWarning, BackendFileAnalysis, ClassificationOptions, FileCategory, Language,
    LanguageBackend, LineExplanation, ScanOptions,
};
