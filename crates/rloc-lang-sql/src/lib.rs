use rloc_core::{
    BackendFileAnalysis, ClassificationOptions, FileCategory, Language, LanguageBackend,
    LanguageDescriptor, Utf8Path,
};

pub mod classify;

#[derive(Debug, Clone, Copy, Default)]
pub struct SqlBackend;

pub const DESCRIPTORS: [LanguageDescriptor; 1] = [LanguageDescriptor::new(
    Language::Sql,
    "SQL",
    &["sql", "psql"],
)];

impl LanguageBackend for SqlBackend {
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

pub fn backend() -> SqlBackend {
    SqlBackend
}
