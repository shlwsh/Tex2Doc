# DOCX 内容与格式对比报告

## 输入

- Left: `examples/paper3/output/to-docx/v132-20260618-133910-论文稿件-jos-rust.docx`
- Right: `examples/paper3/output/to-docx/v12-论文稿件-jos-sh-20260618-133717.docx`

## 摘要

| 指标 | Left | Right | Delta |
| --- | ---: | ---: | ---: |
| 段落数 | 645 | 658 | 13 |
| 表格数 | 11 | 12 | 1 |
| 图片 drawing 数 | 10 | 10 | 0 |
| media 文件数 | 10 | 10 | 0 |

- 相同段落：448
- 近似修改段落：39
- 新增段落：171
- 删除段落：158
- 格式变更段落：15
- 真实格式差异段落：13
- run 分割差异段落（可忽略）：2
- document.xml 规范化 hash 相同：false
- styles.xml 规范化 hash 相同：false

## 段落样式分布

### Left

| 样式 | 段落数 |
| --- | ---: |
| (none) | 5 |
| JOSAbstractEn | 1 |
| JOSAbstractZh | 1 |
| JOSAuthorZh | 1 |
| JOSBody | 125 |
| JOSBodyNoIndent | 4 |
| JOSCaption | 21 |
| JOSCitation | 6 |
| JOSCode | 7 |
| JOSEnglishTitle | 1 |
| JOSHeading1 | 8 |
| JOSHeading2 | 38 |
| JOSHeading3 | 4 |
| JOSImage | 10 |
| JOSInstituteZh | 2 |
| JOSKeywords | 2 |
| JOSReference | 80 |
| JOSReferenceHeading | 2 |
| JOSTableText | 326 |
| JOSTitleZh | 1 |

### Right

| 样式 | 段落数 |
| --- | ---: |
| (none) | 36 |
| JOSAbstractEn | 1 |
| JOSAbstractZh | 1 |
| JOSAuthorZh | 1 |
| JOSBody | 114 |
| JOSBodyNoIndent | 1 |
| JOSCaption | 21 |
| JOSCitation | 6 |
| JOSCode | 4 |
| JOSEnglishTitle | 1 |
| JOSHeading1 | 8 |
| JOSHeading2 | 38 |
| JOSHeading3 | 4 |
| JOSImage | 10 |
| JOSInstituteZh | 2 |
| JOSKeywords | 2 |
| JOSReference | 78 |
| JOSReferenceHeading | 3 |
| JOSTableText | 326 |
| JOSTitleZh | 1 |

## 内容差异

