\NeedsTeXFormat{LaTeX2e}
\ProvidesClass{rjthesis}[2025/06/12 Software journal paper (2025 template)]


%%%%%%%%%%%%%%%%%%%%%%%%%加载类
\LoadClass[twoside, UTF8]{ctexart}  % twoside可以设计奇偶不一样的页眉
\linespread{1.0}  % 覆盖 ctexart 默认的 1.3 倍行距，改为单倍行距

%%%%%%%%%%%%%%%%%%%%%%%%%加载包
\RequirePackage{geometry}           % 定制页边距
% 2025 Word模板精确参数: 页面 18.40×26.00cm
% 边距 L=1.45 R=1.45 T=1.00 B=2.20cm  页眉距 1.30cm
% 首页 masthead 是 Word 正文段落，不作为 first-page header 处理。
\geometry{
  paperwidth=18.40cm, paperheight=26.00cm,
  left=1.45cm, right=1.45cm,
  top=1.00cm, bottom=2.20cm,
  headheight=13pt,
  headsep=0.3cm,
  footskip=1.0cm
}
\RequirePackage{fancyhdr}           % 设置页眉页脚
\RequirePackage{xeCJK}              % 引入中文支持
\RequirePackage{CJKnumb}            % 引入中文支持
\RequirePackage{titlesec}           % 设置各级标题的样式
\RequirePackage{ifthen}             % 用来进行判断的宏包
\RequirePackage{enumitem}           % 设置enumerate/itemize等的item距离
\RequirePackage{booktabs}           % 设置可以使用三线表
\RequirePackage{amsmath}            % 设置可以引用数学公式
\RequirePackage[font=small,labelsep=space,skip=3pt]{caption}  % 控制图表标题与图表间距


%%%%%%%%%%%%%%%%%%%%%%%%%设置字体
\newcommand{\song}{\songti}    	% 宋体（xeCJK 支持）
\newcommand{\fs}{\fangsong}       % 仿宋体
\newcommand{\kai}{\kaishu}      	% 楷体
\newcommand{\hei}{\heiti}      	% 黑体
\newcommand{\li}{\lishu}       % 隶书



%%%%%%%%%%%%%%%%%%%%%%%%%定义字号 (baselineskip ≈ 1.2×字号)
\newcommand{\chuhao}{\fontsize{42pt}{50pt}\selectfont}
\newcommand{\xiaochuhao}{\fontsize{36pt}{44pt}\selectfont}
\newcommand{\yihao}{\fontsize{28pt}{34pt}\selectfont}
\newcommand{\erhao}{\fontsize{21pt}{25pt}\selectfont}
\newcommand{\xiaoerhao}{\fontsize{18pt}{22pt}\selectfont}
\newcommand{\sanhao}{\fontsize{15.75pt}{19pt}\selectfont}
\newcommand{\sihao}{\fontsize{14pt}{17pt}\selectfont}
\newcommand{\xiaosihao}{\fontsize{12pt}{15pt}\selectfont}
\newcommand{\wuhao}{\fontsize{10.5pt}{13pt}\selectfont}
\newcommand{\xiaowuhao}{\fontsize{9pt}{12pt}\selectfont}
\newcommand{\liuhao}{\fontsize{7.875pt}{10pt}\selectfont}
\newcommand{\qihao}{\fontsize{5.25pt}{7pt}\selectfont}


%%%%%%%%%%%%%%%%%%%%%%%%%定义一些变量
\newcommand{\@rjtitle}{}
\newcommand{\@rjauthor}{}
\newcommand{\@rjinfor}{}

