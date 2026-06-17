//! OMML 序列化（Office MathML 子集）
//!
//! 直接由 `MathExpr` 生成 `<m:oMath>` 字节流，**不**经过 MathML 中间格式，
//! 由 docx-writer 嵌入 `word/document.xml` 中的 `<m:oMath>` 段。

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::Writer;

use crate::expr::MathExpr;

const NS: &[u8] = b"http://schemas.openxmlformats.org/officeDocument/2006/math";

/// 序列化为 OMML 字节流。
pub fn to_omml(expr: &MathExpr) -> Vec<u8> {
    let mut w = Writer::new(Vec::new());
    w.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .unwrap();
    let mut root = BytesStart::new("m:oMath");
    root.push_attribute(("xmlns:m", std::str::from_utf8(NS).unwrap()));
    w.write_event(Event::Start(root)).unwrap();
    write_expr(&mut w, expr);
    w.write_event(Event::End(BytesEnd::new("m:oMath"))).unwrap();
    w.into_inner()
}

fn write_expr(w: &mut Writer<Vec<u8>>, e: &MathExpr) {
    match e {
        MathExpr::Number(s) => {
            w.write_event(Event::Start(BytesStart::new("m:num")))
                .unwrap();
            write_run_text(w, s);
            w.write_event(Event::End(BytesEnd::new("m:num"))).unwrap();
        }
        MathExpr::Ident(s) | MathExpr::Text(s) => {
            write_run_text(w, s);
        }
        MathExpr::Op(c) => {
            // Full OMML: use <m:oSupp> with paired <m:begChr>/<m:endChr>
            let s = c.to_string();
            w.write_event(Event::Start(BytesStart::new("m:oSupp")))
                .unwrap();
            w.write_event(Event::Start(BytesStart::new("m:begChr")))
                .unwrap();
            write_run_text(w, &s);
            w.write_event(Event::End(BytesEnd::new("m:begChr")))
                .unwrap();
            w.write_event(Event::Start(BytesStart::new("m:endChr")))
                .unwrap();
            write_run_text(w, &s);
            w.write_event(Event::End(BytesEnd::new("m:endChr")))
                .unwrap();
            w.write_event(Event::End(BytesEnd::new("m:oSupp"))).unwrap();
        }
        MathExpr::Space => {}
        MathExpr::Sub { base, sub } => {
            w.write_event(Event::Start(BytesStart::new("m:sSub")))
                .unwrap();
            w.write_event(Event::Start(BytesStart::new("m:e"))).unwrap();
            write_expr(w, base);
            w.write_event(Event::End(BytesEnd::new("m:e"))).unwrap();
            w.write_event(Event::Start(BytesStart::new("m:sub")))
                .unwrap();
            write_expr(w, sub);
            w.write_event(Event::End(BytesEnd::new("m:sub"))).unwrap();
            w.write_event(Event::End(BytesEnd::new("m:sSub"))).unwrap();
        }
        MathExpr::Sup { base, sup } => {
            w.write_event(Event::Start(BytesStart::new("m:sSup")))
                .unwrap();
            w.write_event(Event::Start(BytesStart::new("m:e"))).unwrap();
            write_expr(w, base);
            w.write_event(Event::End(BytesEnd::new("m:e"))).unwrap();
            w.write_event(Event::Start(BytesStart::new("m:sup")))
                .unwrap();
            write_expr(w, sup);
            w.write_event(Event::End(BytesEnd::new("m:sup"))).unwrap();
            w.write_event(Event::End(BytesEnd::new("m:sSup"))).unwrap();
        }
        MathExpr::SubSup { base, sub, sup } => {
            w.write_event(Event::Start(BytesStart::new("m:sSubSup")))
                .unwrap();
            w.write_event(Event::Start(BytesStart::new("m:e"))).unwrap();
            write_expr(w, base);
            w.write_event(Event::End(BytesEnd::new("m:e"))).unwrap();
            w.write_event(Event::Start(BytesStart::new("m:sub")))
                .unwrap();
            write_expr(w, sub);
            w.write_event(Event::End(BytesEnd::new("m:sub"))).unwrap();
            w.write_event(Event::Start(BytesStart::new("m:sup")))
                .unwrap();
            write_expr(w, sup);
            w.write_event(Event::End(BytesEnd::new("m:sup"))).unwrap();
            w.write_event(Event::End(BytesEnd::new("m:sSubSup")))
                .unwrap();
        }
        MathExpr::Frac { num, den } => {
            w.write_event(Event::Start(BytesStart::new("m:f"))).unwrap();
            w.write_event(Event::Start(BytesStart::new("m:num")))
                .unwrap();
            write_expr(w, num);
            w.write_event(Event::End(BytesEnd::new("m:num"))).unwrap();
            w.write_event(Event::Start(BytesStart::new("m:den")))
                .unwrap();
            write_expr(w, den);
            w.write_event(Event::End(BytesEnd::new("m:den"))).unwrap();
            w.write_event(Event::End(BytesEnd::new("m:f"))).unwrap();
        }
        MathExpr::Sqrt { body, index } => {
            if let Some(idx) = index {
                w.write_event(Event::Start(BytesStart::new("m:rad")))
                    .unwrap();
                w.write_event(Event::Start(BytesStart::new("m:deg")))
                    .unwrap();
                write_expr(w, idx);
                w.write_event(Event::End(BytesEnd::new("m:deg"))).unwrap();
                w.write_event(Event::Start(BytesStart::new("m:e"))).unwrap();
                write_expr(w, body);
                w.write_event(Event::End(BytesEnd::new("m:e"))).unwrap();
                w.write_event(Event::End(BytesEnd::new("m:rad"))).unwrap();
            } else {
                w.write_event(Event::Start(BytesStart::new("m:rad")))
                    .unwrap();
                w.write_event(Event::Start(BytesStart::new("m:degHide")))
                    .unwrap();
                w.write_event(Event::End(BytesEnd::new("m:degHide")))
                    .unwrap();
                w.write_event(Event::Start(BytesStart::new("m:e"))).unwrap();
                write_expr(w, body);
                w.write_event(Event::End(BytesEnd::new("m:e"))).unwrap();
                w.write_event(Event::End(BytesEnd::new("m:rad"))).unwrap();
            }
        }
        MathExpr::Fenced { open, body, close } => {
            w.write_event(Event::Start(BytesStart::new("m:d"))).unwrap();
            w.write_event(Event::Start(BytesStart::new("m:begChr")))
                .unwrap();
            write_run_text(w, open);
            w.write_event(Event::End(BytesEnd::new("m:begChr")))
                .unwrap();
            w.write_event(Event::Start(BytesStart::new("m:e"))).unwrap();
            write_expr(w, body);
            w.write_event(Event::End(BytesEnd::new("m:e"))).unwrap();
            w.write_event(Event::Start(BytesStart::new("m:endChr")))
                .unwrap();
            write_run_text(w, close);
            w.write_event(Event::End(BytesEnd::new("m:endChr")))
                .unwrap();
            w.write_event(Event::End(BytesEnd::new("m:d"))).unwrap();
        }
        MathExpr::Function { name, arg } => {
            w.write_event(Event::Start(BytesStart::new("m:func")))
                .unwrap();
            w.write_event(Event::Start(BytesStart::new("m:fName")))
                .unwrap();
            write_run_text(w, name);
            w.write_event(Event::End(BytesEnd::new("m:fName"))).unwrap();
            w.write_event(Event::Start(BytesStart::new("m:e"))).unwrap();
            write_expr(w, arg);
            w.write_event(Event::End(BytesEnd::new("m:e"))).unwrap();
            w.write_event(Event::End(BytesEnd::new("m:func"))).unwrap();
        }
        MathExpr::Matrix { rows } => {
            w.write_event(Event::Start(BytesStart::new("m:m"))).unwrap();
            for row in rows {
                w.write_event(Event::Start(BytesStart::new("m:mr")))
                    .unwrap();
                for cell in row {
                    w.write_event(Event::Start(BytesStart::new("m:e"))).unwrap();
                    write_expr(w, cell);
                    w.write_event(Event::End(BytesEnd::new("m:e"))).unwrap();
                }
                w.write_event(Event::End(BytesEnd::new("m:mr"))).unwrap();
            }
            w.write_event(Event::End(BytesEnd::new("m:m"))).unwrap();
        }
        MathExpr::Seq(seq) => {
            w.write_event(Event::Start(BytesStart::new("m:r"))).unwrap();
            for e in seq {
                write_expr(w, e);
            }
            w.write_event(Event::End(BytesEnd::new("m:r"))).unwrap();
        }
        MathExpr::Raw(s) => {
            write_run_text(w, s);
        }
    }
}

