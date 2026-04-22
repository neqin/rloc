use rloc_core::{
    BackendFileAnalysis, ClassificationOptions, FileCategory, Language, LanguageBackend,
    LanguageDescriptor, Utf8Path,
};

pub mod classify;

#[derive(Debug, Clone, Copy, Default)]
pub struct GoBackend;

pub const DESCRIPTORS: [LanguageDescriptor; 1] =
    [LanguageDescriptor::new(Language::Go, "Go", &["go"])];

impl LanguageBackend for GoBackend {
    fn descriptor(&self) -> LanguageDescriptor {
        descriptor()
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

pub fn descriptor() -> LanguageDescriptor {
    DESCRIPTORS[0]
}

pub fn descriptors() -> [LanguageDescriptor; 1] {
    DESCRIPTORS
}

pub fn backend() -> GoBackend {
    GoBackend
}
