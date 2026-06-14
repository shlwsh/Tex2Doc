//! Rowan 语法树节点类型定义
//!
//! 注意：`rowan::SyntaxKind` 是 `pub struct SyntaxKind(pub u16)`。
//! 我们用业务枚举 [`SyntaxKind`]，需要写入时调用 [`raw`] 转成 `rowan::SyntaxKind`。

use rowan::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SyntaxKind {
    // 容器
    Root,
    Group,
    Env,

    // 叶子
    Command,
    Text,
    Whitespace,
    Comment,
    MathInline,
    MathDisplay,
    Begin,
    End,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Error,

    // 终结符 / 占位
    TokNewline,
}

impl SyntaxKind {
    /// 转成 `rowan::SyntaxKind`（`u16` newtype）。
    pub const fn to_raw(self) -> rowan::SyntaxKind {
        rowan::SyntaxKind(self as u16)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Lang {}
impl Language for Lang {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        match raw.0 {
            x if x == SyntaxKind::Root as u16 => SyntaxKind::Root,
            x if x == SyntaxKind::Group as u16 => SyntaxKind::Group,
            x if x == SyntaxKind::Env as u16 => SyntaxKind::Env,
            x if x == SyntaxKind::Command as u16 => SyntaxKind::Command,
            x if x == SyntaxKind::Text as u16 => SyntaxKind::Text,
            x if x == SyntaxKind::Whitespace as u16 => SyntaxKind::Whitespace,
            x if x == SyntaxKind::Comment as u16 => SyntaxKind::Comment,
            x if x == SyntaxKind::MathInline as u16 => SyntaxKind::MathInline,
            x if x == SyntaxKind::MathDisplay as u16 => SyntaxKind::MathDisplay,
            x if x == SyntaxKind::Begin as u16 => SyntaxKind::Begin,
            x if x == SyntaxKind::End as u16 => SyntaxKind::End,
            x if x == SyntaxKind::LBrace as u16 => SyntaxKind::LBrace,
            x if x == SyntaxKind::RBrace as u16 => SyntaxKind::RBrace,
            x if x == SyntaxKind::LBracket as u16 => SyntaxKind::LBracket,
            x if x == SyntaxKind::RBracket as u16 => SyntaxKind::RBracket,
            x if x == SyntaxKind::Error as u16 => SyntaxKind::Error,
            x if x == SyntaxKind::TokNewline as u16 => SyntaxKind::TokNewline,
            _ => SyntaxKind::Error,
        }
    }

    fn kind_to_raw(kind: SyntaxKind) -> rowan::SyntaxKind {
        kind.to_raw()
    }
}

pub type SyntaxNode = rowan::SyntaxNode<Lang>;
pub type SyntaxToken = rowan::SyntaxToken<Lang>;
pub type SyntaxElement = rowan::SyntaxElement<Lang>;
pub type GreenNode = rowan::GreenNode;
