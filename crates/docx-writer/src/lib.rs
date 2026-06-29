//! Doc-engine OOXML 序列化层
//!
//! 极简 M1：仅输出 `Heading` + `Paragraph` 的 docx（带基础 styles.xml）。

#![forbid(unsafe_code)]

pub mod model;
pub mod packer;
pub mod page_setup;
pub mod profile;
pub mod serializer;
pub mod styles;
pub mod template;
pub mod validate;

pub use packer::{pack, pack_with_assets, pack_with_page_setup, pack_with_template};
pub use page_setup::PageSetup;
pub use profile::{CjkOptions, ProfileStyleMap, StyleCoverageReport};
pub use serializer::{serialize_document, EmbeddedImage};
pub use template::{merge_styles, parse_styles_xml, parse_template, TemplateStyles};
pub use validate::{OoxmlValidator, SchemaViolation};
