//! LaTeX 数学子集解析

use crate::expr::MathExpr;

/// 解析 LaTeX 公式源码（不含定界符 `$/$$`），返回 [`MathExpr::Seq`]。
///
/// 错误降级：遇到未知语法时累积为 [`MathExpr::Raw`]，不 panic。
pub fn parse_latex_math(input: &str) -> MathExpr {
    let mut p = Parser { s: input, i: 0 };
    let seq = p.parse_seq(false);
    p.skip_ws();
    if p.i < p.s.len() {
        // 剩余部分视为 Raw
        let rest = p.s[p.i..].to_string();
        let mut out = seq;
        out.push(MathExpr::Raw(rest));
        MathExpr::flatten(out)
    } else {
        MathExpr::flatten(seq)
    }
}

struct Parser<'a> {
    s: &'a str,
    i: usize,
}

impl<'a> Parser<'a> {
    fn skip_ws(&mut self) {
        while self.i < self.s.len() {
            let c = self.s.as_bytes()[self.i];
            if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' {
                self.i += 1;
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.s.as_bytes().get(self.i).copied()
    }

    fn eat(&mut self, c: u8) -> bool {
        self.skip_ws();
        if self.peek() == Some(c) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn starts_with(&self, prefix: &str) -> bool {
        self.s[self.i..].starts_with(prefix)
    }

    fn consume(&mut self, n: usize) {
        self.i = (self.i + n).min(self.s.len());
    }

    fn parse_seq(&mut self, stop_brace: bool) -> Vec<MathExpr> {
        let mut out = Vec::new();
        loop {
            self.skip_ws();
            if self.i >= self.s.len() {
                break;
            }
            let c = self.peek().unwrap();
            if c == b'}' && stop_brace {
                break;
            }
            if c == b']' {
                // 闭合可选参数 [...]: 留给 caller 处理
                break;
            }
            if c == b'&' || c == b'\\' && self.s[self.i..].starts_with("\\\\") {
                // 矩阵行分隔：在矩阵上下文外视为结束
                break;
            }
            // 单反斜杠可能是 \cmd
            if c == b'\\' {
                if let Some(e) = self.parse_command() {
                    out.push(e);
                    continue;
                } else {
                    // 未知命令：吞一字符
                    self.i += 1;
                    continue;
                }
            }
            if c == b'^' || c == b'_' {
                // 上下标修饰
                if let Some(last) = out.pop() {
                    let cur = self.peek().unwrap();
                    self.i += 1;
                    let sub = if c == b'_' {
                        self.parse_group_or_single()
                    } else {
                        MathExpr::Seq(Vec::new())
                    };
                    let sup = if cur == b'^' {
                        // ^ 已经在 i-1 处理；其实：先把 ^ 处理，看 _ 紧接
                        MathExpr::Seq(Vec::new())
                    } else {
                        MathExpr::Seq(Vec::new())
                    };
                    // 真正处理：c 是当前 ^ 或 _，要继续读另一符号
                    if c == b'^' {
                        // 期望 _ 紧接
                        if self.peek() == Some(b'_') {
                            self.i += 1;
                            let sub_arg = self.parse_group_or_single();
                            out.push(MathExpr::SubSup {
                                base: Box::new(last),
                                sub: Box::new(sub_arg),
                                sup: Box::new(self.parse_group_or_single()),
                            });
                        } else {
                            out.push(MathExpr::Sup {
                                base: Box::new(last),
                                sup: Box::new(self.parse_group_or_single()),
                            });
                        }
                    } else if self.peek() == Some(b'^') {
                        self.i += 1;
                        let sup_arg = self.parse_group_or_single();
                        out.push(MathExpr::SubSup {
                            base: Box::new(last),
                            sub: Box::new(sub),
                            sup: Box::new(sup_arg),
                        });
                    } else {
                        out.push(MathExpr::Sub {
                            base: Box::new(last),
                            sub: Box::new(sub),
                        });
                    }
                    continue;
                }
            }
            if c == b'{' {
                // 进入组
                self.i += 1;
                let inner = self.parse_seq(true);
                self.skip_ws();
                if self.peek() == Some(b'}') {
                    self.i += 1;
                }
                out.push(MathExpr::Seq(inner));
                continue;
            }
            if c == b'}' {
                // 孤立的 `}`：不消费，作为 Raw 跳过
                self.i += 1;
                out.push(MathExpr::Raw("}".into()));
                continue;
            }
            if c == b'(' || c == b')' || c == b'[' || c == b']' {
                self.i += 1;
                out.push(MathExpr::Op(c as char));
                continue;
            }
            if c == b'+'
                || c == b'-'
                || c == b'*'
                || c == b'/'
                || c == b'='
                || c == b'<'
                || c == b'>'
            {
                self.i += 1;
                out.push(MathExpr::Op(c as char));
                continue;
            }
            if c.is_ascii_digit() {
                let start = self.i;
                while let Some(&b) = self.s.as_bytes().get(self.i) {
                    if b.is_ascii_digit() || b == b'.' {
                        self.i += 1;
                    } else {
                        break;
                    }
                }
                out.push(MathExpr::Number(self.s[start..self.i].to_string()));
                continue;
            }
            if c.is_ascii_alphabetic() {
                let start = self.i;
                while let Some(&b) = self.s.as_bytes().get(self.i) {
                    if b.is_ascii_alphanumeric() {
                        self.i += 1;
                    } else {
                        break;
                    }
                }
                out.push(MathExpr::Ident(self.s[start..self.i].to_string()));
                continue;
            }
            // 其它字符：累积到 Raw
            self.i += 1;
            out.push(MathExpr::Raw((c as char).to_string()));
        }
        out
    }

    fn parse_group_or_single(&mut self) -> MathExpr {
        self.skip_ws();
        if self.peek() == Some(b'{') {
            self.i += 1;
            let inner = self.parse_seq(true);
            self.skip_ws();
            if self.peek() == Some(b'}') {
                self.i += 1;
            }
            return MathExpr::flatten(inner);
        }
        // 单 token
        let saved = self.i;
        let seq = self.parse_seq(false);
        if seq.is_empty() {
            self.i = saved + 1;
            return MathExpr::Raw(String::new());
        }
        if seq.len() == 1 {
            seq.into_iter().next().unwrap()
        } else {
            MathExpr::Seq(seq)
        }
    }

    fn parse_command(&mut self) -> Option<MathExpr> {
        // 已知命令：\frac, \sqrt, \left, \right, \text, \sin, \cos, \tan, \log, \ln, \exp,
        //          \alpha ... \omega
        let cmds = [
            "frac", "sqrt", "left", "right", "text", "sin", "cos", "tan", "log", "ln", "exp",
            "alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta", "iota", "kappa",
            "lambda", "mu", "nu", "xi", "pi", "rho", "sigma", "tau", "phi", "chi", "psi", "omega",
            "Gamma", "Delta", "Theta", "Lambda", "Xi", "Pi", "Sigma", "Phi", "Psi", "Omega",
            "cdot", "times", "div", "pm", "mp", "leq", "geq", "neq", "approx", "infty", "sum",
            "int", "prod",
        ];
        for &cmd in &cmds {
            if self.starts_with(&format!("\\{cmd}")) {
                let len = 1 + cmd.len();
                self.consume(len);
                return Some(self.lower_command(cmd));
            }
        }
        // \begin{matrix} ... \end{matrix}：V1 简化版
        if self.starts_with("\\begin") {
            self.consume("\\begin".len());
            self.skip_ws();
            if self.peek() == Some(b'{') {
                self.i += 1;
                let start = self.i;
                while self.i < self.s.len() && self.s.as_bytes()[self.i] != b'}' {
                    self.i += 1;
                }
                let name = &self.s[start..self.i];
                if self.peek() == Some(b'}') {
                    self.i += 1;
                }
                if name == "matrix" || name == "pmatrix" || name == "bmatrix" {
                    let rows = self.parse_matrix_rows();
                    return Some(MathExpr::Matrix { rows });
                }
            }
        }
        if self.starts_with("\\end") {
            // 简单跳过
            self.consume(self.s.len() - self.i);
        }
        // 未知命令：作为 Raw 吞掉
        self.i += 1;
        Some(MathExpr::Raw("\\?".into()))
    }

    fn parse_matrix_rows(&mut self) -> Vec<Vec<MathExpr>> {
        let mut rows = Vec::new();
        let mut current: Vec<MathExpr> = Vec::new();
        loop {
            self.skip_ws();
            if self.i >= self.s.len() {
                break;
            }
            if self.starts_with("\\\\") {
                self.consume(2);
                rows.push(std::mem::take(&mut current));
                continue;
            }
            if self.starts_with("\\end") {
                rows.push(std::mem::take(&mut current));
                break;
            }
            if self.peek() == Some(b'&') {
                self.i += 1;
                rows.push(std::mem::take(&mut current));
                continue;
            }
            let seq = self.parse_seq(false);
            current.extend(seq);
        }
        if !current.is_empty() {
            rows.push(current);
        }
        rows
    }

    fn lower_command(&mut self, cmd: &str) -> MathExpr {
        match cmd {
            "frac" => {
                let num = self.parse_group_or_single();
                let den = self.parse_group_or_single();
                MathExpr::Frac {
                    num: Box::new(num),
                    den: Box::new(den),
                }
            }
            "sqrt" => {
                self.skip_ws();
                if self.peek() == Some(b'[') {
                    self.i += 1;
                    let idx = self.parse_seq(false);
                    self.skip_ws();
                    if self.peek() == Some(b']') {
                        self.i += 1;
                    }
                    let body = self.parse_group_or_single();
                    MathExpr::Sqrt {
                        body: Box::new(body),
                        index: Some(Box::new(MathExpr::flatten(idx))),
                    }
                } else {
                    let body = self.parse_group_or_single();
                    MathExpr::Sqrt {
                        body: Box::new(body),
                        index: None,
                    }
                }
            }
            "left" => {
                self.skip_ws();
                let open = if let Some(c) = self.peek() {
                    let s = (c as char).to_string();
                    self.i += 1;
                    s
                } else {
                    "(".to_string()
                };
                let body = MathExpr::flatten(self.parse_seq(false));
                let close = if self.starts_with("\\right") {
                    self.consume("\\right".len());
                    self.skip_ws();
                    if let Some(c) = self.peek() {
                        self.i += 1;
                        (c as char).to_string()
                    } else {
                        ")".to_string()
                    }
                } else {
                    ")".to_string()
                };
                MathExpr::Fenced {
                    open,
                    body: Box::new(body),
                    close,
                }
            }
            "right" => {
                // 通常由 \left 吃掉；这里吞一字符
                self.skip_ws();
                if let Some(c) = self.peek() {
                    self.i += 1;
                    return MathExpr::Raw((c as char).to_string());
                }
                MathExpr::Raw(String::new())
            }
            "text" => {
                let inner = self.parse_group_or_single();
                if let MathExpr::Seq(seq) = inner {
                    let text: String = seq
                        .into_iter()
                        .map(|e| match e {
                            MathExpr::Ident(s) | MathExpr::Text(s) | MathExpr::Number(s) => s,
                            _ => String::new(),
                        })
                        .collect();
                    MathExpr::Text(text)
                } else {
                    MathExpr::Text(String::new())
                }
            }
            "sin" | "cos" | "tan" | "log" | "ln" | "exp" => {
                let arg = self.parse_group_or_single();
                MathExpr::Function {
                    name: cmd.to_string(),
                    arg: Box::new(arg),
                }
            }
            "alpha" => MathExpr::Ident("α".into()),
            "beta" => MathExpr::Ident("β".into()),
            "gamma" => MathExpr::Ident("γ".into()),
            "delta" => MathExpr::Ident("δ".into()),
            "epsilon" => MathExpr::Ident("ε".into()),
            "zeta" => MathExpr::Ident("ζ".into()),
            "eta" => MathExpr::Ident("η".into()),
            "theta" => MathExpr::Ident("θ".into()),
            "iota" => MathExpr::Ident("ι".into()),
            "kappa" => MathExpr::Ident("κ".into()),
            "lambda" => MathExpr::Ident("λ".into()),
            "mu" => MathExpr::Ident("μ".into()),
            "nu" => MathExpr::Ident("ν".into()),
            "xi" => MathExpr::Ident("ξ".into()),
            "pi" => MathExpr::Ident("π".into()),
            "rho" => MathExpr::Ident("ρ".into()),
            "sigma" => MathExpr::Ident("σ".into()),
            "tau" => MathExpr::Ident("τ".into()),
            "phi" => MathExpr::Ident("φ".into()),
            "chi" => MathExpr::Ident("χ".into()),
            "psi" => MathExpr::Ident("ψ".into()),
            "omega" => MathExpr::Ident("ω".into()),
            "Gamma" => MathExpr::Ident("Γ".into()),
            "Delta" => MathExpr::Ident("Δ".into()),
            "Theta" => MathExpr::Ident("Θ".into()),
            "Lambda" => MathExpr::Ident("Λ".into()),
            "Xi" => MathExpr::Ident("Ξ".into()),
            "Pi" => MathExpr::Ident("Π".into()),
            "Sigma" => MathExpr::Ident("Σ".into()),
            "Phi" => MathExpr::Ident("Φ".into()),
            "Psi" => MathExpr::Ident("Ψ".into()),
            "Omega" => MathExpr::Ident("Ω".into()),
            "cdot" => MathExpr::Op('·'),
            "times" => MathExpr::Op('×'),
            "div" => MathExpr::Op('÷'),
            "pm" => MathExpr::Op('±'),
            "mp" => MathExpr::Op('∓'),
            "leq" => MathExpr::Op('≤'),
            "geq" => MathExpr::Op('≥'),
            "neq" => MathExpr::Op('≠'),
            "approx" => MathExpr::Op('≈'),
            "infty" => MathExpr::Ident("∞".into()),
            "sum" => MathExpr::Ident("∑".into()),
            "int" => MathExpr::Ident("∫".into()),
            "prod" => MathExpr::Ident("∏".into()),
            _ => MathExpr::Raw(format!("\\{cmd}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_number_and_ident() {
        let e = parse_latex_math("E = mc^2");
        match e {
            MathExpr::Seq(s) => {
                assert!(matches!(s[0], MathExpr::Ident(ref x) if x == "E"));
                let has_eq = s.iter().any(|x| matches!(x, MathExpr::Op('=')));
                assert!(has_eq, "expected Op('=') in {:?}", s);
                // 必含 Sup 且 sup 是 Number("2")
                let sup2 = s.iter().find_map(|x| match x {
                    MathExpr::Sup { sup, .. } => match sup.as_ref() {
                        MathExpr::Number(n) => Some(n.clone()),
                        _ => None,
                    },
                    _ => None,
                });
                assert_eq!(sup2.as_deref(), Some("2"));
            }
            _ => panic!("expected seq"),
        }
    }

    #[test]
    fn parses_frac() {
        let e = parse_latex_math(r"\frac{1}{2}");
        assert!(matches!(e, MathExpr::Frac { .. }));
    }

    #[test]
    fn parses_sqrt_with_index() {
        let e = parse_latex_math(r"\sqrt[3]{x}");
        if let MathExpr::Sqrt {
            index: Some(_),
            body,
            ..
        } = e
        {
            assert!(matches!(*body, MathExpr::Ident(_)));
        } else {
            panic!("expected Sqrt with index");
        }
    }

    #[test]
    fn parses_greek_letters() {
        let e = parse_latex_math(r"\alpha + \beta");
        if let MathExpr::Seq(s) = e {
            assert!(matches!(s[0], MathExpr::Ident(ref x) if x == "α"));
        } else {
            panic!("expected seq");
        }
    }

    #[test]
    fn parses_fenced() {
        let e = parse_latex_math(r"\left( x \right)");
        assert!(matches!(e, MathExpr::Fenced { .. }));
    }

    #[test]
    fn recovers_on_unknown() {
        let _ = parse_latex_math(r"\foobar x");
    }
}
