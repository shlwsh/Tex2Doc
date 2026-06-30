# **Doc-engine：LaTeX → DOCX 纯 Rust 核心 \+ Flutter 全平台转换工具完整技术实现方案（V1）**
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



本方案旨在面向学术论文的高保真、本地化格式转换需求，提供一套基于纯 Rust 核心解析引擎与 Flutter 全平台交互界面的全栈设计。本方案严格遵循 V1 范围收敛原则，在彻底摆脱传统重型 TeX 编译环境依赖的同时，将抽象语法树（Semantic AST）打造为具备高扩展性的长期技术资产。

## **1\. 项目概述与边界控制**

Doc-engine V1 阶段的核心定位是“轻量、高效、零依赖”。它通过直接解析 LaTeX 源码并重构为 Office Open XML (OOXML) 标准，解决科研人员在主流操作系统及浏览器环境下的跨平台文档互转痛点。

### **1.1 V1 严格范围矩阵**

| 核心支持功能 (80%+ 学术论文场景) | 明确排除功能 (V1 阶段收敛边界)   |
| :---- | :---- |
| 文档元数据（标题、作者、摘要、关键词） 多级标题与标准正文段落排版 有序/无序嵌套列表 (enumerate, itemize) 标准学术表格 (tabular 环境及三线表格式) 图片插入与自动化交叉引用 (\\includegraphics, \\ref) 行内与块级数学公式 (LaTeX Math → MathML → OMML) 多文件嵌套解析 (\\include, \\input 自动解析拓扑) BibLaTeX 文献自动解析与末尾 Bibliography 渲染 中文字符集原生支持与 CTeX 字体映射 | TikZ/PGF 等低层矢量绘图宏包直接渲染 Beamer 幻灯片排版框架解析 用户深度自定义的复杂底层宏展开机制 PDF、PPT 等非 DOCX 格式的 Writer 扩展 基于大语言模型的 AI 格式自愈与文本修复 双向同步编辑与实时协同机制 |

## **2\. 总体架构与数据流向设计**

系统采用前后台解耦的敏捷分层构型，核心引擎完全用 Rust 编写，通过高性能 FFI 或 WebAssembly 编译链路向各前端平台输出底层能力。

### **2.1 系统层次拓扑**

**用户界面层：**包含 Flutter 桌面应用（Windows/macOS/Linux）、移动端（Android/iOS）、基于 Flutter Web 的 PWA 应用、Chrome 浏览器扩展插件以及本地纯命令行 CLI 工具。  
**桥接交互层：**本地端采用 flutter\_rust\_bridge v2 执行高性能内存指针映射；Web 与插件端采用 wasm-bindgen 执行 V8 引擎内的双向数据序列化；云端则通过标准 HTTP Multipart 请求与后端 Axum 服务通信。  
**核心逻辑引擎 (Rust Core)：**包含 latex-reader (词法与语法分析)、semantic-ast (核心语义模型)、docx-writer (OOXML 组装)、bib (文献解析) 和 utils (资源与图片转换管道)。

### **2.2 核心长期资产：Semantic AST 的前瞻性设计**

系统放弃了“输入直转输出”的流水线模式，将中间层抽象语法树（Semantic AST）作为核心资产进行沉淀。AST 采用标准强类型 Enum 构建，完全解耦了 Reader 与 Writer。V1 聚焦于 docx-writer，未来该资产可平滑对接 Markdown、HTML、Typst 等全新 Writer，甚至直接对接到大模型 Agent 或 MCP (Model Context Protocol) 插件接口上，实现高度的语义可编辑性。

## **3\. Monorepo 目录结构规范**

项目基于 Cargo Workspace 与标准 Flutter 目录结构进行单仓（Monorepo）管理，以最大限度简化多端同步构建与跨语言 FFI 代码生成的依赖复杂度：

