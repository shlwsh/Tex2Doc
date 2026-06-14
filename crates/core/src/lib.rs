//! Doc-engine 统一对外门面

#![forbid(unsafe_code)]

pub mod convert;
pub mod error;
pub mod options;
pub mod result;

pub use convert::{convert_dir, convert_stream, convert_sync};
pub use error::CoreError;
pub use options::{Attachment, BibStyle, ConvertOptions};
pub use result::{ConvertResult, ProgressEvent, ProgressPhase};
