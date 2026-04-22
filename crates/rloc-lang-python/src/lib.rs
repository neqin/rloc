use rloc_core::{
    BackendFileAnalysis, ClassificationOptions, FileCategory, Language, LanguageBackend,
    LanguageDescriptor, Utf8Path,
};

pub mod classify;
pub mod parser;
pub use parser::parser_name;

#[derive(Debug, Clone, Copy, Default)]
pub struct PythonBackend;

pub const DESCRIPTORS: [LanguageDescriptor; 1] =
    [LanguageDescriptor::new(Language::Python, "Python", &["py"])];

impl LanguageBackend for PythonBackend {
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

pub fn backend() -> PythonBackend {
    PythonBackend
}