`doc-engine/`  
`├── Cargo.toml                  # Workspace 顶层总控配置`  
`├── crates/`  
`│   ├── core/                   # 核心门面库，暴露底层统一对外的 FFI/WASM 接口`  
`│   ├── latex-reader/           # 基于 Logos + Rowan 的词法与语法分析器`  
`│   ├── semantic-ast/           # 统一语义块模型 (Heading, Paragraph, Table, Equation)`  
`│   ├── docx-writer/            # 负责 quick-xml 的 OOXML 序列化与 ZIP 打包`  
`│   ├── bib/                    # 基于 biblatex crate 的文献关系解析器`  
`│   ├── utils/                  # 包含 ZIP 虚拟文件系统、图片解码、include 路径解算`  
`│   ├── server/                 # 后端 Axum 异步 Web 服务`  
`│   └── wasm/                   # 封装给 WASM 绑定的专用包装层`  
`├── flutter_app/                # 主 Flutter 多端跨平台工程`  
`│   ├── lib/                    # 采用 Riverpod 体系的状态管理与 UI 视图代码`  
`│   ├── rust/                   # flutter_rust_bridge 自动生成的 Dart-Rust 桥接代码`  
`│   └── assets/                 # 预置的 reference.docx 样式模板文件`  
`├── extension/                  # Chrome 浏览器扩展工程 (复用 Flutter Web 核心逻辑)`  
`│   ├── manifest.json           # Manifest V3 配置文件`  
`│   └── background.js           # 扩展 Service Worker 拦截与调度逻辑`  
`├── tests/                      # 端到端集成测试与 Insta 快照断言库`  
`│   ├── ieee_fixtures/          # IEEE 经典期刊格式测试样例`  
`│   └── ctex_fixtures/          # 中文期刊与大文章嵌套包含样例`  
`└── docs/                       # 技术演进与架构设计白皮书`

## **4\. 核心模块具体实现细节与算法设计**

### **4.1 latex-reader 与 Rowan 容错解析引擎**

传统的 LaTeX 解析器在面对语法不严谨或缺少特定宏包的源码时极易发生崩溃。本系统在 latex-reader 中引入了 **Two-Pass Parsing (双阶段解析)** 架构：

1. **Pass 1 (Pre-processor):** 扫描文档流中的 \\include{file} 与 \\input{file} 指令，基于 utils 的虚拟文件系统层建立完整的文档依赖拓扑图，将其展开为单一连续的 Token 输入流。  
2. **Pass 2 (Syntax Builder):** 使用 logos 将输入流高频切分为 Token，随后喂入 rowan 语法树构建器。Rowan 维护无损语法树 (Lossless Syntax Tree)，保留所有空格和注释。当遇到无法识别的宏（如自定义绘图指令）时，解析器触发事件驱动的错误恢复机制，将该段未知节点包装为 SyntaxError 节点，并借助闭合符号（如大括号或换行）执行指针前移，继续解析后续的正文与图表，确保转换流程“绝对不中断”。

### **4.2 Semantic AST 数据结构枚举示例**

语义层充当彻底消融 LaTeX 特性的标准结构。各个节点均附带 span 属性，用以记录其在原始文本中的绝对字符偏移量，为前端 GUI 预留“双向定位跳转”能力：

`pub struct Document {`  
    `pub metadata: MetaData,`  
    `pub blocks: Vec<Block>,`  
`}`

`pub enum Block {`  
    `Heading { level: u8, text: String, span: Range<usize> },`  
    `Paragraph { runs: Vec<TextRun>, span: Range<usize> },`  
    `List { is_ordered: bool, items: Vec<Vec<Block>> },`  
    `Table { rows: Vec<TableRow>, caption: Option<String> },`  
    `Figure { path: String, caption: Option<String>, scale: f32 },`  
    `Equation { mathml: String, is_block: bool },`  
`}`

### **4.3 docx-writer 与 OOXML 序列化矩阵**

docx-writer 模块放弃使用开销高昂的高层第三方库，直接基于 quick-xml 流式写入 Office Open XML 标准。它采用高度解耦的底层三层流水线模式：

* **docx-model:** 严格遵循 XML Schema 定义的扁平化结构体，包含段落样式属性（pPr）、文本运行属性（rPr）等。  
* **docx-serializer:** 将 semantic-ast 块逐一映射并流式序列化为 document.xml、styles.xml、document.xml.rels。系统实行强样式分离，正文、标题、图表题注的字体与字号均在 styles.xml 中统一定义，严禁将具体格式内联硬编码至 document.xml。  
* **docx-packer:** 负责将内存中组装完毕的 XML 树结构与提取压缩出来的图片多媒体资源一并打包，通过 zip crate 压缩输出标准符合性的 .docx 二进制文件。

