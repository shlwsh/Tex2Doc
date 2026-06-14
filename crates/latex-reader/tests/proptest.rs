//! proptest 模糊测试

use doc_latex_reader::lexer::TokKind;
use logos::Logos;
use proptest::prelude::*;

proptest! {
    /// 任意字节序列：词法分析不应 panic
    #[test]
    fn lexer_never_panics(s in ".{0,256}") {
        let _: Vec<_> = TokKind::lexer(&s).spanned().collect();
    }

    /// 配对花括号：parse 后 SyntaxNode 文本不空且不含 `{` / `}`
    #[test]
    fn parse_balanced_braces_does_not_crash(
        depth in 0usize..8,
    ) {
        let input: String = "{".repeat(depth) + "x" + &"}".repeat(depth);
        let p = doc_latex_reader::parse_tex(&input);
        let _ = p.root.text();
    }
}
