use rloc_core::{
    BackendFileAnalysis, ClassificationOptions, FileCategory, Language, LanguageBackend,
    LanguageDescriptor, Utf8Path,
};

pub mod classify;

#[derive(Debug, Clone, Copy, Default)]
pub struct HtmlBackend;

#[derive(Debug, Clone, Copy, Default)]
pub struct CssBackend;

pub const HTML_DESCRIPTOR: LanguageDescriptor =
    LanguageDescriptor::new(Language::Html, "HTML", &["html", "htm", "xhtml", "gohtml"]);
pub const CSS_DESCRIPTOR: LanguageDescriptor =
    LanguageDescriptor::new(Language::Css, "CSS", &["css"]);

impl LanguageBackend for HtmlBackend {
    fn descriptor(&self) -> LanguageDescriptor {
        html_descriptor()
    }

    fn classify_file(
        &self,
        path: &Utf8Path,
        category: FileCategory,
        options: &ClassificationOptions,
    ) -> Result<BackendFileAnalysis, String> {
        classify::classify_file(path, Language::Html, category, options)
    }
}

impl LanguageBackend for CssBackend {
    fn descriptor(&self) -> LanguageDescriptor {
        css_descriptor()
    }

    fn classify_file(
        &self,
        path: &Utf8Path,
        category: FileCategory,
        options: &ClassificationOptions,
    ) -> Result<BackendFileAnalysis, String> {
        classify::classify_file(path, Language::Css, category, options)
    }
}

pub fn html_descriptor() -> LanguageDescriptor {
    HTML_DESCRIPTOR
}

pub fn css_descriptor() -> LanguageDescriptor {
    CSS_DESCRIPTOR
}

pub fn html_backend() -> HtmlBackend {
    HtmlBackend
}

pub fn css_backend() -> CssBackend {
    CssBackend
}