\newcommand{\rjtitle}[1]{\renewcommand{\@rjtitle}{#1}}
\newcommand{\rjauthor}[1]{\renewcommand{\@rjauthor}{#1}}
\newcommand{\rjinfor}[1]{\renewcommand{\@rjinfor}{#1}}


%%%%%%%%%%%%%%%%%%%%%%%%%定义一些命令
\newcommand{\rj}{软件学报}

\newcommand{\rjmaketitle}{%
	\begin{flushleft}
	    {\liuhao\song
	    \noindent
	    \begin{minipage}[t]{0.62\textwidth}
	    软件学报ISSN 1000-9825, CODEN RUXUEW\\
	    Journal of Software, [doi: 10.13328/j.cnki.jos.000000]\\
	    ©中国科学院软件研究所版权所有.
	    \end{minipage}\hfill
	    \begin{minipage}[t]{0.32\textwidth}
	    \raggedleft
	    E-mail: jos@iscas.ac.cn\\
	    http://www.jos.org.cn\\
	    Tel: +86-10-62562563
	    \end{minipage}
	    \par}
	    \vspace{0.6em}
		{\sihao\hei\@rjtitle}\\
		\vspace{6pt}
		{\xiaosihao\fs{\@rjauthor}}\\
		\vspace{8pt}
		{\zihao{8}\song\@rjinfor}
	\end{flushleft}}

\newcommand{\rjkeywords}[1]{%
	\par\noindent\xiaowuhao {\hei 关键词:}\hspace{1em}{\kai#1}\par\vspace{0.4em}}

% 中图法分类号（2025 模板新增位置：关键词下方）
\newcommand{\rjcategory}[1]{%
	\par\noindent\xiaowuhao {\hei 中图法分类号:}\hspace{1em}{\song #1}\par\vspace{1.0em}}

% 基金项目（2025 模板尾部）
\newcommand{\rjfunding}[1]{%
	\par\noindent{\xiaowuhao\song {\hei 基金项目:}\hspace{0.5em}#1}}

% 收稿/修改/采用时间（2025 模板最末行）
\newcommand{\rjdates}[1]{%
	\par\noindent{\xiaowuhao\song #1}}


%%%%%%%%%%%%%%%%%%%%%%%%%定义环境

\newenvironment{rjabstract}{
	 \renewcommand{\baselinestretch}{1.0}  % 摘要区单倍行距
	 \begin{flushleft}\xiaowuhao {\hei 摘\hspace{2em}要:} \kai}
{\end{flushleft}\xiaowuhao}



%%%%%%%%%%%%%%%%%%%%%%%%%定义计数器


%%%%%%%%%%%%%%%%%%%%%%%%%重新定义一下内置命令，例如section/subsection等命令
% \@startsection{name}{level}{indent}{beforskip}{afterskip}
% 2025 模板: Heading 1 = 10.5pt(五号)黑体, space_before=8pt, space_after=8pt
\renewcommand\section{\@startsection{section}{1}{\z@}%
	{8pt}%
	{8pt}%
	{\heiti\wuhao}}

% 2025 模板: Heading 2 = 9pt(小五号)黑体, space_before=1.2pt, space_after=1.2pt
\renewcommand\subsection{\@startsection{subsection}{2}{\z@}%
	{1.2pt}%
	{1.2pt}%
	{\heiti\xiaowuhao}}

\renewcommand{\thesection}{\arabic{section}}
\renewcommand{\thesubsection}{\thesection.\arabic{subsection}}

\renewcommand{\theenumi}{(\arabic{enumi})}




%%%%%%%%%%%%%%%%%%%%%%%%%开始设置页眉和页脚
\pagestyle{fancy}
\fancyhf{}                            % 清除页眉页脚
%\fancyhead[LO]{\kai\xiaowuhao 陈翔 等:静态软件缺陷预测方法研究}
%\fancyhead[RO]{\thepage}   % 设置奇数页右边页眉
\fancyhead[RE]{\xiaowuhao\textit{\xiaowuhao Journal of Software} {\kai 软件学报}}
\fancyhead[LE]{\thepage}              % 设置偶数页左边页眉
\renewcommand{\headrulewidth}{0pt}    % 去掉下划线

%\newcommand{\rjhead}[1]{\fancyhead[LO]{\xiaowuhao\kai #1}}
% 首页 masthead 按 2025 Word 样例作为正文开头内容排入 \rjmaketitle；
% 页眉仅用于第 2 页及以后。
\fancyhead[RO]{\ifthenelse{\value{page}=1}{}{\thepage}}
\newcommand{\rjhead}[1]{%
	\fancyhead[LO]{\ifthenelse{\value{page}=1}{}{\xiaowuhao\kai #1}}}

\setlength{\headheight}{13pt}

%%%%%%%%%%%%%%%%%%%%%%%%%正文排版参数
% 2025 模板: Normal 字号 9pt (小五号), 样式1 正文段落无额外行距
% 正文默认小五号、单倍行距（与 Word 模板一致）
\renewcommand{\normalsize}{\fontsize{9pt}{13pt}\selectfont}  % 9pt字号, 13pt行距 (≈ Word 单倍行距 + CJK 补偿)
\setlength{\parindent}{2em}  % 段首缩进两字符 (≈18pt at 9pt)
\setlength{\parskip}{0pt}    % 段间距为0 (模板无额外段间距)
\setlist{nosep}    %设置item之间的间距
