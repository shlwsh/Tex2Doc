//! 端到端 smoke：包含公式 / 列表 / 表格 / 图片 / Bib 的 hello.docx

use doc_core::{convert_sync, ConvertOptions};
use std::io::Write;

const SAMPLE_TEX: &str = r#"
\title{Demo}
\section{Heading}

A paragraph with \textbf{bold} and \textit{italic} runs.

\begin{itemize}
\item First item
\item Second item
\end{itemize}

\begin{tabular}{c|c}
A & B \\
C & D \\
\end{tabular}

\begin{equation}
E = mc^2
\end{equation}

\begin{equation}
\int_{0}^{1} x^2 \, dx = \frac{1}{3}
\end{equation}

\begin{figure}
\includegraphics[width=.7\textwidth]{a.png}
\caption{Demo figure}
\end{figure}
"#;

#[test]
fn end_to_end_full() {
    let opts = ConvertOptions::default();
    let result = convert_sync("sample.tex", SAMPLE_TEX, &opts).expect("convert");
    assert!(!result.docx.is_empty());
    assert_eq!(&result.docx[..4], b"PK\x03\x04");

    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("output");
    std::fs::create_dir_all(&out_dir).unwrap();
    let out = out_dir.join("sample.docx");
    let mut f = std::fs::File::create(&out).unwrap();
    f.write_all(&result.docx).unwrap();
    eprintln!("wrote {}", out.display());
}
