//! 公式简化 AST

#[derive(Debug, Clone, PartialEq)]
pub enum MathExpr {
    /// 字面量
    Number(String),
    /// 标识符（按 italic 渲染）
    Ident(String),
    /// 文本（`\text{...}`）
    Text(String),
    /// 运算符：+ - * / = < >
    Op(char),
    /// 空格
    Space,
    /// 上下标
    Sub {
        base: Box<MathExpr>,
        sub: Box<MathExpr>,
    },
    Sup {
        base: Box<MathExpr>,
        sup: Box<MathExpr>,
    },
    SubSup {
        base: Box<MathExpr>,
        sub: Box<MathExpr>,
        sup: Box<MathExpr>,
    },
    /// 分式
    Frac {
        num: Box<MathExpr>,
        den: Box<MathExpr>,
    },
    /// 根式
    Sqrt {
        body: Box<MathExpr>,
        index: Option<Box<MathExpr>>,
    },
    /// 括号包裹
    Fenced {
        open: String,
        body: Box<MathExpr>,
        close: String,
    },
    /// 函数应用 `\sin` `\cos` ...
    Function {
        name: String,
        arg: Box<MathExpr>,
    },
    /// 行内矩阵
    Matrix {
        rows: Vec<Vec<MathExpr>>,
    },
    /// 错误降级：原文
    Raw(String),
    /// 序列
    Seq(Vec<MathExpr>),
}

impl MathExpr {
    /// 折叠空 Seq
    pub fn flatten(seq: Vec<MathExpr>) -> MathExpr {
        if seq.len() == 1 {
            seq.into_iter().next().unwrap()
        } else {
            MathExpr::Seq(seq)
        }
    }
}

/// 测试辅助：把 `Box<MathExpr>` 解出后剥开单元素 `Seq` 包装
impl MathExpr {
    pub fn unwrap_seq(self) -> MathExpr {
        if let MathExpr::Seq(s) = &self {
            if s.len() == 1 {
                return s[0].clone();
            }
        }
        self
    }
}
