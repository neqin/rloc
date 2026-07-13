use rloc_core::{
    BackendFileAnalysis, ClassificationOptions, FileCategory, Language, LanguageBackend,
    LanguageDescriptor, Utf8Path,
};

pub mod classify;

pub const C_DESCRIPTOR: LanguageDescriptor = LanguageDescriptor::new(Language::C, "C", &["c", "h"]);
pub const CPP_DESCRIPTOR: LanguageDescriptor =
    LanguageDescriptor::new(Language::Cpp, "C++", &["cpp"]);
pub const JAVA_DESCRIPTOR: LanguageDescriptor =
    LanguageDescriptor::new(Language::Java, "Java", &["java"]);
pub const SWIFT_DESCRIPTOR: LanguageDescriptor =
    LanguageDescriptor::new(Language::Swift, "Swift", &["swift"]);
pub const OBJECTIVE_C_DESCRIPTOR: LanguageDescriptor =
    LanguageDescriptor::new(Language::ObjectiveC, "Objective-C", &["m"]);
pub const ZIG_DESCRIPTOR: LanguageDescriptor =
    LanguageDescriptor::new(Language::Zig, "Zig", &["zig"]);

pub const DESCRIPTORS: [LanguageDescriptor; 6] = [
    C_DESCRIPTOR,
    CPP_DESCRIPTOR,
    JAVA_DESCRIPTOR,
    SWIFT_DESCRIPTOR,
    OBJECTIVE_C_DESCRIPTOR,
    ZIG_DESCRIPTOR,
];

#[derive(Debug, Clone, Copy, Default)]
pub struct CBackend;

#[derive(Debug, Clone, Copy, Default)]
pub struct CppBackend;

#[derive(Debug, Clone, Copy, Default)]
pub struct JavaBackend;

#[derive(Debug, Clone, Copy, Default)]
pub struct SwiftBackend;

#[derive(Debug, Clone, Copy, Default)]
pub struct ObjectiveCBackend;

#[derive(Debug, Clone, Copy, Default)]
pub struct ZigBackend;

macro_rules! impl_backend {
    ($backend:ty, $descriptor:ident) => {
        impl LanguageBackend for $backend {
            fn descriptor(&self) -> LanguageDescriptor {
                $descriptor
            }

            fn classify_file(
                &self,
                path: &Utf8Path,
                category: FileCategory,
                options: &ClassificationOptions,
            ) -> Result<BackendFileAnalysis, String> {
                classify::classify_file(path, $descriptor.language, category, options)
            }
        }
    };
}

impl_backend!(CBackend, C_DESCRIPTOR);
impl_backend!(CppBackend, CPP_DESCRIPTOR);
impl_backend!(JavaBackend, JAVA_DESCRIPTOR);
impl_backend!(SwiftBackend, SWIFT_DESCRIPTOR);
impl_backend!(ObjectiveCBackend, OBJECTIVE_C_DESCRIPTOR);
impl_backend!(ZigBackend, ZIG_DESCRIPTOR);

pub fn descriptors() -> [LanguageDescriptor; 6] {
    DESCRIPTORS
}

pub fn c_backend() -> CBackend {
    CBackend
}

pub fn cpp_backend() -> CppBackend {
    CppBackend
}

pub fn java_backend() -> JavaBackend {
    JavaBackend
}

pub fn swift_backend() -> SwiftBackend {
    SwiftBackend
}

pub fn objective_c_backend() -> ObjectiveCBackend {
    ObjectiveCBackend
}

pub fn zig_backend() -> ZigBackend {
    ZigBackend
}