### **4.4 数学公式管道转化 (LaTeX Math → MathML → OMML)**

为了使输出的 Word 文档中公式完全具备原生可编辑性（非静态图片），公式转换采用标准转换矩阵：首先利用 latex2mathml 将 LaTeX 公式字符串转换为语义明确的 MathML 结构，接着在 docx-writer 内部通过专用的内置节点映射管道，将 MathML 树无损平铺重构为 Word 专属的 **OMML (Office Math Markup Language)**。转换后的结构直接作为 \<m:oMathPara\> 标签对写入 OOXML 中，用户双击即可在 Microsoft Word 内通过原生公式编辑器进行二次调整。

## **5\. 前端功能与多平台交互界面细化设计**

### **5.1 Flutter 桌面端与移动端 App 交互设计**

主控 App 基于 Google Material 3 规范构建响应式动效界面。核心面板分为：

* **中央看板（工作台）：**大面积支持系统级原生拖拽（Drag and Drop）响应区。当用户拖入单个 .tex 文件或包含完整图片及 .bib 依赖的工程 .zip 压缩包时，工作台即刻激活任务流，展示文件列表骨架屏与实时文件大小解析。  
* **高级选项侧边栏（配置矩阵）：**用户可动态切换预置的论文样式模板（如标准 IEEE 双栏、Springer 单栏或自定义上传的 reference.docx 模板）。侧边栏还提供中文字体底层映射机制配置项（例如：允许指定 LaTeX 中的宋体/黑体自动绑定映射为 Word 中的标准宋体、微软雅黑或仿宋）。  
* **状态总线与动态进度：**借助 flutter\_rust\_bridge v2 的底层 Stream 监听能力，转换过程不再阻塞前端主 UI 线程。前端建立响应式进度条，实时显示当前正在执行的具体阶段（例如：\[1/4\] 正在解析多文件拓扑... \[2/4\] 正在解析数学公式... \[3/4\] 正在组装 OOXML 矩阵... \[4/4\] 正在执行 ZIP 压包）。  
* **实时日志审计抽屉：**底端提供可弹出的详细开发者调试日志区。实时过滤并打印 Rust 核心抛出的语法警告信息，例如："Warning: Macro \\tikz inherited is not supported in V1. Fallback to plain text at span \[1240..1280\]."。允许用户一键复制日志以方便调试。

### **5.2 Chrome 浏览器扩展插件（Manifest V3）深度交互**

扩展插件针对学术轻量级快捷提取场景进行定制：

* **Popup 极简转换窗：**点击扩展图标弹出 360px 宽度的悬浮视窗，包含一个精简的文件上传控件以及历史转换成功文件的本地快捷下载链接。  
* **上下文菜单注入 (Context Menu)：**当用户在 Overleaf、ArXiv 或任何学术网页上鼠标圈选一段 LaTeX 源码公式或段落时，右键菜单会自动弹出 "使用 Doc-engine 转换至 Word" 按钮。点击后，后台 Service Worker 激活并调用 WASM 核心模块进行实时本地内存互转，并在剪贴板中自动生成带有 OOXML 富文本格式的片段，用户可直接在本地 Word 里执行 Ctrl+V 完美粘帖。  
* **边缘分流机制：**Service Worker 会实时监测待转换文件大小，若文件大于 5MB 或包含复杂的依赖，会自动在前端弹出提示气泡，一键导流跳转至功能完备的本地 App 或云端 Web PWA 转换页。

### **5.3 Flutter Web (PWA) 响应式与全离线验证端**

Web 端扮演无门槛体验中心与快速在线转型的双重角色：