fn write_run_text(w: &mut Writer<Vec<u8>>, s: &str) {
    w.write_event(Event::Start(BytesStart::new("m:r"))).unwrap();
    w.write_event(Event::Start(BytesStart::new("m:t"))).unwrap();
    w.write_event(Event::Text(quick_xml::events::BytesText::new(s)))
        .unwrap();
    w.write_event(Event::End(BytesEnd::new("m:t"))).unwrap();
    w.write_event(Event::End(BytesEnd::new("m:r"))).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::latex::parse_latex_math;

    #[test]
    fn omml_basic() {
        let s = to_omml(&parse_latex_math("E = mc^2"));
        let s = String::from_utf8_lossy(&s);
        assert!(s.contains("<m:oMath"));
        assert!(s.contains("<m:sSup"));
        assert!(s.contains("<m:t>E</m:t>"));
    }

    #[test]
    fn omml_frac() {
        let s = to_omml(&parse_latex_math(r"\frac{1}{2}"));
        let s = String::from_utf8_lossy(&s);
        assert!(s.contains("<m:f>"));
        assert!(s.contains("<m:num>"));
        assert!(s.contains("<m:den>"));
    }

    #[test]
    fn omml_matrix() {
        let s = to_omml(&parse_latex_math(
            r"\begin{matrix} a & b \\ c & d \end{matrix}",
        ));
        let s = String::from_utf8_lossy(&s);
        assert!(s.contains("<m:m>"));
        assert!(s.contains("<m:mr>"));
    }

    #[test]
    #[allow(non_snake_case)]
    fn omml_op_uses_oSupp() {
        let s = to_omml(&parse_latex_math("x + y"));
        let s = String::from_utf8_lossy(&s);
        assert!(s.contains("<m:oSupp"));
        assert!(s.contains("<m:begChr"));
        assert!(s.contains("<m:endChr>"));
    }

    #[test]
    fn omml_seq_uses_r() {
        // Sequence with operators should wrap in <m:r>
        let s = to_omml(&parse_latex_math("a + b"));
        let s = String::from_utf8_lossy(&s);
        assert!(s.contains("<m:oMath"));
        // Should have <m:r> wrappers
        assert!(s.contains("<m:r>"));
    }
}
