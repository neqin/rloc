use rloc_core::{
    BackendFileAnalysis, ClassificationOptions, FileCategory, Language, LanguageBackend,
    LanguageDescriptor, Utf8Path,
};

pub mod classify;

#[derive(Debug, Clone, Copy, Default)]
pub struct PowerShellBackend;

pub const DESCRIPTOR: LanguageDescriptor =
    LanguageDescriptor::new(Language::PowerShell, "PowerShell", &["ps1"]);

impl LanguageBackend for PowerShellBackend {
    fn descriptor(&self) -> LanguageDescriptor {
        DESCRIPTOR
    }

    fn classify_file(
        &self,
        path: &Utf8Path,
        category: FileCategory,
        options: &ClassificationOptions,
    ) -> Result<BackendFileAnalysis, String> {
        classify::classify_file(path, category, options)
    }
}

pub fn backend() -> PowerShellBackend {
    PowerShellBackend
}
