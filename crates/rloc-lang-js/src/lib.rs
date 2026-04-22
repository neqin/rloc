use rloc_core::{
    BackendFileAnalysis, ClassificationOptions, FileCategory, Language, LanguageBackend,
    LanguageDescriptor, Utf8Path,
};

pub mod classify;
pub mod parser;
pub use parser::parser_name;

#[derive(Debug, Clone, Copy)]
pub struct JsFamilyBackend {
    descriptor: LanguageDescriptor,
}

pub const DESCRIPTORS: [LanguageDescriptor; 4] = [
    LanguageDescriptor::new(Language::JavaScript, "JavaScript", &["js"]),
    LanguageDescriptor::new(Language::TypeScript, "TypeScript", &["ts"]),
    LanguageDescriptor::new(Language::Jsx, "JSX", &["jsx"]),
    LanguageDescriptor::new(Language::Tsx, "TSX", &["tsx"]),
];

impl LanguageBackend for JsFamilyBackend {
    fn descriptor(&self) -> LanguageDescriptor {
        self.descriptor
    }

    fn classify_file(
        &self,
        path: &Utf8Path,
        category: FileCategory,
        options: &ClassificationOptions,
    ) -> Result<BackendFileAnalysis, String> {
        classify::classify_file(path, self.descriptor.language, category, options)
    }
}

pub fn descriptors() -> [LanguageDescriptor; 4] {
    DESCRIPTORS
}

pub fn javascript_backend() -> JsFamilyBackend {
    JsFamilyBackend {
        descriptor: DESCRIPTORS[0],
    }
}

pub fn typescript_backend() -> JsFamilyBackend {
    JsFamilyBackend {
        descriptor: DESCRIPTORS[1],
    }
}

pub fn jsx_backend() -> JsFamilyBackend {
    JsFamilyBackend {
        descriptor: DESCRIPTORS[2],
    }
}

pub fn tsx_backend() -> JsFamilyBackend {
    JsFamilyBackend {
        descriptor: DESCRIPTORS[3],
    }
}
