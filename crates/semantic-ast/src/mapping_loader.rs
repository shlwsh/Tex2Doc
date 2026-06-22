//! Mapping Registry YAML 加载器（v12）
//!
//! 用于把 `standards/mappings/*.yaml` 声明式规则加载为 [`MappingRegistry`]。
//! YAML 字段对应 [`MappingRule`] 的五元组：`id`、`source_kind`、`target_kind`、
//! `style_from_profile`、`rule_type`。YAML 是单一事实源；`for_profile()`
//! 仍保留为兜底硬编码，YAML 解析失败或字段缺失时回退到硬编码默认。

use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::docx_render::{MappingRegistry, MappingRule};

#[derive(Debug, Error)]
pub enum MappingLoadError {
    #[error("YAML 解析失败: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("YAML 中缺少必要字段 `rules`")]
    MissingRules,
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),
}

/// YAML 文件顶层 schema。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MappingFile {
    pub schema_version: String,
    pub target_format: String,
    pub rules: Vec<MappingFileRule>,
}

/// YAML 中单条规则的 schema。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MappingFileRule {
    pub id: String,
    /// 源 AST 节点类型（如 `Heading`、`List`）。
    /// 当 `rule_type=inline` 时此字段也可用 `source_inline_kind` 表达。
    #[serde(default)]
    pub source_ast_kind: Option<String>,
    #[serde(default)]
    pub source_inline_kind: Option<String>,
    /// DOCX 目标节点（如 `Paragraph`、`Run`、`Drawing`）。
    pub target_render_node: String,
    /// 由 profile 解析得到的样式 ID（如 `body`、`reference`）。
    #[serde(default)]
    pub style_from_profile: Option<String>,
    /// 关系规则 ID（如 `opc.relationship.image`）。
    #[serde(default)]
    pub relationship_rule: Option<String>,
    /// run 级属性（如 superscript）。
    #[serde(default)]
    pub run_properties: Option<MappingRunProperties>,
    /// 校验规则。
    #[serde(default)]
    pub validation: Option<MappingValidation>,
    /// `block` / `inline` / `resource`，缺省按 `source_ast_kind` 是否为 None 推断。
    #[serde(default)]
    pub rule_type: Option<String>,
    /// 规则版本，便于长期演进。
    #[serde(default = "default_version")]
    pub version: String,
}

fn default_version() -> String {
    "0.1".to_string()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MappingRunProperties {
    #[serde(default)]
    pub vertical_align: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MappingValidation {
    #[serde(default)]
    pub required_text: bool,
    #[serde(default)]
    pub require_media: bool,
    #[serde(default)]
    pub require_caption: bool,
    #[serde(default)]
    pub require_line_numbers: bool,
}

impl MappingFileRule {
    /// 转换为运行期 `MappingRule`。
    pub fn to_rule(&self) -> MappingRule {
        let source_kind = self
            .source_ast_kind
            .clone()
            .or_else(|| self.source_inline_kind.clone())
            .unwrap_or_default();
        let rule_type = self.rule_type.clone().unwrap_or_else(|| {
            if self.source_inline_kind.is_some() {
                "inline".to_string()
            } else if self.relationship_rule.is_some() {
                "resource".to_string()
            } else {
                "block".to_string()
            }
        });
        let style_from_profile = self.style_from_profile.clone();
        MappingRule {
            id: self.id.clone(),
            source_kind,
            target_kind: self.target_render_node.clone(),
            style_from_profile,
            rule_type,
        }
    }
}

impl MappingRegistry {
    /// 从 YAML 文本加载。YAML 中既可包含 block 规则也可包含 inline/resource 规则，
    /// 加载器按 `rule_type` 字段自动分桶。
    pub fn from_yaml(profile_id: impl Into<String>, yaml: &str) -> Result<Self, MappingLoadError> {
        let file: MappingFile = serde_yaml::from_str(yaml)?;
        Self::from_mapping_file(profile_id, &file)
    }

    /// 从 YAML 文件路径加载。
    pub fn from_yaml_path(
        profile_id: impl Into<String>,
        path: impl AsRef<Path>,
    ) -> Result<Self, MappingLoadError> {
        let text = std::fs::read_to_string(path)?;
        Self::from_yaml(profile_id, &text)
    }

    /// 从已解析的 [`MappingFile`] 构造。
    pub fn from_mapping_file(
        profile_id: impl Into<String>,
        file: &MappingFile,
    ) -> Result<Self, MappingLoadError> {
        let mut block_mappings = Vec::new();
        let mut inline_mappings = Vec::new();
        let mut resource_mappings = Vec::new();
        for rule in &file.rules {
            let mr = rule.to_rule();
            match mr.rule_type.as_str() {
                "inline" => inline_mappings.push(mr),
                "resource" => resource_mappings.push(mr),
                _ => block_mappings.push(mr),
            }
        }
        Ok(Self {
            profile_id: profile_id.into(),
            block_mappings,
            inline_mappings,
            resource_mappings,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const JOS_RULES_YAML: &str = r#"
schema_version: "0.1"
target_format: docx
rules:
  - id: map.heading.docx
    source_ast_kind: Heading
    target_render_node: Paragraph
    style_from_profile: heading_by_level
  - id: map.paragraph.docx
    source_ast_kind: Paragraph
    target_render_node: Paragraph
    style_from_profile: body
  - id: map.list.docx
    source_ast_kind: List
    target_render_node: ParagraphList
    style_from_profile: body
  - id: map.citation.docx
    source_inline_kind: Citation
    target_render_node: Run
    run_properties:
      vertical_align: superscript
  - id: opc.relationship.image
    target_render_node: Relationship
    relationship_rule: opc.relationship.image
"#;

    #[test]
    fn loads_jos_profile_yaml() {
        let reg = MappingRegistry::from_yaml("jos-2025", JOS_RULES_YAML).expect("load ok");
        assert_eq!(reg.profile_id, "jos-2025");
        assert!(reg
            .block_mappings
            .iter()
            .any(|r| r.id == "map.heading.docx"));
        assert!(reg.block_mappings.iter().any(|r| r.id == "map.list.docx"));
        assert!(reg
            .inline_mappings
            .iter()
            .any(|r| r.id == "map.citation.docx"));
        assert!(reg
            .resource_mappings
            .iter()
            .any(|r| r.id == "opc.relationship.image"));
    }

    #[test]
    fn rule_type_inferred() {
        let file: MappingFile = serde_yaml::from_str(JOS_RULES_YAML).unwrap();
        let citation = file
            .rules
            .iter()
            .find(|r| r.id == "map.citation.docx")
            .unwrap();
        let mr = citation.to_rule();
        assert_eq!(mr.rule_type, "inline");
    }

    #[test]
    fn invalid_yaml_errors() {
        let bad = "schema_version: 0.1\nrules: not_a_list";
        let err = MappingRegistry::from_yaml("x", bad);
        assert!(err.is_err());
    }

    #[test]
    fn for_profile_still_works_as_fallback() {
        let reg = MappingRegistry::for_profile("jos-2025");
        assert!(reg
            .block_mappings
            .iter()
            .any(|r| r.id == "map.heading.docx"));
    }
}