* **渐进式 Web 应用支持 (PWA)：**完全配置 manifest.json 与高性能 service\_worker.js。用户在浏览器首次打开后，可直接点击地址栏右侧将其作为原生应用常驻安装至操作系统桌面。  
* **纯本地 WASM 运算沙箱：**借助于 wasm-pack 构建的 doc\_engine\_wasm.js，在用户授权离线后，Service Worker 完美接管网络拦截。用户即使完全断网，依然可以通过浏览器窗口直接在前端沙箱内通过 WASM 转换小于 10MB 的常规论文工程。所有的文件读写均在浏览器分配的内存 VFS 中闭环进行，保障数据绝对隐私安全。  
* **本地历史存储网格：**利用前端 IndexedDB 技术，本地 PWA 自动维护一个最大容量为 50 条的转换流水账单，记录每一次转换的文件名、MD5 哈希值、转换时间以及存储在浏览器本地沙箱内的历史 DOCX 二进制导出实体。

### **5.4 CLI 命令行工具定义**

基于 clap v4 派生宏构建高阶工程化命令行工具，专为服务器脚本批量处理或极客用户打造：

`doc-engine convert <INPUT_PATH> [OUTPUT_PATH] --template <TEMPLATE_DOCX> --verbose`

支持标准 stdout 流式日志输出，并能够通过退出状态码（0 代表成功，1 代表解析断言失败，2 代表 IO 文件缺失）完美嵌入至自动化 CI/CD 或本地编译监控脚本中。

## **6\. 工程落地、双轨并发里程碑与质量保障**

### **6.1 双轨倒排里程碑控制矩阵 (14周完成闭环)**

| 阶段 (Sprint) | Rust Core 引擎侧核心输出目标 | Flutter / 多端 UI 前端侧并发配合目标   |
| :---- | :---- | :---- |
| **M1-M2 (2周)** | 完成 Logos 词法分析定义；构建 Rowan 零拷贝语法树总控骨架；跑通端到端最简 Heading 与 Paragraph 语义解析。 | 初始化 Monorepo 仓储拓扑；配置自动化 CI/CD 打包流水线；生成最基础的 FFI 桥接接口代码。 |
| **M3-M4 (2周)** | 引入 biblatex 解析器，解析学术论文中的引用链路；编写标准的 List、Table 和 Figure 处理子 crate。 | 构建桌面端主 Material 3 工作台 UI；完成多文件拖拽拖放功能开发；引入 Riverpod 异步状态管理总线。 |
| **M5-M6 (2周)** | 全面攻坚数学公式流水线，实现 LaTeX Math → MathML → OMML 映射；在中文字符集层注入 CTeX 标准样式映射表。 | 对接 FFI 异步 Stream，实现转换进度的实时高帧率动画展示；完成前端高级参数配置面板。 |
| **M7-M8 (2周)** | 打磨 docx-writer，确保完全融合外部自定义 reference.docx 样式模板，实现复杂排版无损融合。 | 完成前端底层日志抽屉的异常抓取过滤与实时展示逻辑。 |
| **M9-M10 (2周)** | 支持多文件嵌套 \\include 解析算法与完整的 .zip 论文工程多资源多图片依赖拓扑解析。 | 深度联调 CLI 命令控制工具，确保在 Linux、Windows 等多环境下 Clap 指令表现一致。 |
| **M11-M12 (2周)** | 通过 WASM 分流，提供针对 WebAssembly 环境的专用裁剪内存控制优化包。 | 全面打磨 PWA 应用离线 Service Worker 缓存，并完成 Chrome 扩展插件（Manifest V3）Popup 悬浮视窗的编码。 |
| **M13-M14 (2周)** | 搭建 Axum 高并发 Web 后端转换服务原型；提供异步队列处理大文件上云的 Fallback 安全降级通道。 | 多平台混合发布前的兼容性全方位黑盒冒烟测试与安装包静态签名发布。 |

### **6.2 质量保障与快照断言策略**

为了防止核心引擎在版本迭代中发生样式劣化（Regression），项目引入 insta 快照测试框架。针对 IEEE 经典期刊、Springer 单栏模板等主流学术 Fixtures 建立回归测试矩阵。每次代码提交时，测试系统会自动执行解包，比对生成的 OOXML 内部关键 XML 标签（如 document.xml 的具体 DOM 节点排列），一旦发现标签偏离安全边界即刻阻断 CI 流水线，以此保障工程的高度健壮性与输出样式的一致性。