| 类型 | Left# | Right# | 相似度 | Left 样式 | Right 样式 | 文本 |
| --- | ---: | ---: | ---: | --- | --- | --- |
| Modified | 2 | 2 | 1.000 | JOSAuthorZh | JOSAuthorZh | L: 石 洪 雷 , 赵 涓 涓<br>R: 石 洪 雷, 赵 涓 涓 |
| Delete | 4 | - | - | JOSInstituteZh | - | 通讯作者:石洪雷, E-mail: shihonglei0042@link.tyut.edu.cn |
| Delete | 5 | - | - | JOSAbstractZh | - | 摘 要: 微服务架构下，日志来源高度分散、格式各异，总量随并发请求线性增长，节点资源、网络带宽和集中存储的压力随之显著增加。传统全量采集模式难以兼顾高价值日志保留与系统开销控制。本文提出一种基于动态关注清单的微服务日志定向采集方法，核心思路是“只采必要日志”。主要贡献包括四个方面：（1）构建网关预筛选、节点定向采集与中心二次过滤的三层协同架构，以 Sidecar 模式部署，从源头实现采集前端减量；（2）提出动态关注度评分模型（DASM）... |
| Insert | - | 4 | - | - | JOSInstituteZh | 通讯作者: 石洪雷, E-mail: shihonglei0042@link.tyut.edu.cn |
| Insert | - | 5 | - | - | JOSAbstractZh | 摘 要: 微服务架构下，日志来源高度分散、格式各异，总量随并发请求线性增长，节点资源、网络带宽和集中存储的压力随之显著增加。传统全量采集模式难以兼顾高价值日志保留与系统开销控制。本文提出一种基于动态关注清单的微服务日志定向采集方法，核心思路是“只采必要日志”。主要贡献包括四个方面：（1）构建网关预筛选、节点定向采集与中心二次过滤的三层协同架构，以 Sidecar 模式部署，从源头实现采集前端减量；（2）提出动态关注度评分模型（DASM）... |
| Delete | 10 | - | - | JOSCitation | - | 英文引用格式: Shi HL, Zhao JJ. Dynamic attention list-based directed log collection method for microservices. Ruan Jian Xue Bao/Journal of Software (in Chinese). |
| Delete | 11 | - | - | JOSCitation | - | http://www.jos.org.cn/1000-9825/0000.htm |
| Insert | - | 10 | - | - | JOSCitation | 英文引用格式: Shi HL, Zhao JJ. Dynamic attention list-based directed log collection method for |
| Insert | - | 11 | - | - | JOSCitation | microservices. Ruan Jian Xue Bao/Journal of Software (in Chinese). http://www.jos.org.cn/1000-9825/0000.htm |
| Modified | 13 | 13 | 1.000 | JOSCitation | JOSCitation | L: SHI Hong-Lei , ZHAO Juan-Juan<br>R: SHI Hong-Lei, ZHAO Juan-Juan |
| Modified | 15 | 15 | 0.997 | JOSAbstractEn | JOSAbstractEn | L: Abstract: In microservice architectures, log sources are highly distributed with heterogeneous formats, and their aggregate volume grows linearly with request concurrency, imposing significant overhead on node resources,...<br>R: Abstract: In microservice architectures, log sources are highly distributed with heterogeneous formats, and their aggregate volume grows linearly with request concurrency, imposing significant overhead on node resources,... |
| Delete | 19 | - | - | JOSBody | - | 目前的研究主要集中在日志存储压缩、采样过滤、故障诊断和 eBPF 无侵入采集等方向，但很少有人从 网关流量感知 的角度出发，动态驱动各节点的应用日志定向采集。包航宇等[11]总结了智能运维的实践现状与标准化框架，贾统等[12]系统梳理了基于日志的分布式故障诊断技术。本文的思路不同，把关注点前移到 采集前端 ——网关是南北向流量的入口，天然掌握着 URL、状态码、响应时延等结构化信号。利用这些信号，不需要侵入业务代码，就能识别出哪些请求是... |
| Delete | 20 | - | - | JOSBody | - | 本文关注的核心问题是 ：在不侵入业务代码、不改变微服务调用链的前提下，能否利用网关层的流量异常信号，从采集前端大幅降低日志入库量，同时保留异常和慢请求的高价值上下文？围绕这个核心问题，本文拆解出四个科学子问题：（1）如何从采集前端实现大幅减量，同时保留故障诊断所需的关键上下文？（2）定向采集会不会给 Sidecar 带来不可接受的资源开销？（3）如何在不侵入业务代码的前提下，利用网关流量信号动态识别高价值请求，并确保采集策略在网关预筛选... |
| Delete | 21 | - | - | JOSBody | - | 本文提出以下科学假设 ：（H1）网关流量驱动的关注清单能使定向采集模式下的 Loki 入库量相比同架构全量采集显著下降，同时在规则级保留异常与慢请求的关键上下文。（H2）定向采集的 Sidecar 资源开销（CPU、内存）不高于同架构全量采集。（H3）网关访问日志中的 URL、状态码和响应时延足以动态生成覆盖异常与慢请求模式的关注清单；通过三次策略转换，可以维持跨层策略语义一致，形成端到端的采集闭环。（H4）固定缓存块与指数退避机制能够... |
| Delete | 22 | - | - | JOSBody | - | 基于上述问题与假设，本文提出 分布式定向日志采集框架 （Distributed Directed Log Collection Framework）。框架分为三层： 网关预筛选、节点定向采集、中心二次过滤 ，三层之间通过三次策略转换保持语义一致。和 Promtail/Fluent Bit “全量推送后在存储端压缩”的传统方案相比，本文直接在采集前端就把 Loki 入库量控制在百条量级（八节点实测详见 §6.3），从源头降低了网络和存储开... |
| Delete | 23 | - | - | JOSBody | - | （1） 三层协同定向采集架构 。网关节点和微服务节点均以 Sidecar 方式部署，中心负责策略生成与二次过滤，节点负责本地匹配、缓存与可靠上传，构建了“流量感知—定向采集—语义一致入库”的完整闭环。这个架构将日志减量逻辑从传统的后端治理前置到采集前端，补上了 AIOps 数据链路中“采集前端减量”这一缺失环节。 |
| Delete | 24 | - | - | JOSBody | - | （2） 动态关注度评分模型（DASM） 。提出以频次、错误率、延迟严重度和热度趋势四个归一化因子加权求和的动态评分机制（式 (1)），结合指数时间衰减（式 (2)）和负载自适应权重调节，实时生成高价值 URL 模式清单并下发至各节点（算法 1，排序实现复杂度上界为 O(N log N)，最小堆实现为 O(N log K)，K ≪ N）。和简单的频次×严重度加权相比，DASM 能够感知模式的时序变化趋势并自动适应不同负载特征，同时满足评分... |
| Delete | 25 | - | - | JOSBody | - | （3） 定向策略三次转换机制 。通过统一的 URL 泛化函数和清单版本控制，确保同一关注清单在网关预筛选、节点定向采集和中心二次过滤三层之间的语义一致性，避免跨层策略漂移（图 2）。 |
| Delete | 26 | - | - | JOSBody | - | （4） 资源受限环境下的可靠传输架构 。在 Sidecar 中引入固定缓存块（环形队列+双约束）和压力感知指数退避机制（式 (3)），辅以 BoltDB 本地兜底，在资源压力下提供有界内存占用和有界重试能力。 |
| Delete | 27 | - | - | JOSBody | - | （5） 真实部署验证 。在 8 微服务 DSB-Lite 真实部署环境下，n=5 配对实验显示 DASM 减量 25.9% ± 0.9%（p<0.001），且不引入额外延迟开销。 |
| Delete | 28 | - | - | JOSBody | - | （6） 与 OpenTelemetry 尾采样的工程对比 。在相同负载下对比 OTel Collector v0.103 + Tail Sampling，验证了 DASM 在 高价值端点保留率 和 决策延迟 两个维度上的优势。 |
| Insert | - | 19 | - | - | JOSBody | 目前的研究主要集中在日志存储压缩、采样过滤、故障诊断和 eBPF 无侵入采集等方向，但很少有人从网关流量感知的角度出发，动态驱动各节点的应用日志定向采集。包航宇等[11]总结了智能运维的实践现状与标准化框架，贾统等[12]系统梳理了基于日志的分布式故障诊断技术。本文的思路不同，把关注点前移到采集前端——网关是南北向流量的入口，天然掌握着 URL、状态码、响应时延等结构化信号。利用这些信号，不需要侵入业务代码，就能识别出哪些请求是高价值的... |
| Insert | - | 20 | - | - | JOSBody | 本文关注的核心问题是：在不侵入业务代码、不改变微服务调用链的前提下，能否利用网关层的流量异常信号，从采集前端大幅降低日志入库量，同时保留异常和慢请求的高价值上下文？围绕这个核心问题，本文拆解出四个科学子问题：（1）如何从采集前端实现大幅减量，同时保留故障诊断所需的关键上下文？（2）定向采集会不会给 Sidecar 带来不可接受的资源开销？（3）如何在不侵入业务代码的前提下，利用网关流量信号动态识别高价值请求，并确保采集策略在网关预筛选、... |
| Insert | - | 21 | - | - | JOSBody | 本文提出以下科学假设：（H1）网关流量驱动的关注清单能使定向采集模式下的 Loki 入库量相比同架构全量采集显著下降，同时在规则级保留异常与慢请求的关键上下文。（H2）定向采集的 Sidecar 资源开销（CPU、内存）不高于同架构全量采集。（H3）网关访问日志中的 URL、状态码和响应时延足以动态生成覆盖异常与慢请求模式的关注清单；通过三次策略转换，可以维持跨层策略语义一致，形成端到端的采集闭环。（H4）固定缓存块与指数退避机制能够在... |
| Insert | - | 22 | - | - | JOSBody | 基于上述问题与假设，本文提出分布式定向日志采集框架（Distributed Directed Log Collection Framework）。框架分为三层：网关预筛选、节点定向采集、中心二次过滤，三层之间通过三次策略转换保持语义一致。和 Promtail/Fluent Bit “全量推送后在存储端压缩”的传统方案相比，本文直接在采集前端就把 Loki 入库量控制在百条量级（八节点实测详见 §6.3），从源头降低了网络和存储开销。主要... |
| Insert | - | 23 | - | - | JOSBody | （1）三层协同定向采集架构。网关节点和微服务节点均以 Sidecar 方式部署，中心负责策略生成与二次过滤，节点负责本地匹配、缓存与可靠上传，构建了“流量感知—定向采集—语义一致入库”的完整闭环。这个架构将日志减量逻辑从传统的后端治理前置到采集前端，补上了 AIOps 数据链路中“采集前端减量”这一缺失环节。 |
| Insert | - | 24 | - | - | JOSBody | （2）动态关注度评分模型（DASM）。提出以频次、错误率、延迟严重度和热度趋势四个归一化因子加权求和的动态评分机制（式 (1)），结合指数时间衰减（式 (2)）和负载自适应权重调节，实时生成高价值 URL 模式清单并下发至各节点（算法 1，排序实现复杂度上界为 O(N log N)，最小堆实现为 O(N log K)，K ≪ N）。和简单的频次×严重度加权相比，DASM 能够感知模式的时序变化趋势并自动适应不同负载特征，同时满足评分有界... |
| Insert | - | 25 | - | - | JOSBody | （3）定向策略三次转换机制。通过统一的 URL 泛化函数和清单版本控制，确保同一关注清单在网关预筛选、节点定向采集和中心二次过滤三层之间的语义一致性，避免跨层策略漂移（图 2）。 |
| Insert | - | 26 | - | - | JOSBody | （4）资源受限环境下的可靠传输架构。在 Sidecar 中引入固定缓存块（环形队列+双约束）和压力感知指数退避机制（式 (3)），辅以 BoltDB 本地兜底，在资源压力下提供有界内存占用和有界重试能力。 |
| Insert | - | 27 | - | - | JOSBody | （5）真实部署验证。在 8 微服务 DSB-Lite 真实部署环境下，n=5 配对实验显示 DASM 减量 25.9% ± 0.9%（p<0.001），且不引入额外延迟开销。 |
| Insert | - | 28 | - | - | JOSBody | （6）与 OpenTelemetry 尾采样的工程对比。在相同负载下对比 OTel Collector v0.103 + Tail Sampling，验证了 DASM 在高价值端点保留率和决策延迟两个维度上的优势。 |
| Modified | 33 | 33 | 1.000 | JOSBody | JOSBody | L: ELK/EFK 以 Elasticsearch 为核心，检索能力强，但索引成本高昂。多项综述指出，大规模系统的日志在采集、传输、存储、解析和查询各阶段都会带来显著开销[7-9,13]。为降低这些开销，已有研究提出日志压缩、模式挖掘、模板解析、云端低成本存储和离线数据集构建等方法[14-17]。这些工作大多聚焦于日志入库之后的压缩与治理，前提是日志已被全量采集到中心端。本文关注的则是日志进入中心 之前 的 采集前端定向减量 。<br>R: ELK/EFK 以 Elasticsearch 为核心，检索能力强，但索引成本高昂。多项综述指出，大规模系统的日志在采集、传输、存储、解析和查询各阶段都会带来显著开销[7-9,13]。为降低这些开销，已有研究提出日志压缩、模式挖掘、模板解析、云端低成本存储和离线数据集构建等方法[14-17]。这些工作大多聚焦于日志入库之后的压缩与治理，前提是日志已被全量采集到中心端。本文关注的则是日志进入中心之前的采集前端定向减量。 |
| Modified | 43 | 43 | 1.000 | JOSBody | JOSBody | L: 包航宇等[11]基于大规模企业调研，总结了智能运维的实践现状，并提出了 AIOps-OSA 能力建设框架。后续研究也表明，生产系统需要将运行时数据、自动化分析和工程治理打通成闭环[20,39-40]。本文聚焦其中 数据采集能力 的前端减量环节：借助网关流量驱动的关注清单，从源头降低进入日志平台和后续分析模块的数据规模。<br>R: 包航宇等[11]基于大规模企业调研，总结了智能运维的实践现状，并提出了 AIOps-OSA 能力建设框架。后续研究也表明，生产系统需要将运行时数据、自动化分析和工程治理打通成闭环[20,39-40]。本文聚焦其中数据采集能力的前端减量环节：借助网关流量驱动的关注清单，从源头降低进入日志平台和后续分析模块的数据规模。 |
| Modified | 45 | 45 | 1.000 | JOSBody | JOSBody | L: 贾统等[12]聚焦日志收集之后的下游环节，系统梳理了日志解析、异常检测、故障定位和知识提取等技术。在日志解析方面，Logram、模板识别和大规模解析评测等工作推动了非结构化日志向结构化事件的转换[17,41-42]。在异常检测方面，CNN-text、LogFormer、深度学习失效预测、融合学习时序检测和微服务性能异常检测等研究展示了日志智能分析的最新进展[34,43-48]。多变量日志异常检测与图式根因分析则进一步拓宽了故障诊断的边界...<br>R: 贾统等[12]聚焦日志收集之后的下游环节，系统梳理了日志解析、异常检测、故障定位和知识提取等技术。在日志解析方面，Logram、模板识别和大规模解析评测等工作推动了非结构化日志向结构化事件的转换[17,41-42]。在异常检测方面，CNN-text、LogFormer、深度学习失效预测、融合学习时序检测和微服务性能异常检测等研究展示了日志智能分析的最新进展[34,43-48]。多变量日志异常检测与图式根因分析则进一步拓宽了故障诊断的边界... |
| Modified | 49 | 49 | 1.000 | JOSBody | JOSBody | L: eBPF 技术使内核级日志与事件采集成为可能[38,54]。eBPF 侧重 基础设施级 事件，对 应用级 日志的定向采集支持有限，且缺乏网关流量驱动的动态策略源。OpenTelemetry 尾部采样以 分布式追踪 span 为单位做取舍，与本文以 URL 模式 为粒度的应用日志定向过滤构成互补关系。<br>R: eBPF 技术使内核级日志与事件采集成为可能[38,54]。eBPF 侧重基础设施级事件，对应用级日志的定向采集支持有限，且缺乏网关流量驱动的动态策略源。OpenTelemetry 尾部采样以分布式追踪 span 为单位做取舍，与本文以URL 模式为粒度的应用日志定向过滤构成互补关系。 |
| Modified | 51 | 51 | 1.000 | JOSBody | JOSBody | L: 上述先例存在两个共同局限：日志过滤决策 局限于单一节点 （代理或网关），且多依赖 静态或手工配置 规则。本文的增量创新在于：以网关流量统计为输入，自动生成 Top-K URL 模式清单，经 gRPC 下发至多个微服务节点驱动应用日志定向采集，最后由中心二次过滤保障语义一致。这一流程形成了“网关流量 → 多节点应用日志动态清单 → 中心二次过滤”的 贯通闭环 ，以规则驱动的轻量匹配替代 ML 推理，更适合边缘资源受限环境。<br>R: 上述先例存在两个共同局限：日志过滤决策局限于单一节点（代理或网关），且多依赖静态或手工配置规则。本文的增量创新在于：以网关流量统计为输入，自动生成 Top-K URL 模式清单，经 gRPC 下发至多个微服务节点驱动应用日志定向采集，最后由中心二次过滤保障语义一致。这一流程形成了“网关流量 → 多节点应用日志动态清单 → 中心二次过滤”的贯通闭环，以规则驱动的轻量匹配替代 ML 推理，更适合边缘资源受限环境。 |
| Modified | 119 | 119 | 1.000 | JOSBody | JOSBody | L: 从上述分析可以看出，包航宇等的 AIOps 标准化研究着力解决平台能力建设问题[11]，贾统等的日志诊断综述着力解决后端分析方法体系问题[12]。二者都需要稳定、低成本且高价值的日志输入作为基础。本文定位为 AIOps 采集前端的定向减量技术 ：将网关 access log 作为动态策略输入，下发至各微服务节点执行应用日志定向采集，再由中心二次过滤后入库。借助关注清单、三次策略转换与可复现的 Docker 原型，本文在采集阶段将入库量控...<br>R: 从上述分析可以看出，包航宇等的 AIOps 标准化研究着力解决平台能力建设问题[11]，贾统等的日志诊断综述着力解决后端分析方法体系问题[12]。二者都需要稳定、低成本且高价值的日志输入作为基础。本文定位为AIOps 采集前端的定向减量技术：将网关 access log 作为动态策略输入，下发至各微服务节点执行应用日志定向采集，再由中心二次过滤后入库。借助关注清单、三次策略转换与可复现的 Docker 原型，本文在采集阶段将入库量控制在... |
| Insert | - | 120 | - | - | JOSBody | *八节点、180 s、n=3；全量基线发射频率（400 ms）高于定向模式（2 s），降幅为策略过滤与源配置联合效果；基于真实分布的多进程同频模拟表明，策略过滤独立降幅为 67.8%（§6.3）。 |
| Modified | 121 | 122 | 1.000 | JOSBody | JOSBody | L: OpenTelemetry 生态提供了多种采样策略：Tail Sampling Processor（基于 trace 错误率/延迟的后置概率采样）、Adaptive Sampling（Uber Jaeger 的自适应采样）、Collector Filter Processor（基于属性的预过滤）。本文与 OTel 采样策略的核心区别在于： OTel 侧重于追踪（Trace）维度的后置采样决策，本文聚焦于日志（Log）维度的内容驱动定向减...<br>R: OpenTelemetry 生态提供了多种采样策略：Tail Sampling Processor（基于 trace 错误率/延迟的后置概率采样）、Adaptive Sampling（Uber Jaeger 的自适应采样）、Collector Filter Processor（基于属性的预过滤）。本文与 OTel 采样策略的核心区别在于：OTel 侧重于追踪（Trace）维度的后置采样决策，本文聚焦于日志（Log）维度的内容驱动定向减量... |
| Delete | 124 | - | - | JOSBody | - | 本文框架采用 三层协同定向采集架构 ，由两个核心角色组成： 日志节点采集器（Agent） 和 日志中心（Center） 。Agent 有两种部署形态——网关节点实例和微服务节点实例。网关实例通过 OpenResty Lua 采集 HTTP 流量日志，微服务实例负责采集应用访问日志。Center 的职责包括：接收 Agent 上传的日志、生成关注清单、下发规则、执行二次过滤，最终将结果写入 Loki。总体架构见图 1。 |
| Delete | 125 | - | - | JOSBody | - | 从能力边界看，整个框架可分为三层： 网关预筛选、节点定向采集、中心二次过滤 ，如表 2 所示。三层分别负责运行时流量感知、本地日志减量和中心入库控制，每层的输入输出都可以独立审计，也方便与 AIOps 平台的数据采集能力对接。 |
| Insert | - | 125 | - | - | JOSBody | 本文框架采用三层协同定向采集架构，由两个核心角色组成：日志节点采集器（Agent）和日志中心（Center）。Agent 有两种部署形态——网关节点实例和微服务节点实例。网关实例通过 OpenResty Lua 采集 HTTP 流量日志，微服务实例负责采集应用访问日志。Center 的职责包括：接收 Agent 上传的日志、生成关注清单、下发规则、执行二次过滤，最终将结果写入 Loki。总体架构见图 1。 |
| Insert | - | 126 | - | - | JOSBody | 从能力边界看，整个框架可分为三层：网关预筛选、节点定向采集、中心二次过滤，如表 2 所示。三层分别负责运行时流量感知、本地日志减量和中心入库控制，每层的输入输出都可以独立审计，也方便与 AIOps 平台的数据采集能力对接。 |
| Modified | 153 | 154 | 1.000 | JOSBody | JOSBody | L: Sidecar 模式 ：Agent 与业务容器共享网络命名空间（ network_mode: service:* ），适用于 Kubernetes 或 Docker Compose 环境。 混合开发模式 ：基础设施跑在容器里，Center 和 Agent 在本地调试。原型部署在 WSL2 上，网关映射端口 8088（避开 Windows IIS 占用的 80 端口）。<br>R: Sidecar 模式：Agent 与业务容器共享网络命名空间（network_mode: service:*），适用于 Kubernetes 或 Docker Compose 环境。混合开发模式：基础设施跑在容器里，Center 和 Agent 在本地调试。原型部署在 WSL2 上，网关映射端口 8088（避开 Windows IIS 占用的 80 端口）。 |
| Delete | 156 | - | - | JOSBody | - | • 时间复杂度：清单生成 O(M log K) per node；Center 聚合 O(N · M · log K)；在 1000 节点规模下理论耗时 < 50 ms（实测 8 节点 2.3 ms）。 |
| Delete | 157 | - | - | JOSBody | - | • 空间复杂度：O(N · W · M)，与节点数线性相关；1000 节点 × 20 模式 × 3 窗口 × 64 B ≈ 3.7 MB，完全可承载于单机内存。 |
| Delete | 158 | - | - | JOSBody | - | • 通信复杂度：Agent → Center 增量同步 O(M) per node，O(N · M) 总通信量；1000 节点场景约 ∼ 2 MB/s，远低于 1 Gbps 网络。 |
| Delete | 159 | - | - | JOSBody | - | • Center 水平扩展：通过 URL 前缀分片（sharding by URL prefix），单 Center 可承载约 1000 节点；10000 节点仅需 10 个 Center 实例。 |
| Insert | - | 157 | - | - | JOSBody | itemize 时间复杂度：清单生成 O(M log K) per node；Center 聚合 O(N · M · log K)；在 1000 节点规模下理论耗时 < 50 ms（实测 8 节点 2.3 ms）。 空间复杂度：O(N · W · M)，与节点数线性相关；1000 节点 × 20 模式 × 3 窗口 × 64 B approx 3.7 MB，完全可承载于单机内存。 通信复杂度：Agent to Center 增量同步 O... |
| Modified | 163 | 161 | 0.926 | JOSBody | JOSBody | L: 输入 ：时间窗口内网关流量日志集合 L={l1,…,lN}；定向策略 S（响应时延阈值 T、错误码集合 E）；历史窗口统计 ℋ。 输出 ：关注清单 A={(pi,wi)}，pi 为泛化 URL 模式，wi ∈ [0,1] 为动态关注度评分。<br>R: 输入：时间窗口内网关流量日志集合 L={l1,…,lN}；定向策略 S（响应时延阈值 T、错误码集合 E）；历史窗口统计 mathcalH。 输出：关注清单 A={(pi,wi)}，pi 为泛化 URL 模式，wi ∈ [0,1] 为动态关注度评分。 |
| Delete | 165 | - | - | JOSBody | - | 简单的频次×严重度加权无法反映模式的时序变化趋势和负载特征差异。为此，本文提出 动态关注度评分模型 （Dynamic Attention Scoring Model, DASM），将四个归一化因子加权求和： |
| Delete | 166 | - | - | JOSCode | - | Score(u,t)=α Freq(u,t)+β Err(u,t)+γ Delay(u,t)+δ Trend(u,t) (1) |
| Delete | 167 | - | - | - | - | Score((u,t))==αFreq((u,t))++βErr((u,t))++γDelay((u,t))++δTrend((u,t))eq:dasm |
| Delete | 168 | - | - | JOSBody | - | 其中 α+β+γ+δ=1（均非负），四个因子分别定义为： |
| Delete | 169 | - | - | JOSBody | - | • Freq(u,t)=count(u)/maxv count(v)，归一化频次； |
| Delete | 170 | - | - | JOSBody | - | • Err(u,t)=err_count(u)/count(u)，错误率； |
| Delete | 171 | - | - | JOSBody | - | • Delay(u,t)=min(1, rt(u)/T)，延迟严重度； |
| Delete | 172 | - | - | JOSBody | - | • Trend(u,t)=σ\!((Freqt-Freqt-1)/max(Freqt-1,ε))，热度趋势（σ 为 sigmoid 函数，将变化率映射到 (0,1)）。 |
| Delete | 173 | - | - | JOSBody | - | 为了让近期异常的权重更高，引入 指数时间衰减 ： |
| Delete | 174 | - | - | JOSCode | - | wdecay(u,t)=Σi wi·exp(-λ (t-ti)) (2) |
| Delete | 175 | - | - | - | - | wdecay((u,t))==∑iwi··exp((--λ((t--ti))))eq:decay |
| Delete | 176 | - | - | JOSBody | - | 其中 λ 为衰减系数（默认 0.1/s），ti 为第 i 个历史窗口的时间戳。 |
| Delete | 177 | - | - | JOSBody | - | 参数自适应。 当全局错误率超过 10% 时，β 自动提升 50%；当全局平均延迟超过 2T 时，γ 自动提升 50%；提升后重新归一化，确保权重系数之和为 1。默认均衡模式取 α=0.3，β=0.3，γ=0.2，δ=0.2。 |
| Insert | - | 163 | - | - | JOSBody | 简单的频次×严重度加权无法反映模式的时序变化趋势和负载特征差异。为此，本文提出动态关注度评分模型（Dynamic Attention Scoring Model, DASM），将四个归一化因子加权求和： |
| Insert | - | 164 | - | - | JOSCode | Score(u,t)=α Freq(u,t)+β Err(u,t)+gamma Delay(u,t)+delta Trend(u,t) (1) |
| Insert | - | 165 | - | - | JOSBody | 其中 α+β+gamma+delta=1（均非负），四个因子分别定义为： itemize Freq(u,t)=count(u)/max_v count(v)，归一化频次； Err(u,t)=err_count(u)/count(u)，错误率； Delay(u,t)=minbigl(1, overlinert(u)/Tbigr)，延迟严重度； Trend(u,t)=sigma\!bigl((Freq_t-Freq_t-1)/max(Fre... |
| Insert | - | 166 | - | - | JOSBody | 为了让近期异常的权重更高，引入指数时间衰减： |
| Insert | - | 167 | - | - | JOSCode | wdecay(u, t) = sumi wi·exp\!(-lambda (t-ti)) (2) |
| Insert | - | 168 | - | - | JOSBody | 其中 lambda 为衰减系数（默认 0.1/s），ti 为第 i 个历史窗口的时间戳。 |
| Insert | - | 169 | - | - | JOSBody | 参数自适应。当全局错误率超过 10% 时，β 自动提升 50%；当全局平均延迟超过 2T 时，gamma 自动提升 50%；提升后重新归一化，确保权重系数之和为 1。默认均衡模式取 α=0.3，β=0.3，gamma=0.2，delta=0.2。 |
| Delete | 179 | - | - | JOSCode | - | Input: 流量日志集合 L，策略 S=(T,E,K,TTL)，历史窗口 ℋ |
| Insert | - | 171 | - | - | - | Input: 流量日志集合 L，策略 S=(T,E,K,TTL)，历史窗口 mathcalH |
| Delete | 181 | - | - | JOSCode | - | \| )，历史窗口{H \| \| $ \| \| H ← ∅ \| ForEach \| ForEach () \| End \| end \| \| { \| If \| If () \| End \| end \| \| (l) > T$ {or \| \| ${status \| \| (l) E$ \| \| { \| \| $p {Generalize \| \| (l.{url \| \| )$ {数字→{id}，UUID→{uuid\ \| \| $H[p].{count \| \| ... |
| Delete | 182 | - | - | JOSBody | - | 复杂度分析。 线性扫描 N 条日志为 O(N)；评分计算为 O(M)（M 为泛化模式数，M≤ N）；Top-K 选取为 O(M log K)（最小堆）或 O(M log M)（排序）。总体复杂度 O(N+M log M)，上界可简写为 O(N log N)。当前原型采用排序实现，便于审计和复现。 |
| Delete | 183 | - | - | JOSBodyNoIndent | - | 定理 对任意 URL 模式 u 和时间 t，Score(u,t)∈[0,1]。 |
| Delete | 184 | - | - | JOSBodyNoIndent | - | 证明 四个因子 Freq、Err、Delay、Trend 的值域均为 [0,1]（Trend 经 sigmoid 映射后属于 (0,1)⊂[0,1]）。权重系数满足 α,β,γ,δ≥ 0 且 α+β+γ+δ=1。由凸组合性质，Score=α f1+β f2+γ f3+δ f4∈[0,1]。 |
| Delete | 185 | - | - | JOSBodyNoIndent | - | 命题 当 δ=0（无趋势因子）且 λ=0（无时间衰减）时，DASM 退化为静态加权 Top-K。 |
| Delete | 186 | - | - | JOSBody | - | URL 泛化规则包括：纯数字路径段映射为 {id} ，UUID 或长哈希段映射为 {uuid} ，查询参数按键名归一化。如果遇到异常极稀疏的窗口，算法会退化为仅保留 ERROR 级别的兜底规则。后文微基准测试显示，处理 5000 条合成日志大约耗时 15 ms。和 Promtail 依赖静态标签配置不同，DASM 以网关 access log 为策略输入，通过趋势感知和时间衰减动态识别高价值 URL 模式，避免历史噪声污染清单。 |
| Insert | - | 173 | - | - | - | 1 |
| Insert | - | 174 | - | - | - | H ← ∅; |

## 格式差异

| Left# | Right# | 文本 | 字段变化 |
| ---: | ---: | --- | --- |
| 114 | 114 | 72 vs 4388 条* | runs: `style=-;b=false;i=false;u=-;va=-;sz=15;font=Times New Roman/宋体 x1` -> `style=-;b=false;i=false;u=-;va=-;sz=15;font=Times New Roman/宋体 x1; style=-;b=false;i=false;u=-;va=superscript;sz=15;font=Times New Roman/宋体 x1` |
| 116 | 116 | 0.05% CPU* | runs: `style=-;b=false;i=false;u=-;va=-;sz=15;font=Times New Roman/宋体 x1` -> `style=-;b=false;i=false;u=-;va=-;sz=15;font=Times New Roman/宋体 x1; style=-;b=false;i=false;u=-;va=superscript;sz=15;font=Times New Roman/宋体 x1` |
| 144 | 145 | 图 1 分布式定向日志采集系统总体架构 | runs: `style=-;b=false;i=true;u=-;va=-;sz=-;font=-/- x1` -> `style=-;b=false;i=false;u=-;va=-;sz=-;font=-/- x1` |
| 178 | 170 | Algorithm 1: 基于 DASM 的关注清单动态生成 | runs: `style=-;b=true;i=false;u=-;va=-;sz=-;font=-/- x1` -> `style=-;b=false;i=false;u=-;va=-;sz=18;font=Times New Roman/宋体 x1; style=-;b=true;i=false;u=-;va=-;sz=18;font=Times New Roman/宋体 x1` |
| 180 | 172 | Output: 关注清单 A | paragraph.style: `JOSCode` -> `-`<br>runs: `style=-;b=false;i=false;u=-;va=-;sz=-;font=-/- x1; style=-;b=true;i=false;u=-;va=-;sz=-;font=-/- x1` -> `style=-;b=false;i=false;u=-;va=-;sz=18;font=Times New Roman/宋体 x1; style=-;b=true;i=false;u=-;va=-;sz=18;font=Times New Roman/宋体 x1` |
| 253 | 276 | 图 2 定向策略三次转换算法流程 | runs: `style=-;b=false;i=true;u=-;va=-;sz=-;font=-/- x1` -> `style=-;b=false;i=false;u=-;va=-;sz=-;font=-/- x1` |
| 260 | 282 | 图 3 指数退避重试延迟分布 | runs: `style=-;b=false;i=true;u=-;va=-;sz=-;font=-/- x1` -> `style=-;b=false;i=false;u=-;va=-;sz=-;font=-/- x1` |
| 373 | 395 | 图 4 定向采集与全量采集资源消耗对比（八节点，180 s，n=10 配对实测） | runs: `style=-;b=false;i=true;u=-;va=-;sz=-;font=-/- x1` -> `style=-;b=false;i=false;u=-;va=-;sz=-;font=-/- x1` |
| 378 | 400 | 图 5 消融实验结果（策略仿真，非集群实测） | runs: `style=-;b=false;i=true;u=-;va=-;sz=-;font=-/- x1` -> `style=-;b=false;i=false;u=-;va=-;sz=-;font=-/- x1` |
| 381 | 403 | 图 6 关注清单生成算法微基准 | runs: `style=-;b=false;i=true;u=-;va=-;sz=-;font=-/- x1` -> `style=-;b=false;i=false;u=-;va=-;sz=-;font=-/- x1` |
| 391 | 413 | 图 7 多规模扩展实验（绿色/实线：8/16/32 节点集群实测；灰色/虚线：64 节点线性外推，非实测） | runs: `style=-;b=false;i=true;u=-;va=-;sz=-;font=-/- x1` -> `style=-;b=false;i=false;u=-;va=-;sz=-;font=-/- x1` |
| 397 | 419 | 图 8 工业基线趋势对比（P3 定向采集为集群实测；Promtail 为同负载多进程规则复现；OTel/eBPF 为同负载估算，柱形样式区分证据等级） | runs: `style=-;b=false;i=true;u=-;va=-;sz=-;font=-/- x1` -> `style=-;b=false;i=false;u=-;va=-;sz=-;font=-/- x1` |
| 422 | 444 | 图 9 不同负载下的减量效果对比：(a) httpbin 与 DSB 的降幅/召回对比；(b) DSB 负载下不同 RPS 的扩展性 | runs: `style=-;b=false;i=true;u=-;va=-;sz=-;font=-/- x1` -> `style=-;b=false;i=false;u=-;va=-;sz=-;font=-/- x1` |
| 533 | 555 | 图 10 参数敏感性分析：(a) Top-K 取值对入库量和覆盖率的影响；(b) 延迟阈值 T 对入库量和高价值日志数的影响 | runs: `style=-;b=false;i=true;u=-;va=-;sz=-;font=-/- x1` -> `style=-;b=false;i=false;u=-;va=-;sz=-;font=-/- x1` |
| 641 | 656 | 作者简介 | paragraph.style: `JOSBody` -> `JOSReferenceHeading`<br>runs: `style=-;b=true;i=false;u=-;va=-;sz=-;font=-/- x1` -> `style=-;b=false;i=false;u=-;va=-;sz=-;font=-/- x1` |

## OOXML Hash

| Part | Equal | Left hash | Right hash |
| --- | --- | --- | --- |
| word/document.xml | false | 05c1d2a8aab87344 | abef806ccb0811fd |
| word/styles.xml | false | a0096d5d826ae2d3 | 381ee950c2ac8390 |
