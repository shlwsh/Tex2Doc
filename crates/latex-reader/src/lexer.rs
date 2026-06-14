//! Logos 词法
//!
//! V1 词法元素：
//! - 控制序列：`\` 后接字母
//! - 分组符：`{` `}` `[` `]`
//! - 数学定界符：`$` `$$`
//! - 注释：`%` 至行尾
//! - 空白 / 换行（保留至 SyntaxKind::Whitespace / TokNewline）
//! - 其它单字符 / 文本

pub use logos::Logos;

use crate::green::SyntaxKind;

#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokKind {
    #[regex(r"\\[A-Za-z@]+")]
    Command,

    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,

    #[token("$")]
    Dollar,
    #[token("$$")]
    DollarDollar,

    #[regex(r"%[^\n]*")]
    Comment,

    #[regex(r"[ \t]+")]
    Whitespace,
    #[regex(r"\r?\n")]
    Newline,
    #[regex(r"\\\\")]
    LineBreak,
    #[token(r"\par")]
    Par,

    // 兜底：其它任意 UTF-8 字符
    Error,
}

impl TokKind {
    /// 映射到 [`crate::green::SyntaxKind`]。
    pub fn into_syntax(self) -> SyntaxKind {
        match self {
            TokKind::Command => SyntaxKind::Command,
            TokKind::LBrace => SyntaxKind::LBrace,
            TokKind::RBrace => SyntaxKind::RBrace,
            TokKind::LBracket => SyntaxKind::LBracket,
            TokKind::RBracket => SyntaxKind::RBracket,
            TokKind::Comment => SyntaxKind::Comment,
            TokKind::Whitespace => SyntaxKind::Whitespace,
            TokKind::Newline => SyntaxKind::TokNewline,
            TokKind::LineBreak => SyntaxKind::TokNewline,
            TokKind::Par => SyntaxKind::TokNewline,
            TokKind::Dollar | TokKind::DollarDollar => SyntaxKind::MathInline,
            TokKind::Error => SyntaxKind::Error,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_section() {
        let kinds: Vec<_> = TokKind::lexer("\\section{Hi}")
            .spanned()
            .map(|(t, _)| t.unwrap_or(TokKind::Error))
            .collect();
        assert_eq!(kinds[0], TokKind::Command);
        assert!(kinds.contains(&TokKind::LBrace));
        assert!(kinds.contains(&TokKind::RBrace));
    }

    #[test]
    fn lex_brace_pair() {
        let kinds: Vec<_> = TokKind::lexer("{}")
            .spanned()
            .map(|(t, _)| t.unwrap_or(TokKind::Error))
            .collect();
        assert_eq!(kinds, vec![TokKind::LBrace, TokKind::RBrace]);
    }

    #[test]
    fn lex_comment() {
        let kinds: Vec<_> = TokKind::lexer("a%bb\nc")
            .spanned()
            .map(|(t, _)| t.unwrap_or(TokKind::Error))
            .collect();
        assert!(kinds.contains(&TokKind::Comment));
    }
}
