# 中文稿须用 XeLaTeX（pdflatex 无法加载 unisong 等字体）
$pdf_mode = 5;
$bibtex_use = 2;
$ENV{'TEXINPUTS'} = '../docs/latex-models/software-journal//:' . ($ENV{'TEXINPUTS'} // '');
