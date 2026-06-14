//! Doc-engine 通用工具库
//!
//! ## 模块
//!
//! - [`vfs`]：内存虚拟文件系统，支持真实目录挂载。
//! - [`path`]：LaTeX include / graphicspath 路径解算。
//! - [`image`]：图片格式探测与重压缩。
//! - [`fontmap`]：CTeX 字体 → Office 字体映射。
//! - [`error`]：统一错误类型。

#![forbid(unsafe_code)]
#![deny(rust_2018_idioms)]

pub mod error;
pub mod fontmap;
pub mod image;
pub mod path;
pub mod vfs;

pub use error::{DocError, DocResult};
pub use fontmap::{default_map, FontMap, OfficeFont};
pub use image::{read_meta, renormalize, ImageMeta, SupportedFormat};
pub use path::{parse_graphics_path, PathResolver};
pub use vfs::VirtualFs;
