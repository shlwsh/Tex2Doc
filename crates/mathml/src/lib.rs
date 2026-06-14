//! 公式管道（V1 简化版）
//!
//! ## 阶段
//!
//! 1. `parse_latex_math` 把 LaTeX 源码解析成 [`MathExpr`] 简化 AST。
//! 2. `to_mathml` 把 MathExpr 序列化成 MathML（Presentation MathML 子集）。
//! 3. `to_omml` 把 MathML 序列化成 Office MathML（`<m:oMath>`），由 docx-writer 嵌入 `document.xml`。
//!
//! ## 支持语法（V1）
//!
//! - 数字、字母标识符（隐式 italic）
//! - 二元运算符：`+ - * / = < >`
//! - 上下标：`x^{...}` / `x_{...}`
//! - 分式：`\frac{a}{b}`
//! - 根式：`\sqrt{...}` / `\sqrt[n]{...}`
//! - 括号：`\left( ... \right)`
//! - 三角函数：`\sin` `\cos` `\tan`（直译为函数应用）
//! - 希腊字母：`\alpha` ... `\omega`（V1 子集）
//! - 矩阵：`\begin{matrix} ... \end{matrix}`（V1 占位：仅 \\\\ & 解析）
//!
//! 其余语法降级为 mtext。

#![forbid(unsafe_code)]

pub mod latex;
pub mod mathml;
pub mod omml;
pub mod expr;

pub use expr::MathExpr;
pub use latex::parse_latex_math;
pub use mathml::to_mathml;
pub use omml::to_omml;
