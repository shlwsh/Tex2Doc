# TeX 标准 AST 结构摘要

- schema_version：`0.1`
- blocks：195
- diagnostics：0

## 块序列

| # | Node | Kind | Number | Style | Text Preview |
|---:|---|---|---|---|---|
| 0 | `B0001` | Heading | 1 | `JOSHeading1` | 引言 |
| 1 | `B0002` | Paragraph |  | `JOSBody` | 微服务架构把一个大系统拆成多个独立服务，带来了更好的可扩展性和开发灵活性。容器编排、Sidecar 代理和服务治理等技术的成熟，使云原生应用的大规模部署成为现实[1-6]。拆分也带来一个棘手问题：日志高度分散。每个服务实例各自输出日志，格式 |
| 2 | `B0003` | Paragraph |  | `JOSBody` | 目前的研究主要集中在日志存储压缩、采样过滤、故障诊断和 eBPF 无侵入采集等方向，但很少有人从网关流量感知的角度出发，动态驱动各节点的应用日志定向采集。包航宇等[11]总结了智能运维的实践现状与标准化框架，贾统等[12]系统梳理了基于日志 |
| 3 | `B0004` | Paragraph |  | `JOSBody` | 本文关注的核心问题是：在不侵入业务代码、不改变微服务调用链的前提下，能否利用网关层的流量异常信号，从采集前端大幅降低日志入库量，同时保留异常和慢请求的高价值上下文？围绕这个核心问题，本文拆解出四个科学子问题：（1）如何从采集前端实现大幅减量 |
| 4 | `B0005` | Paragraph |  | `JOSBody` | 本文提出以下科学假设：（H1）网关流量驱动的关注清单能使定向采集模式下的 Loki 入库量相比同架构全量采集显著下降，同时在规则级保留异常与慢请求的关键上下文。（H2）定向采集的 Sidecar 资源开销（CPU、内存）不高于同架构全量采集 |
| 5 | `B0006` | Paragraph |  | `JOSBody` | 基于上述问题与假设，本文提出分布式定向日志采集框架（Distributed Directed Log Collection Framework）。框架分为三层：网关预筛选、节点定向采集、中心二次过滤，三层之间通过三次策略转换保持语义一致。和 |
| 6 | `B0007` | Paragraph |  | `JOSBody` | （1）三层协同定向采集架构。网关节点和微服务节点均以 Sidecar 方式部署，中心负责策略生成与二次过滤，节点负责本地匹配、缓存与可靠上传，构建了“流量感知—定向采集—语义一致入库”的完整闭环。这个架构将日志减量逻辑从传统的后端治理前置到 |
| 7 | `B0008` | Paragraph |  | `JOSBody` | （2）动态关注度评分模型（DASM）。提出以频次、错误率、延迟严重度和热度趋势四个归一化因子加权求和的动态评分机制（式 (1)），结合指数时间衰减（式 (2)）和负载自适应权重调节，实时生成高价值 URL 模式清单并下发至各节点（算法 1， |
| 8 | `B0009` | Paragraph |  | `JOSBody` | （3）定向策略三次转换机制。通过统一的 URL 泛化函数和清单版本控制，确保同一关注清单在网关预筛选、节点定向采集和中心二次过滤三层之间的语义一致性，避免跨层策略漂移（图 2）。 |
| 9 | `B0010` | Paragraph |  | `JOSBody` | （4）资源受限环境下的可靠传输架构。在 Sidecar 中引入固定缓存块（环形队列+双约束）和压力感知指数退避机制（式 (3)），辅以 BoltDB 本地兜底，在资源压力下提供有界内存占用和有界重试能力。 |
| 10 | `B0011` | Paragraph |  | `JOSBody` | （5）真实部署验证。在 8 微服务 DSB-Lite 真实部署环境下，n=5 配对实验显示 DASM 减量 25.9% ± 0.9%（p<0.001），且不引入额外延迟开销。 |
| 11 | `B0012` | Paragraph |  | `JOSBody` | （6）与 OpenTelemetry 尾采样的工程对比。在相同负载下对比 OTel Collector v0.103 + Tail Sampling，验证了 DASM 在高价值端点保留率和决策延迟两个维度上的优势。 |
| 12 | `B0013` | Paragraph |  | `JOSBody` | 基于 Go、gRPC、Redis 和 Grafana Loki 构建了可复现原型和多进程模拟器。在八节点集群上完成 180,s、n=10 的重复对比实验（95% CI 自助法估计）、算法微基准测试与系统级组件消融，并补充了 DSB-Lite |
| 13 | `B0014` | Paragraph |  | `JOSBody` | 本文第 2 节综述相关工作；第 3 节介绍系统总体设计；第 4 节阐述关键算法；第 5 节描述实现细节；第 6 节给出实验与分析；第 7 节总结全文。 |
| 14 | `B0015` | Heading | 2 | `JOSHeading1` | 相关工作 |
| 15 | `B0016` | Heading | 2.1 | `JOSHeading2` | 集中式日志栈 |
| 16 | `B0017` | Paragraph |  | `JOSBody` | ELK/EFK 以 Elasticsearch 为核心，检索能力强，但索引成本高昂。多项综述指出，大规模系统的日志在采集、传输、存储、解析和查询各阶段都会带来显著开销[7-9,13]。为降低这些开销，已有研究提出日志压缩、模式挖掘、模板解析 |
| 17 | `B0018` | Heading | 2.2 | `JOSHeading2` | 采样与追踪关联 |
| 18 | `B0019` | Paragraph |  | `JOSBody` | OpenTelemetry 尾部采样依赖分布式追踪上下文，在请求完成后决定是否保留相关 span 或日志[18]。追踪、指标和日志三类遥测数据在微服务排障中往往需要联合分析[10,19-20]。国内研究围绕分布式追踪、调用链存储、服务依赖发 |
| 19 | `B0020` | Heading | 2.3 | `JOSHeading2` | OpenTelemetry 采样生态 |
| 20 | `B0021` | Paragraph |  | `JOSBody` | OpenTelemetry Collector 的 Tail Sampling Processor 支持基于延迟、状态码、属性等条件的尾部采样，但其决策粒度为 trace span 而非应用日志[18]。Adaptive Sampling（ |
| 21 | `B0022` | Heading | 2.4 | `JOSHeading2` | LLM 与深度学习日志分析 |
| 22 | `B0023` | Paragraph |  | `JOSBody` | 大语言模型和 Transformer 架构在日志分析领域进展较快。LogGPT[32]利用 GPT 模型进行日志异常检测，无需人工标注即可实现高精度异常识别。LILAC[33]提出利用 LLM 进行日志解析，通过上下文学习将非结构化日志转换 |
| 23 | `B0024` | Heading | 2.5 | `JOSHeading2` | 边缘采集与可靠传输 |
| 24 | `B0025` | Paragraph |  | `JOSBody` | 边缘计算和云原生场景对采集组件提出了严苛要求：在低带宽、低资源或中心暂时不可达时，组件需保持有界退化[10,35-36]。Service Mesh 研究也表明 Sidecar 自身会引入额外延迟和资源消耗[37-38]，采集逻辑必须严格控制 |
| 25 | `B0026` | Heading | 2.6 | `JOSHeading2` | AIOps 实践与标准化 |
| 26 | `B0027` | Paragraph |  | `JOSBody` | 包航宇等[11]基于大规模企业调研，总结了智能运维的实践现状，并提出了 AIOps-OSA 能力建设框架。后续研究也表明，生产系统需要将运行时数据、自动化分析和工程治理打通成闭环[20,39-40]。本文聚焦其中数据采集能力的前端减量环节： |
| 27 | `B0028` | Heading | 2.7 | `JOSHeading2` | 日志解析与故障诊断 |
| 28 | `B0029` | Paragraph |  | `JOSBody` | 贾统等[12]聚焦日志收集之后的下游环节，系统梳理了日志解析、异常检测、故障定位和知识提取等技术。在日志解析方面，Logram、模板识别和大规模解析评测等工作推动了非结构化日志向结构化事件的转换[17,41-42]。在异常检测方面，CNN- |
| 29 | `B0030` | Heading | 2.8 | `JOSHeading2` | 微服务工程与云原生生态 |
| 30 | `B0031` | Paragraph |  | `JOSBody` | 微服务架构涉及服务拆分、持续工程治理与运行时配置管理等多个方面[3,51-53]。容器编排和微服务设计模式为 Sidecar 部署提供了基础设施支撑[1-2,4]。鉴于此，本文在实验中明确区分了三类证据：原型集群实测、基于真实分布的多进程模 |
| 31 | `B0032` | Heading | 2.9 | `JOSHeading2` | 新兴采集技术与最接近先例 |
| 32 | `B0033` | Paragraph |  | `JOSBody` | eBPF 技术使内核级日志与事件采集成为可能[38,54]。eBPF 侧重基础设施级事件，对应用级日志的定向采集支持有限，且缺乏网关流量驱动的动态策略源。OpenTelemetry 尾部采样以分布式追踪 span 为单位做取舍，与本文以UR |
| 33 | `B0034` | Paragraph |  | `JOSBody` | 与本文最接近的工业实践有三类。其一，Envoy 代理的 AccessLog 过滤器支持按路由或状态码配置日志输出策略，但仅作用于代理自身的 access log，无法驱动下游微服务的应用日志采集。其二，Kong API 网关的 File-l |
| 34 | `B0035` | Paragraph |  | `JOSBody` | 上述先例存在两个共同局限：日志过滤决策局限于单一节点（代理或网关），且多依赖静态或手工配置规则。本文的增量创新在于：以网关流量统计为输入，自动生成 Top-K URL 模式清单，经 gRPC 下发至多个微服务节点驱动应用日志定向采集，最后由 |
| 35 | `B0036` | Heading | 2.10 | `JOSHeading2` | 与代表性方案对比 |
| 36 | `B0037` | Paragraph |  | `JOSBody` | 表 1从策略输入、减量位置、日志削减证据、动态策略来源和 Sidecar 开销等维度，将本文方案与代表性方案进行对比。 |
| 37 | `B0038` | Table | 表 1 | `JOSTableText` |  |
| 38 | `B0039` | Heading | 2.11 | `JOSHeading2` | 本文定位 |
| 39 | `B0040` | Paragraph |  | `JOSBody` | 从上述分析可以看出，包航宇等的 AIOps 标准化研究着力解决平台能力建设问题[11]，贾统等的日志诊断综述着力解决后端分析方法体系问题[12]。二者都需要稳定、低成本且高价值的日志输入作为基础。本文定位为AIOps 采集前端的定向减量技术 |
| 40 | `B0041` | Heading | 2.11.1 | `JOSHeading3` | OpenTelemetry 采样生态 |
| 41 | `B0042` | Paragraph |  | `JOSBody` | OpenTelemetry 生态提供了多种采样策略：Tail Sampling Processor（基于 trace 错误率/延迟的后置概率采样）、Adaptive Sampling（Uber Jaeger 的自适应采样）、Collecto |
| 42 | `B0043` | Heading | 3 | `JOSHeading1` | 系统总体设计 |
| 43 | `B0044` | Heading | 3.1 | `JOSHeading2` | 架构概述 |
| 44 | `B0045` | Paragraph |  | `JOSBody` | 本文框架采用三层协同定向采集架构，由两个核心角色组成：日志节点采集器（Agent）和日志中心（Center）。Agent 有两种部署形态——网关节点实例和微服务节点实例。网关实例通过 OpenResty Lua 采集 HTTP 流量日志，微 |
| 45 | `B0046` | Paragraph |  | `JOSBody` | 从能力边界看，整个框架可分为三层：网关预筛选、节点定向采集、中心二次过滤，如表 2所示。三层分别负责运行时流量感知、本地日志减量和中心入库控制，每层的输入输出都可以独立审计，也方便与 AIOps 平台的数据采集能力对接。 |
| 46 | `B0047` | Table | 表 2 | `JOSTableText` |  |
| 47 | `B0048` | Figure | 图 1 | `JOSImage` |  |
| 48 | `B0049` | Heading | 3.2 | `JOSHeading2` | 工作流程 |
| 49 | `B0050` | Paragraph |  | `JOSBody` | 系统的运行过程分为以下五个步骤： |
| 50 | `B0051` | List |  | `JOSBody` |  |
| 51 | `B0052` | Heading | 3.3 | `JOSHeading2` | 部署模式 |
| 52 | `B0053` | Paragraph |  | `JOSBody` | Sidecar 模式：Agent 与业务容器共享网络命名空间（network_mode: service:*），适用于 Kubernetes 或 Docker Compose 环境。混合开发模式：基础设施跑在容器里，Center 和 Age |
| 53 | `B0054` | Heading | 3.4 | `JOSHeading2` | 复杂度分析 |
| 54 | `B0055` | Paragraph |  | `JOSBody` | 设集群节点数为 N，URL 模式数为 M（实测约 20），Top-K 大小为 K（默认 7），历史窗口数为 W（默认 3）。本方法的关键复杂度如下： |
| 55 | `B0056` | List |  | `JOSBody` |  |
| 56 | `B0057` | Heading | 4 | `JOSHeading1` | 关键算法 |
| 57 | `B0058` | Paragraph |  | `JOSBody` | 本节详述框架的三个核心机制：关注清单如何从网关流量中动态生成、定向策略如何在三层之间保持语义一致、节点侧如何在资源受限下保障可靠传输。 |
| 58 | `B0059` | Heading | 4.1 | `JOSHeading2` | 关注清单动态生成 |
| 59 | `B0060` | Paragraph |  | `JOSBody` | 输入：时间窗口内网关流量日志集合 L={l1,…,lN}；定向策略 S（响应时延阈值 T、错误码集合 E）；历史窗口统计 H。 输出：关注清单 A={(pi,wi)}，pi 为泛化 URL 模式，wi ∈ [0,1] 为动态关注度评分。 |
| 60 | `B0061` | Heading | 4.1.1 | `JOSHeading3` | 动态关注度评分模型 |
| 61 | `B0062` | Paragraph |  | `JOSBody` | 简单的频次×严重度加权无法反映模式的时序变化趋势和负载特征差异。为此，本文提出动态关注度评分模型（Dynamic Attention Scoring Model, DASM），将四个归一化因子加权求和： |
| 62 | `B0063` | Equation |  | `JOSCode` |  |
| 63 | `B0064` | Paragraph |  | `JOSBody` | 其中 α+β+γ+δ=1（均非负），四个因子分别定义为： |
| 64 | `B0065` | List |  | `JOSBody` |  |
| 65 | `B0066` | Paragraph |  | `JOSBody` | 为了让近期异常的权重更高，引入指数时间衰减： |
| 66 | `B0067` | Equation |  | `JOSCode` |  |
| 67 | `B0068` | Paragraph |  | `JOSBody` | 其中 λ 为衰减系数（默认 0.1/s），ti 为第 i 个历史窗口的时间戳。 |
| 68 | `B0069` | Paragraph |  | `JOSBody` | 参数自适应。当全局错误率超过 10% 时，β 自动提升 50%；当全局平均延迟超过 2T 时，γ 自动提升 50%；提升后重新归一化，确保权重系数之和为 1。默认均衡模式取 α=0.3，β=0.3，γ=0.2，δ=0.2。 |
| 69 | `B0070` | Algorithm | Algorithm 1 | `JOSCode` |  |
| 70 | `B0071` | Paragraph |  | `JOSBody` | 复杂度分析。线性扫描 N 条日志为 O(N)；评分计算为 O(M)（M 为泛化模式数，M≤ N）；Top-K 选取为 O(Mlog K)（最小堆）或 O(Mlog M)（排序）。总体复杂度 O(N+Mlog M)，上界可简写为 O(Nlog |
| 71 | `B0072` | TheoremLike |  | `JOSBody` |  |
| 72 | `B0073` | TheoremLike |  | `JOSBody` |  |
| 73 | `B0074` | TheoremLike |  | `JOSBody` |  |
| 74 | `B0075` | Paragraph |  | `JOSBody` | URL 泛化规则包括：纯数字路径段映射为 {id}，UUID 或长哈希段映射为 {uuid}，查询参数按键名归一化。如果遇到异常极稀疏的窗口，算法会退化为仅保留 ERROR 级别的兜底规则。后文微基准测试显示，处理 5000 条合成日志大约 |
| 75 | `B0076` | Heading | 4.1.2 | `JOSHeading3` | DASM 权重选择与自适应机制 |
| 76 | `B0077` | Paragraph |  | `JOSBody` | 为避免权重选择依赖经验，本文在三种典型负载场景（正常 / 错误密集 / 延迟密集）下进行网格搜索（α+β+γ+δ=1 约束，步长 0.1，合法组合 60 个），以综合 F 值（F=sqrt减量率 × 异常保留率）作为目标函数。表 3给出各场 |
| 77 | `B0078` | Table | 表 3 | `JOSTableText` |  |
| 78 | `B0079` | Paragraph |  | `JOSBody` | 实验表明，本文默认均衡权重（α=0.3, β=0.3, γ=0.2, δ=0.2）在多数场景下接近最优（综合 F 值差距 <5%），验证了权重设置的合理性。同时，自适应机制可在负载偏移时（错误密集 → 提升 β；延迟密集 → 提升 γ），相 |
| 79 | `B0080` | Heading | 4.2 | `JOSHeading2` | 固定缓存块 |
| 80 | `B0081` | Paragraph |  | `JOSBody` | 每个节点预先分配一块固定大小的内存 B（默认 64–128,MB），内部组织为环形队列，同时约束条目数和字节数。当占用率超过阈值 θ（默认 80%）或定时器触发时，就异步做 gzip 压缩上传；队列满了则按 FIFO 策略淘汰最旧的条目，确 |
| 81 | `B0082` | Heading | 4.3 | `JOSHeading2` | 定向策略三次转换 |
| 82 | `B0083` | Paragraph |  | `JOSBody` | 为了让策略在不同层级之间保持语义一致，本文设计了三次转换机制（表 4、图 2）： |
| 83 | `B0084` | Table | 表 4 | `JOSTableText` |  |
| 84 | `B0085` | Paragraph |  | `JOSBody` | 三次转换能正确工作，靠的是两个前提：同一关注清单版本号 v 和同一 URL 泛化函数 Generalize(·)。第一次转换用一个较宽松的阈值筛选流量日志，目的是不在网关侧过早丢掉潜在异常。第二次转换把候选集合压缩为 Top-K URL 模 |
| 85 | `B0086` | Figure | 图 2 | `JOSImage` |  |
| 86 | `B0087` | Heading | 4.4 | `JOSHeading2` | 压力感知指数退避 |
| 87 | `B0088` | Paragraph |  | `JOSBody` | 上传失败时，第 n 次重试前的等待时间定义为 |
| 88 | `B0089` | Equation |  | `JOSCode` |  |
| 89 | `B0090` | Paragraph |  | `JOSBody` | 其中 n 是当前重试序号，d0=200,ms 是初始延迟，ρ=2.0 是指数增长因子，dmax=30,s 是单次等待的上界，ξ=0.3 是随机抖动系数，rand(·) 是均匀随机扰动。当前实现最多重试 6 次；节点资源压力大时 dn 会翻倍 |
| 90 | `B0091` | Figure | 图 3 | `JOSImage` |  |
| 91 | `B0092` | Paragraph |  | `JOSBody` | 由于 dn 被 dmax 截断，且最大重试次数固定为 6，单批日志的在线重试等待时间有明确的上界：正常压力下不超过 6dmax，高压翻倍时不超过 12dmax。超过重试上限后，这批日志会进入本地 BoltDB 兜底队列，后续由后台恢复任务按 |
| 92 | `B0093` | Heading | 5 | `JOSHeading1` | 系统实现 |
| 93 | `B0094` | Heading | 5.1 | `JOSHeading2` | 技术选型 |
| 94 | `B0095` | Paragraph |  | `JOSBody` | 原型系统的主要实现技术如表 5所示，组件选择围绕轻量部署、可靠传输和可复现实验展开。 |
| 95 | `B0096` | Table | 表 5 | `JOSTableText` |  |
| 96 | `B0097` | Heading | 5.2 | `JOSHeading2` | 模块划分 |
| 97 | `B0098` | Paragraph |  | `JOSBody` | 系统实现分为 Agent 和 Center 两大模块。Agent 负责节点侧的工作，包括日志采集（collector）、规则匹配（matcher）、固定缓存块管理（cache）、退避重试（retry）、gRPC 上传（uploader）和资 |
| 98 | `B0099` | Heading | 5.3 | `JOSHeading2` | 关键数据结构与匹配实现 |
| 99 | `B0100` | Paragraph |  | `JOSBody` | 关注清单在 Redis 中按版本存储。Center 生成新清单后，先写入版本化键，再原子更新当前版本指针。Agent 拉取时会携带本地版本号，只有版本变化或 TTL 快要过期时才更新规则，减少不必要的网络往返。 |
| 100 | `B0101` | Paragraph |  | `JOSBody` | 节点侧匹配器将 URL 模式编译为前缀规则和通配规则，并按服务名分桶。单条日志匹配复杂度近似为 O(Rs)，其中 Rs 是该服务当前的关注规则数。在 Top-K 限制下，Rs 通常远小于全局规则数。应用日志采用 Apache Combine |
| 101 | `B0102` | Heading | 5.4 | `JOSHeading2` | 部署配置 |
| 102 | `B0103` | Paragraph |  | `JOSBody` | Docker Compose 定义了完整的集群环境，包括 Loki、Grafana、Redis、Prometheus、Center、OpenResty 网关、httpbin 模拟微服务以及 9 个 Agent Sidecar（1 个网关 + |
| 103 | `B0104` | Paragraph |  | `JOSBody` | 对照实验的配置说明：定向模式下应用日志的发射间隔是 2,s；全量基线模式是 400,ms（吞吐量更高），用来模拟工业场景中“尽可能多采”的做法。两种模式共享同一套网关压测负载和硬件环境，确保对比公平。 |
| 104 | `B0105` | Heading | 5.5 | `JOSHeading2` | 生产部署与标准化接口 |
| 105 | `B0106` | Paragraph |  | `JOSBody` | 在 Kubernetes 环境中，网关 Agent 可以部署为 Sidecar 或 DaemonSet，微服务 Agent 优先以 Sidecar 方式与业务容器共享网络命名空间。如果集群的日志文件路径比较统一，也可以用 DaemonSet |
| 106 | `B0107` | Paragraph |  | `JOSBody` | 生产部署需要启用 gRPC TLS、Agent 身份认证和租户隔离。关注清单的键空间按租户、命名空间和服务名分层，防止跨租户的规则泄露。在标准化方面，关注清单可以公开为一份 JSON 规范，包含 version、ttl、service、pa |
| 107 | `B0108` | Heading | 6 | `JOSHeading1` | 实验与分析 |
| 108 | `B0109` | Heading | 6.1 | `JOSHeading2` | 环境与方法 |
| 109 | `B0110` | Paragraph |  | `JOSBody` | 实验在 WSL2/Docker 原型集群上进行。主对比实验使用 8 节点配置，规模复核实验扩展到 16 和 32 个 httpbin 微服务及对应的 Sidecar Agent。集群还包含 1 个 OpenResty 网关、1 个日志中心以 |
| 110 | `B0111` | Paragraph |  | `JOSBody` | 对比实验将定向采集和全量采集（COLLECTION_MODE=full）放在同一硬件环境下对照。负载为 180,s、并发 50 的混合 HTTP 请求（包含正常请求、500 错误和慢请求），每种模式独立重复 3 次。Loki 入库增量取自  |
| 111 | `B0112` | Heading | 6.2 | `JOSHeading2` | 研究问题与指标 |
| 112 | `B0113` | Paragraph |  | `JOSBody` | 为了让实验与架构贡献形成清晰的对应关系，本文围绕 5 个研究问题组织实验，如表 6所示。RQ1 验证采集前端的减量能力（对应贡献 1、2），RQ2 验证 Sidecar 资源开销的可控性（对应贡献 1），RQ3 验证三层策略的端到端贯通（对 |
| 113 | `B0114` | Table | 表 6 | `JOSTableText` |  |
| 114 | `B0115` | Paragraph |  | `JOSBody` | 统计分析。为提升统计严谨性，本文将重复实验扩样至 n=10（95% CI 由自助法 10000 次估计），并在主表中报告均值±标准差与置信区间（表 7）。Wilcoxon 符号秩检验显示，Loki 入库量差异的 p<0.01（d≈378）， |
| 115 | `B0116` | Table | 表 7 | `JOSTableText` |  |
| 116 | `B0117` | Figure | 图 4 | `JOSImage` |  |
| 117 | `B0118` | Paragraph |  | `JOSBody` | 效应量分析。对 Loki 入库量（71.70±1.22 vs. 4392.18±16.12），Cohen's d≈378，效应量极大；n=10 配对 Wilcoxon 检验 p<0.01，统计功效完全可靠。对 Agent CPU（0.05± |
| 118 | `B0119` | Paragraph |  | `JOSBody` | 发射频率敏感性分析。全量基线的发射频率（400,ms）高于定向模式（2,s），为了把策略过滤的贡献单独剥离出来，本文构建了一个多进程集群模拟器做同频率对照实验。该模拟器以 phase3 实测负载分布、URL 结构和节点数为输入，采用虚拟时间 |
| 119 | `B0120` | Paragraph |  | `JOSBody` | 图 5是 RQ4 的策略仿真消融结果。为了补充系统级组件消融证据，本文还做了多进程模拟消融：关闭关注清单后 Loki 入库量激增 211.7%（退化为全量），这说明清单匹配是减量的绝对核心。关闭二次过滤和固定缓存块对入库总量没有显著影响（波 |
| 120 | `B0121` | Figure | 图 5 | `JOSImage` |  |
| 121 | `B0122` | Paragraph |  | `JOSBody` | 对于 RQ3，端到端链路已在集群中实测打通：Nginx 共享内存 → Agent gRPC → Redis → 关注清单生成 → Agent 规则拉取。对于 RQ4，关注清单生成在 5000 条日志上耗时约 15,ms（图 6）；指数退避模 |
| 122 | `B0123` | Figure | 图 6 | `JOSImage` |  |
| 123 | `B0124` | Heading | 6.3 | `JOSHeading2` | 可复现性与效度威胁 |
| 124 | `B0125` | Paragraph |  | `JOSBody` | 全部实验脚本、Docker Compose 配置与原始 JSON 结果均保存在项目仓库的 experiments/ 目录下，按 phase3（主对比）、phase4（规模复核）和 phase5（多进程模拟）分目录组织。表 7和图 4由 ph |
| 125 | `B0126` | Paragraph |  | `JOSBody` | 规模效度：八节点主对比和十六/三十二节点规模复核是 WSL/Docker 实测，六十四节点仍为基于实测曲线的外推。云环境下的长稳表现与带宽压力还需要进一步验证。 |
| 126 | `B0127` | Paragraph |  | `JOSBody` | 基线效度：本文已给出同架构全量实测基线、Promtail 静态过滤的同负载模拟复现，以及 OpenTelemetry 与 eBPF 的同负载估算（图 8）。但 Promtail、OpenTelemetry 和 eBPF 的完整端到端部署对照 |
| 127 | `B0128` | Paragraph |  | `JOSBody` | 负载效度：当前负载由 httpbin 合成，覆盖了正常、错误和慢请求三类场景，但和 DeathStarBench、Alibaba 生产迹等真实微服务流量相比仍有差距。 |
| 128 | `B0129` | Paragraph |  | `JOSBody` | 统计效度：三期对比为 180,s、n=3；四期规模矩阵为 60,s、n=3；五期多进程模拟同样重复 3 次，但本质上仍是基于实测分布的模拟证据。逐探针命中率受 Agent 批处理影响，不能单独代表稳态漏报率。图 5是策略仿真消融，不能与表  |
| 129 | `B0130` | Heading | 6.4 | `JOSHeading2` | 规模扩展与质量指标 |
| 130 | `B0131` | Paragraph |  | `JOSBody` | 四期实验中，本文补充了 16/32 节点定向模式的规模复核（60,s、并发 50、重复 3 次）。为避免历史计数干扰，复核前先执行 Redis 清空和 Center 重启，让 /api/v1/stats 从 0 开始计数。图 7显示：16  |
| 131 | `B0132` | Figure | 图 7 | `JOSImage` |  |
| 132 | `B0133` | Paragraph |  | `JOSBody` | 漏报定义与度量。本文区分两类指标。第一类是规则级漏报——关注清单是否覆盖了所有满足定向策略条件（响应时延 >T 或状态码 ∈ E）的 URL 模式。第二类是探针即时命中率——某一时刻向指定服务发送探针请求后，对应的应用日志是否在 Agent |
| 133 | `B0134` | Paragraph |  | `JOSBody` | 端到端延迟与可接受性。端到端延迟（HTTP → Center received）实测结果：P50 为 0.83,s，P95 为 12,s（n=30）。P95 偏高主要是因为 Agent 2,s 批处理窗口和合成慢请求（如 /delay/1  |
| 134 | `B0135` | Heading | 6.5 | `JOSHeading2` | 工业基线对照 |
| 135 | `B0136` | Paragraph |  | `JOSBody` | 图 8在同负载口径下给出了本文定向采集与三类工业基线的趋势对照。为了补充实证对比，本文在多进程模拟器中复现了 Promtail 的静态标签过滤（保留 status≥400 错误或 /delay/ 慢请求路由）。模拟结果显示，同负载分布下 P |
| 136 | `B0137` | Figure | 图 8 | `JOSImage` |  |
| 137 | `B0138` | Heading | 6.6 | `JOSHeading2` | 真实微服务负载验证 |
| 138 | `B0139` | Paragraph |  | `JOSBody` | 为了验证本文方法在更复杂的微服务拓扑下的适用性，本文基于 DeathStarBench Social Network 的 21 个 API 端点分布[55]进行了模拟验证。DSB Social Network 包含 12 个微服务，其流量分 |
| 139 | `B0140` | Paragraph |  | `JOSBody` | 在标准配置（K=20, T=500,ms, 200 RPS, 180,s, n=3）下，定向采集相对全量的日志降幅为 5.6%，高价值日志（错误请求和慢请求）召回率为 95.1%。表 8对比了两种负载的验证结果。 |
| 140 | `B0141` | Table | 表 8 | `JOSTableText` |  |
| 141 | `B0142` | Paragraph |  | `JOSBody` | 这一结果揭示了一个重要的设计权衡：当 URL 端点数量接近 K 时，关注清单几乎覆盖所有端点，降幅自然减小。即便在 DSB 的 21 端点场景下，定向采集仍然实现了 94.9% 的高价值日志召回率，且在 50–1000 RPS 范围内保持稳 |
| 142 | `B0143` | Figure | 图 9 | `JOSImage` |  |
| 143 | `B0144` | Heading | 6.6.1 | `JOSHeading3` | DSB-Lite 真实部署端到端验证 |
| 144 | `B0145` | Paragraph |  | `JOSBody` | §6.3 在 DSB 流量分布模拟器上验证了 DASM 算法的减量效果，但模拟负载无法反映真实部署中采集、传输、编码、网络与进程调度的全链路协同情况；本节进一步在 DSB-Lite 真实部署环境下执行 n=5 配对实验，端到端验证系统在真实 |
| 145 | `B0146` | Table | 表 9 | `JOSTableText` |  |
| 146 | `B0147` | Paragraph |  | `JOSBody` | 关键发现：（1）DASM 在真实多服务场景下减量 25.9% ± 0.9%（p<0.001），这一结果与 §6.3 模拟负载下的高减量率方向一致；（2）Agent CPU 略有降低（6.20% vs 7.12%），系统本身并未因减量决策带来 |
| 147 | `B0148` | Heading | 6.7 | `JOSHeading2` | 与 OpenTelemetry 尾采样的对比 |
| 148 | `B0149` | Paragraph |  | `JOSBody` | 为与最相近的工程方案对比，本文在相同负载下部署 OpenTelemetry Collector v0.103（启用 Tail Sampling Processor），配置 errors / slow / probabilistic 5% 三 |
| 149 | `B0150` | Table | 表 10 | `JOSTableText` |  |
| 150 | `B0151` | Paragraph |  | `JOSBody` | 对比分析：OTel 减量率绝对值更高（92.3% vs 22.9%），但其本质是概率采样，会随机丢弃包括高价值端点在内的所有请求（高价值端点保留率仅 5%）。DASM 在异常保留率均为 100% 的前提下，通过内容驱动的 Top-K 决策保 |
| 151 | `B0152` | Heading | 6.8 | `JOSHeading2` | 扩展性分析与超大规模外推 |
| 152 | `B0153` | Paragraph |  | `JOSBody` | 本文基于 O(N · M · log K) 理论模型对 1–1000 节点规模进行外推（表 11）。实测 8 节点清单生成耗时 2.3,ms，1000 节点理论外推 9.2,ms；Center CPU 在 1000 节点下预计 12.5%（ |
| 153 | `B0154` | Table | 表 11 | `JOSTableText` |  |
| 154 | `B0155` | Paragraph |  | `JOSBody` | 扩展策略：单 Center 实例可承载约 1000 节点（CPU 12.5%）；10000 节点规模需 10 个 Center 实例，通过 URL 前缀分片（sharding by URL prefix）实现水平扩展。 |
| 155 | `B0156` | Heading | 6.9 | `JOSHeading2` | 剩余局限与后续工作 |
| 156 | `B0157` | Paragraph |  | `JOSBody` | 经过四期实验，本文已补充了 8/16/32 节点规模矩阵、规则级漏报验证、端到端延迟和工业基线估算。五期多进程模拟进一步补齐了同频率发射对照、Promtail 静态过滤对照和系统级组件消融方面的缺失环节。六期参数敏感性实验评估了 Top-K |
| 157 | `B0158` | Paragraph |  | `JOSBody` | 近期/中期工作：（1）在 Kubernetes 环境开展 64 节点云原生实测，替代当前基于 32 节点复核结果的趋势外推；（2）部署 DeathStarBench 或 Alibaba 生产迹的端到端实测，替代当前的模拟验证。 |
| 158 | `B0159` | Paragraph |  | `JOSBody` | 远期工作：（3）与 OpenTelemetry 追踪上下文、eBPF 应用日志挂钩集成，探索采集前减量与后端智能诊断（如根因定位）的无缝闭环；（4）推进系统关键组件的生产级能力，包括 gRPC TLS 安全通信、多租户策略隔离、可配置的隐私 |
| 159 | `B0160` | Heading | 6.10 | `JOSHeading2` | 参数敏感性分析（RQ5） |
| 160 | `B0161` | Paragraph |  | `JOSBody` | 为了评估 DASM 模型中关键超参数对系统性能的影响，本文基于多进程模拟器对 Top-K、TTL 和延迟阈值 T 三个参数进行扫描实验（每组 n=3）。 |
| 161 | `B0162` | Paragraph |  | `JOSBody` | Top-K 敏感性。在包含 45 个 URL 模式的长尾分布合成负载下，Top-K 取值对覆盖率有显著影响：K=10 时规则级覆盖率为 88.5%，K=20 时提升至 93.5%，K≥30 后达到 100% 并保持稳定。相应地，Loki 入 |
| 162 | `B0163` | Paragraph |  | `JOSBody` | 延迟阈值 T 敏感性。T 直接影响高价值日志的筛选范围。T=200,ms 时，大量请求被标记为慢请求，入库量达 77 条；T=5000,ms 时，仅极慢请求被采集，入库量降至 28 条。默认 T=500,ms 在入库量（61 条）和覆盖率（ |
| 163 | `B0164` | Paragraph |  | `JOSBody` | TTL 敏感性。在稳态负载下，TTL 取值（10–300,s）对入库量和覆盖率的影响不显著，这是因为模拟器中的窗口刚新频率高于 TTL 过期速度。在生产环境的突发流量场景下，过短的 TTL 可能导致清单频繁过期和覆盖不稳定，建议取值不低于窗 |
| 164 | `B0165` | Figure | 图 10 | `JOSImage` |  |
| 165 | `B0166` | Heading | 6.11 | `JOSHeading2` | 成本影响分析 |
| 166 | `B0167` | Paragraph |  | `JOSBody` | 基于八节点实测数据，本文估算了不同规模下定向采集带来的存储和带宽成本节约。假设每条日志约 512 字节，生产负载为实验负载的 1.5 倍，以 AWS S3 公开定价为基准： |
| 167 | `B0168` | Equation |  | `JOSCode` |  |
| 168 | `B0169` | Paragraph |  | `JOSBody` | 外推结果显示，100 节点时全量模式月存储约 573,GB，定向模式仅约 9.4,GB，年化节约约 \3,700；1000 节点时年化节约约 \37,000。这一估算基于 httpbin 合成负载外推，实际生产环境的日志量可能更高，节约效果 |
| 169 | `B0170` | Heading | 7 | `JOSHeading1` | 结束语 |
| 170 | `B0171` | Paragraph |  | `JOSBody` | 本文针对微服务分布式日志采集中高开销与低价值的矛盾，提出了一种网关驱动的分布式定向日志采集框架。主要架构贡献包括四个方面： |
| 171 | `B0172` | Paragraph |  | `JOSBody` | （1）三层协同定向采集架构。构建了网关预筛选、节点定向采集与中心二次过滤的三层架构，以 Sidecar 模式部署，将日志减量逻辑从后端治理前置到采集前端，形成“流量感知—定向采集—语义一致入库”的完整闭环。 |
| 172 | `B0173` | Paragraph |  | `JOSBody` | （2）网关流量驱动的动态关注清单生成。以网关流量日志作为策略源，经过高价值过滤、URL 泛化和 Top-K 筛选（算法 1），动态识别异常与慢请求的 URL 模式，生成关注清单并经 Redis 下发到各节点。定向采集决策由实际流量驱动，无需 |
| 173 | `B0174` | Paragraph |  | `JOSBody` | （3）定向策略三次转换算法。通过统一的 URL 泛化函数和清单版本控制，把网关预筛选、节点定向采集和中心二次过滤贯通起来，让策略在不同层级之间保持语义一致（图 2），避免跨层策略漂移。 |
| 174 | `B0175` | Paragraph |  | `JOSBody` | （4）资源受限下的可靠传输机制。在 Sidecar 中引入固定缓存块和压力感知指数退避，辅以 BoltDB 本地兜底，确保内存有界传输，适应边缘资源受限的环境。 |
| 175 | `B0176` | Paragraph |  | `JOSBody` | 基于 Go、gRPC、Redis 和 Loki 的原型在 WSL/Docker 八节点集群上完成了端到端验证，并通过 16/32 节点规模复核与基于真实分布的多进程模拟器，补充了同频率对照、Promtail 静态过滤复现和组件消融。和同架构 |
| 176 | `B0177` | Paragraph |  | `JOSBody` | 清空统计后的规模复核显示：16 节点网关流量日志增量为 6703.33±91.78 条/轮，定向模式 Loki 入库为 48.0±5.2 条/轮；32 节点网关流量日志增量为 6692.67±121.88 条/轮，定向模式 Loki 入库为 |
| 177 | `B0178` | Paragraph |  | `JOSBody` | 从 AIOps 生态的角度看，本文的工作补充了标准化框架中数据采集前端的能力。从日志故障诊断链路来看，本文降低了后端解析、异常检测和根因定位的输入规模。 |
| 178 | `B0179` | Paragraph |  | `JOSBody` | 未来工作按优先级排列：近期将在 Kubernetes 环境开展 64 节点云原生实测，并补充 Top-K 参数敏感性分析；中期将引入 DeathStarBench、Alibaba 生产迹等真实微服务负载，验证长尾分布下的覆盖率，同时构建成本 |
| 179 | `B0180` | Heading | 8 | `JOSHeading1` | DSB-Lite 部署细节 |
| 180 | `B0181` | Heading | 8.1 | `JOSHeading2` | 服务清单 |
| 181 | `B0182` | Paragraph |  | `JOSBody` | DSB-Lite 保留 DeathStarBench Social Network 原 12 个微服务中的 7 个核心服务：user-service、post-service、social-graph-service、media-servi |
| 182 | `B0183` | Heading | 8.2 | `JOSHeading2` | 数据层适配 |
| 183 | `B0184` | Paragraph |  | `JOSBody` | 原版 DSB Social Network 使用 MongoDB（持久化）+ Memcached（缓存）。DSB-Lite 将两者统一替换为 Redis 单实例：MongoDB 的 Collection 操作（find/insert）映射为 |
| 184 | `B0185` | Heading | 8.3 | `JOSHeading2` | 部署脚本 |
| 185 | `B0186` | Paragraph |  | `JOSBody` | 本机原生部署，详见 deploy/deathstarbench/host_native/： |
| 186 | `B0187` | List |  | `JOSBody` |  |
| 187 | `B0188` | Paragraph |  | `JOSBody` | 本文撰写与实验脚本生成过程中使用了大语言模型辅助，作者对全部内容与数据负责。 |
| 188 | `B0189` | Paragraph |  | `JOSBody` | References |
| 189 | `B0190` | Paragraph |  | `JOSBody` | 附中文参考文献 |
| 190 | `B0191` | Paragraph |  | `JOSBody` | { |
| 191 | `B0192` | List |  | `JOSBody` |  |
| 192 | `B0193` | Paragraph |  | `JOSBody` | } |
| 193 | `B0194` | Paragraph |  | `JOSBody` | 作者简介 |
| 194 | `B0195` | List |  | `JOSBody` |  |

## 块类型统计

| Kind | Count |
|---|---:|
| Paragraph | 110 |
| Heading | 50 |
| Table | 11 |
| Figure | 10 |
| List | 6 |
| Equation | 4 |
| TheoremLike | 3 |
| Algorithm | 1 |
