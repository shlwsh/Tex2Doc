//! 核心层：统一转换入口
//!
//! 端到端管道：
//! 1. include 拓扑（`IncludeGraph::build`）
//! 2. 拼接单流（`IncludeGraph::join`）
//! 3. Logos + Rowan 解析（`LatexParser`）
//! 4. 降级到 `semantic-ast`
//! 5. docx-writer 序列化 + ZIP 打包

use doc_latex_reader::{lower_to_document, parse_tex, IncludeGraph};
use doc_semantic_ast::Document;
use doc_utils::{DocError, DocResult, VirtualFs};

use crate::error::CoreError;
use crate::options::ConvertOptions;
use crate::result::{ConvertResult, ProgressEvent, ProgressPhase};

/// 同步转换入口（V1 M1-M2）。
pub fn convert_sync(
    main_tex: &str,
    source: &str,
    _options: &ConvertOptions,
) -> Result<ConvertResult, CoreError> {
    let doc = parse_tex_to_doc(main_tex, source)?;
    let docx = doc_docx_writer::pack(&doc).map_err(|e| CoreError::Serialize(e.0))?;
    Ok(ConvertResult {
        docx,
        warnings: vec![],
    })
}

/// 进度流入口（占位，M5-M6 落地为真流）。
pub async fn convert_stream(
    main_tex: &str,
    source: &str,
    options: &ConvertOptions,
) -> Result<ConvertResult, CoreError> {
    for phase in [
        ProgressPhase::Reading,
        ProgressPhase::Parsing,
        ProgressPhase::Lowering,
        ProgressPhase::Serializing,
        ProgressPhase::Packing,
    ] {
        let _ = ProgressEvent {
            phase,
            ratio: 0.0,
            message: format!("{:?}", phase),
        };
    }
    convert_sync(main_tex, source, options)
}

/// 内部：tex 源 → Document。
pub(crate) fn parse_tex_to_doc(main_tex: &str, source: &str) -> Result<Document, CoreError> {
    let mut vfs = VirtualFs::new();
    vfs.insert(main_tex, source.as_bytes().to_vec());
    let graph = IncludeGraph::build(&vfs, std::path::Path::new(main_tex))?;
    let joined = graph.join(&vfs)?;
    let parse = parse_tex(&joined.text);
    Ok(lower_to_document(&parse, Some(&joined)))
}

pub(crate) fn to_doc_err(e: DocError) -> CoreError {
    e.into()
}
