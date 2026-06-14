//! Rowan 语法树构建（Pass-2）
//!
//! V1 策略：朴素「无文法硬编码」 —— 直接将 Lexer 出的 token 序列
//! 包装为 `Root { token* }`，仅按规则做：
//! - `\begin{name} ... \end{name}` 配对 → `Env`
//! - `{ ... }` → `Group`
//! - 其它 token 平铺到 Root
//!
//! 错误恢复：未闭合的 Group / Env 自动补一个虚拟闭合（不报错、不丢 token）。

use rowan::GreenNodeBuilder;

use crate::green::{GreenNode, SyntaxKind as S, SyntaxNode};
use crate::lexer::{Logos, TokKind};

/// 解析后的句法树根。
#[derive(Debug, Clone)]
pub struct Parse {
    pub green: GreenNode,
    pub root: SyntaxNode,
    /// 原始输入文本（保留所有字符，供 Lowering 使用）
    pub source: String,
}

/// 极简解析。
pub fn parse(text: &str) -> Parse {
    let mut b = GreenNodeBuilder::new();
    b.start_node(S::Root.to_raw());
    parse_into(text, &mut b);
    b.finish_node();
    let green = b.finish();
    let root = SyntaxNode::new_root(green.clone());
    Parse {
        green,
        root,
        source: text.to_string(),
    }
}

fn parse_into(text: &str, b: &mut GreenNodeBuilder<'static>) {
    use TokKind as T;
    let mut brace_depth: i32 = 0;
    let mut env_depth: i32 = 0;
    let mut env_stack: Vec<u16> = Vec::new();

    for (tok, span) in T::lexer(text).spanned() {
        let tok = match tok {
            Ok(t) => t,
            Err(_) => T::Error,
        };
        let slice = &text[span.start..span.end];
        match tok {
            T::LBrace => {
                b.start_node(S::Group.to_raw());
                brace_depth += 1;
            }
            T::RBrace => {
                if brace_depth > 0 {
                    b.finish_node();
                    brace_depth -= 1;
                } else {
                    b.start_node(S::Error.to_raw());
                    b.token(S::RBrace.to_raw(), slice);
                    b.finish_node();
                }
            }
            T::Command => {
                // 极简 Begin/End 探测（V1）：见 Command 文本是否以 \begin / \end 开头
                if slice.starts_with("\\begin") {
                    b.start_node(S::Env.to_raw());
                    env_stack.push(S::Env as u16);
                    env_depth += 1;
                    b.token(S::Begin.to_raw(), slice);
                } else if slice.starts_with("\\end") {
                    if env_stack.pop().is_some() {
                        b.finish_node();
                        env_depth -= 1;
                    } else {
                        b.start_node(S::Error.to_raw());
                        b.token(S::End.to_raw(), slice);
                        b.finish_node();
                    }
                } else {
                    b.token(S::Command.to_raw(), slice);
                }
            }
            T::Whitespace => b.token(S::Whitespace.to_raw(), slice),
            T::Newline | T::LineBreak | T::Par => b.token(S::TokNewline.to_raw(), slice),
            T::Comment => b.token(S::Comment.to_raw(), slice),
            T::LBracket => b.token(S::LBracket.to_raw(), slice),
            T::RBracket => b.token(S::RBracket.to_raw(), slice),
            T::Dollar | T::DollarDollar => b.token(S::MathInline.to_raw(), slice),
            T::Error => b.token(S::Error.to_raw(), slice),
        }
    }
    // 收尾：未闭合自动闭合（V1 容错：绝不 panic）
    while brace_depth > 0 {
        b.finish_node();
        brace_depth -= 1;
    }
    while let Some(_) = env_stack.pop() {
        b.finish_node();
    }
    let _ = env_depth;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_braces() {
        let p = parse("{a}");
        let txt = p.root.text().to_string();
        // 大括号作为 group 容器不计入叶子文本
        assert_eq!(txt, "a");
    }

    #[test]
    fn parse_unbalanced_recovers() {
        let p = parse("{a");
        let txt = p.root.text().to_string();
        assert_eq!(txt, "a");
    }

    #[test]
    fn parse_extra_rbrace_recovers() {
        let p = parse("}a}");
        // 至少能产出字符串，不 panic
        assert!(!p.root.text().to_string().is_empty());
    }
}
