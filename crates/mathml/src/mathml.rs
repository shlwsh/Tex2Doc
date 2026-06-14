//! MathML 序列化（Presentation MathML 子集）
//!
//! 输出 `<math xmlns="http://www.w3.org/1998/Math/MathML">...</math>` 字节流。

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::Writer;

use crate::expr::MathExpr;

/// 序列化为 MathML 字节流（UTF-8）。
pub fn to_mathml(expr: &MathExpr) -> Vec<u8> {
    let mut w = Writer::new(Vec::new());
    w.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .unwrap();
    let mut math = BytesStart::new("math");
    math.push_attribute(("xmlns", "http://www.w3.org/1998/Math/MathML"));
    w.write_event(Event::Start(math)).unwrap();
    write_expr(&mut w, expr);
    w.write_event(Event::End(BytesEnd::new("math"))).unwrap();
    w.into_inner()
}

fn write_expr(w: &mut Writer<Vec<u8>>, e: &MathExpr) {
    match e {
        MathExpr::Number(s) | MathExpr::Ident(s) | MathExpr::Text(s) => {
            let tag = if matches!(e, MathExpr::Number(_)) {
                "mn"
            } else if matches!(e, MathExpr::Text(_)) {
                "mtext"
            } else {
                "mi"
            };
            w.write_event(Event::Start(BytesStart::new(tag))).unwrap();
            w.write_event(Event::Text(quick_xml::events::BytesText::new(s)))
                .unwrap();
            w.write_event(Event::End(BytesEnd::new(tag))).unwrap();
        }
        MathExpr::Op(c) => {
            let s = c.to_string();
            w.write_event(Event::Start(BytesStart::new("mo"))).unwrap();
            w.write_event(Event::Text(quick_xml::events::BytesText::new(&s)))
                .unwrap();
            w.write_event(Event::End(BytesEnd::new("mo"))).unwrap();
        }
        MathExpr::Space => {
            w.write_event(Event::Empty(BytesStart::new("mspace")))
                .unwrap();
        }
        MathExpr::Sub { base, sub } => {
            w.write_event(Event::Start(BytesStart::new("msub")))
                .unwrap();
            write_expr(w, base);
            write_expr(w, sub);
            w.write_event(Event::End(BytesEnd::new("msub"))).unwrap();
        }
        MathExpr::Sup { base, sup } => {
            w.write_event(Event::Start(BytesStart::new("msup")))
                .unwrap();
            write_expr(w, base);
            write_expr(w, sup);
            w.write_event(Event::End(BytesEnd::new("msup"))).unwrap();
        }
        MathExpr::SubSup { base, sub, sup } => {
            w.write_event(Event::Start(BytesStart::new("msubsup")))
                .unwrap();
            write_expr(w, base);
            write_expr(w, sub);
            write_expr(w, sup);
            w.write_event(Event::End(BytesEnd::new("msubsup"))).unwrap();
        }
        MathExpr::Frac { num, den } => {
            w.write_event(Event::Start(BytesStart::new("mfrac")))
                .unwrap();
            write_expr(w, num);
            write_expr(w, den);
            w.write_event(Event::End(BytesEnd::new("mfrac"))).unwrap();
        }
        MathExpr::Sqrt { body, index } => {
            if let Some(idx) = index {
                w.write_event(Event::Start(BytesStart::new("mroot")))
                    .unwrap();
                write_expr(w, body);
                write_expr(w, idx);
                w.write_event(Event::End(BytesEnd::new("mroot"))).unwrap();
            } else {
                w.write_event(Event::Start(BytesStart::new("msqrt")))
                    .unwrap();
                write_expr(w, body);
                w.write_event(Event::End(BytesEnd::new("msqrt"))).unwrap();
            }
        }
        MathExpr::Fenced { open, body, close } => {
            w.write_event(Event::Start(BytesStart::new("mrow")))
                .unwrap();
            w.write_event(Event::Start(BytesStart::new("mo"))).unwrap();
            w.write_event(Event::Text(quick_xml::events::BytesText::new(open)))
                .unwrap();
            w.write_event(Event::End(BytesEnd::new("mo"))).unwrap();
            write_expr(w, body);
            w.write_event(Event::Start(BytesStart::new("mo"))).unwrap();
            w.write_event(Event::Text(quick_xml::events::BytesText::new(close)))
                .unwrap();
            w.write_event(Event::End(BytesEnd::new("mo"))).unwrap();
            w.write_event(Event::End(BytesEnd::new("mrow"))).unwrap();
        }
        MathExpr::Function { name, arg } => {
            w.write_event(Event::Start(BytesStart::new("mrow")))
                .unwrap();
            w.write_event(Event::Start(BytesStart::new("mi"))).unwrap();
            w.write_event(Event::Text(quick_xml::events::BytesText::new(name)))
                .unwrap();
            w.write_event(Event::End(BytesEnd::new("mi"))).unwrap();
            // 简化为 mi( arg )
            w.write_event(Event::Start(BytesStart::new("mo"))).unwrap();
            w.write_event(Event::Text(quick_xml::events::BytesText::new("(")))
                .unwrap();
            w.write_event(Event::End(BytesEnd::new("mo"))).unwrap();
            write_expr(w, arg);
            w.write_event(Event::Start(BytesStart::new("mo"))).unwrap();
            w.write_event(Event::Text(quick_xml::events::BytesText::new(")")))
                .unwrap();
            w.write_event(Event::End(BytesEnd::new("mo"))).unwrap();
            w.write_event(Event::End(BytesEnd::new("mrow"))).unwrap();
        }
        MathExpr::Matrix { rows } => {
            w.write_event(Event::Start(BytesStart::new("mtable")))
                .unwrap();
            for row in rows {
                w.write_event(Event::Start(BytesStart::new("mtr"))).unwrap();
                for cell in row {
                    w.write_event(Event::Start(BytesStart::new("mtd"))).unwrap();
                    write_expr(w, cell);
                    w.write_event(Event::End(BytesEnd::new("mtd"))).unwrap();
                }
                w.write_event(Event::End(BytesEnd::new("mtr"))).unwrap();
            }
            w.write_event(Event::End(BytesEnd::new("mtable"))).unwrap();
        }
        MathExpr::Seq(seq) => {
            // 序列：如果只有一个元素，扁平化
            if seq.len() == 1 {
                write_expr(w, &seq[0]);
            } else {
                w.write_event(Event::Start(BytesStart::new("mrow")))
                    .unwrap();
                for e in seq {
                    write_expr(w, e);
                }
                w.write_event(Event::End(BytesEnd::new("mrow"))).unwrap();
            }
        }
        MathExpr::Raw(s) => {
            w.write_event(Event::Start(BytesStart::new("mtext")))
                .unwrap();
            w.write_event(Event::Text(quick_xml::events::BytesText::new(s)))
                .unwrap();
            w.write_event(Event::End(BytesEnd::new("mtext"))).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::latex::parse_latex_math;

    #[test]
    fn mathml_contains_key_tags() {
        let s = to_mathml(&parse_latex_math(r"x^{2}"));
        let s = String::from_utf8_lossy(&s);
        assert!(s.contains("<math"));
        assert!(s.contains("<msup"));
        assert!(s.contains("<mi>x</mi>"));
        assert!(s.contains("<mn>2</mn>"));
    }

    #[test]
    fn mathml_frac() {
        let s = to_mathml(&parse_latex_math(r"\frac{1}{2}"));
        let s = String::from_utf8_lossy(&s);
        assert!(s.contains("<mfrac"));
    }

    #[test]
    fn mathml_matrix() {
        let s = to_mathml(&parse_latex_math(
            r"\begin{matrix} a & b \\ c & d \end{matrix}",
        ));
        let s = String::from_utf8_lossy(&s);
        assert!(s.contains("<mtable"));
        assert!(s.contains("<mtr"));
        assert!(s.contains("<mtd"));
    }
}
