# DOCX 内容与格式对比报告

## 输入

- Left: `examples/paper3/output/main-jos-rust.docx`
- Right: `examples/paper3/output/to-docx/v12-论文稿件-jos-sh-20260618-070357.docx`

## 摘要

| 指标 | Left | Right | Delta |
| --- | ---: | ---: | ---: |
| 段落数 | 702 | 658 | -44 |
| 表格数 | 12 | 12 | 0 |
| 图片 drawing 数 | 10 | 10 | 0 |
| media 文件数 | 10 | 10 | 0 |

- 相同段落：487
- 近似修改段落：43
- 新增段落：128
- 删除段落：172
- 格式变更段落：2
- 真实格式差异段落：0
- run 分割差异段落（可忽略）：2
- document.xml 规范化 hash 相同：false
- styles.xml 规范化 hash 相同：false

## 段落样式分布

### Left

| 样式 | 段落数 |
| --- | ---: |
| (none) | 78 |
| JOSAbstractEn | 1 |
| JOSAbstractZh | 1 |
| JOSAuthorZh | 1 |
| JOSBody | 115 |
| JOSBodyNoIndent | 1 |
| JOSCaption | 21 |
| JOSCitation | 5 |
| JOSCode | 4 |
| JOSEnglishTitle | 1 |
| JOSHeading1 | 8 |
| JOSHeading2 | 38 |
| JOSHeading3 | 4 |
| JOSImage | 10 |
| JOSInstituteZh | 2 |
| JOSKeywords | 2 |
| JOSReference | 80 |
| JOSReferenceHeading | 3 |
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
| Modified | 5 | 5 | 0.992 | JOSAbstractZh | JOSAbstractZh | L: 摘 要: 微服务架构下，日志来源高度分散、格式各异，总量随并发请求线性增长，节点资源、网络带宽和集中存储的压力随之显著增加。传统全量采集模式难以兼顾高价值日志保留与系统开销控制。本文提出一种基于动态关注清单的微服务日志定向采集方法，核心思路是“只采必要日志”。主要贡献包括四个方面：（1）构建网关预筛选、节点定向采集与中心二次过滤的三层协同架构，以 Sidecar 模式部署，从源头实现采集前端减量；（2）提出动态关注度评分模型（DASM）...<br>R: 摘 要: 微服务架构下，日志来源高度分散、格式各异，总量随并发请求线性增长，节点资源、网络带宽和集中存储的压力随之显著增加。传统全量采集模式难以兼顾高价值日志保留与系统开销控制。本文提出一种基于动态关注清单的微服务日志定向采集方法，核心思路是“只采必要日志”。主要贡献包括四个方面：（1）构建网关预筛选、节点定向采集与中心二次过滤的三层协同架构，以 Sidecar 模式部署，从源头实现采集前端减量；（2）提出动态关注度评分模型（DASM）... |
| Delete | 8 | - | - | JOSCitation | - | 石洪雷,赵涓涓.基于动态关注清单的微服务日志定向采集方法.软件学报. http://www.jos.org.cn/1000-9825/0000.htm |
| Delete | 9 | - | - | JOSCitation | - | Shi HL, Zhao JJ. Dynamic attention list-based directed log collection method for microservices. Ruan |
| Delete | 10 | - | - | JOSCitation | - | Jian Xue Bao/Journal of Software (in Chinese). http://www.jos.org.cn/1000-9825/0000.htm |
| Insert | - | 8 | - | - | JOSCitation | 中文引用格式: 石洪雷, 赵涓涓. 基于动态关注清单的微服务日志定向采集方法. 软件学报. |
| Insert | - | 9 | - | - | JOSCitation | http://www.jos.org.cn/1000-9825/0000.htm |
| Insert | - | 10 | - | - | JOSCitation | 英文引用格式: Shi HL, Zhao JJ. Dynamic attention list-based directed log collection method for |
| Insert | - | 11 | - | - | JOSCitation | microservices. Ruan Jian Xue Bao/Journal of Software (in Chinese). http://www.jos.org.cn/1000-9825/0000.htm |
| Modified | 14 | 15 | 0.997 | JOSAbstractEn | JOSAbstractEn | L: Abstract: In microservice architectures, log sources are highly distributed with heterogeneous formats, and their aggregate volume grows linearly with request concurrency, imposing significant overhead on node resources,...<br>R: Abstract: In microservice architectures, log sources are highly distributed with heterogeneous formats, and their aggregate volume grows linearly with request concurrency, imposing significant overhead on node resources,... |
| Modified | 21 | 22 | 1.000 | JOSBody | JOSBody | L: 基于上述问题与假设，本文提出分布式定向日志采集框架（Distributed Directed Log Collection Framework）。框架分为三层：网关预筛选、节点定向采集、中心二次过滤，三层之间通过三次策略转换保持语义一致。和 Promtail/Fluent Bit “全量推送后在存储端压缩”的传统方案相比，本文直接在采集前端就把 Loki 入库量控制在百条量级（八节点实测详见§6.3），从源头降低了网络和存储开销。主要架...<br>R: 基于上述问题与假设，本文提出分布式定向日志采集框架（Distributed Directed Log Collection Framework）。框架分为三层：网关预筛选、节点定向采集、中心二次过滤，三层之间通过三次策略转换保持语义一致。和 Promtail/Fluent Bit “全量推送后在存储端压缩”的传统方案相比，本文直接在采集前端就把 Loki 入库量控制在百条量级（八节点实测详见 §6.3），从源头降低了网络和存储开销。主要... |
| Modified | 23 | 24 | 0.980 | JOSBody | JOSBody | L: （2）动态关注度评分模型（DASM）。提出以频次、错误率、延迟严重度和热度趋势四个归一化因子加权求和的动态评分机制（式(1)），结合指数时间衰减（式(2)）和负载自适应权重调节，实时生成高价值 URL 模式清单并下发至各节点（算法 1，排序实现复杂度上界为 O(N log N)，最小堆实现为 O(N log K)，K ≪ N）。和简单的频次×严重度加权相比，DASM 能够感知模式的时序变化趋势并自动适应不同负载特征，同时满足评分有界性（...<br>R: （2）动态关注度评分模型（DASM）。提出以频次、错误率、延迟严重度和热度趋势四个归一化因子加权求和的动态评分机制（式 (1)），结合指数时间衰减（式 (2)）和负载自适应权重调节，实时生成高价值 URL 模式清单并下发至各节点（算法 1，排序实现复杂度上界为 O(N log N)，最小堆实现为 O(N log K)，K ≪ N）。和简单的频次×严重度加权相比，DASM 能够感知模式的时序变化趋势并自动适应不同负载特征，同时满足评分有界... |
| Delete | 25 | - | - | JOSBody | - | （4）资源受限环境下的可靠传输架构。在 Sidecar 中引入固定缓存块（环形队列+双约束）和压力感知指数退避机制（式(3)），辅以 BoltDB 本地兜底，在资源压力下提供有界内存占用和有界重试能力。 |
| Delete | 26 | - | - | JOSBody | - | （5）真实部署验证。在 8 微服务 DSB-Lite 真实部署环境下，n=5 配对实验显示 DASM 减量 25.9%± 0.9%（p<0.001），且不引入额外延迟开销。 |
| Insert | - | 26 | - | - | JOSBody | （4）资源受限环境下的可靠传输架构。在 Sidecar 中引入固定缓存块（环形队列+双约束）和压力感知指数退避机制（式 (3)），辅以 BoltDB 本地兜底，在资源压力下提供有界内存占用和有界重试能力。 |
| Insert | - | 27 | - | - | JOSBody | （5）真实部署验证。在 8 微服务 DSB-Lite 真实部署环境下，n=5 配对实验显示 DASM 减量 25.9% ± 0.9%（p<0.001），且不引入额外延迟开销。 |
| Modified | 28 | 29 | 1.000 | JOSBody | JOSBody | L: 基于 Go、gRPC、Redis 和 Grafana Loki 构建了可复现原型和多进程模拟器。在八节点集群上完成 180 s、n=10 的重复对比实验（95% CI 自助法估计）、算法微基准测试与系统级组件消融，并补充了 DSB-Lite 真实部署验证、OpenTelemetry 尾采样对比、1–1000 节点扩展性外推。结果显示，定向采集将 Loki 入库量从 4388±1 条降至 72 条（降幅 98.4%，策略过滤独立贡献 67...<br>R: 基于 Go、gRPC、Redis 和 Grafana Loki 构建了可复现原型和多进程模拟器。在八节点集群上完成 180 s、n=10 的重复对比实验（95% CI 自助法估计）、算法微基准测试与系统级组件消融，并补充了 DSB-Lite 真实部署验证、OpenTelemetry 尾采样对比、1–1000 节点扩展性外推。结果显示，定向采集将 Loki 入库量从 4388±1 条降至 72 条（降幅 98.4%，策略过滤独立贡献 67... |
| Modified | 48 | 49 | 1.000 | JOSBody | JOSBody | L: eBPF 技术使内核级日志与事件采集成为可能[38,54]。eBPF 侧重基础设施级事件，对应用级日志的定向采集支持有限，且缺乏网关流量驱动的动态策略源。OpenTelemetry 尾部采样以分布式追踪 span 为单位做取舍，与本文以 URL 模式为粒度的应用日志定向过滤构成互补关系。<br>R: eBPF 技术使内核级日志与事件采集成为可能[38,54]。eBPF 侧重基础设施级事件，对应用级日志的定向采集支持有限，且缺乏网关流量驱动的动态策略源。OpenTelemetry 尾部采样以分布式追踪 span 为单位做取舍，与本文以URL 模式为粒度的应用日志定向过滤构成互补关系。 |
| Modified | 50 | 51 | 1.000 | JOSBody | JOSBody | L: 上述先例存在两个共同局限：日志过滤决策局限于单一节点（代理或网关），且多依赖静态或手工配置规则。本文的增量创新在于：以网关流量统计为输入，自动生成 Top-K URL 模式清单，经 gRPC 下发至多个微服务节点驱动应用日志定向采集，最后由中心二次过滤保障语义一致。这一流程形成了“网关流量→多节点应用日志动态清单→中心二次过滤”的贯通闭环，以规则驱动的轻量匹配替代 ML 推理，更适合边缘资源受限环境。<br>R: 上述先例存在两个共同局限：日志过滤决策局限于单一节点（代理或网关），且多依赖静态或手工配置规则。本文的增量创新在于：以网关流量统计为输入，自动生成 Top-K URL 模式清单，经 gRPC 下发至多个微服务节点驱动应用日志定向采集，最后由中心二次过滤保障语义一致。这一流程形成了“网关流量 → 多节点应用日志动态清单 → 中心二次过滤”的贯通闭环，以规则驱动的轻量匹配替代 ML 推理，更适合边缘资源受限环境。 |
| Modified | 118 | 119 | 1.000 | JOSBody | JOSBody | L: 从上述分析可以看出，包航宇等的 AIOps 标准化研究着力解决平台能力建设问题[11]，贾统等的日志诊断综述着力解决后端分析方法体系问题[12]。二者都需要稳定、低成本且高价值的日志输入作为基础。本文定位为 AIOps 采集前端的定向减量技术：将网关 access log 作为动态策略输入，下发至各微服务节点执行应用日志定向采集，再由中心二次过滤后入库。借助关注清单、三次策略转换与可复现的 Docker 原型，本文在采集阶段将入库量控制...<br>R: 从上述分析可以看出，包航宇等的 AIOps 标准化研究着力解决平台能力建设问题[11]，贾统等的日志诊断综述着力解决后端分析方法体系问题[12]。二者都需要稳定、低成本且高价值的日志输入作为基础。本文定位为AIOps 采集前端的定向减量技术：将网关 access log 作为动态策略输入，下发至各微服务节点执行应用日志定向采集，再由中心二次过滤后入库。借助关注清单、三次策略转换与可复现的 Docker 原型，本文在采集阶段将入库量控制在... |
| Insert | - | 120 | - | - | JOSBody | *八节点、180 s、n=3；全量基线发射频率（400 ms）高于定向模式（2 s），降幅为策略过滤与源配置联合效果；基于真实分布的多进程同频模拟表明，策略过滤独立降幅为 67.8%（§6.3）。 |
| Modified | 120 | 122 | 1.000 | JOSBody | JOSBody | L: OpenTelemetry 生态提供了多种采样策略：Tail Sampling Processor（基于 trace 错误率/延迟的后置概率采样）、Adaptive Sampling（Uber Jaeger 的自适应采样）、Collector Filter Processor（基于属性的预过滤）。本文与 OTel 采样策略的核心区别在于：OTel 侧重于追踪（Trace）维度的后置采样决策，本文聚焦于日志（Log）维度的内容驱动定向减量...<br>R: OpenTelemetry 生态提供了多种采样策略：Tail Sampling Processor（基于 trace 错误率/延迟的后置概率采样）、Adaptive Sampling（Uber Jaeger 的自适应采样）、Collector Filter Processor（基于属性的预过滤）。本文与 OTel 采样策略的核心区别在于：OTel 侧重于追踪（Trace）维度的后置采样决策，本文聚焦于日志（Log）维度的内容驱动定向减量... |
| Modified | 155 | 157 | 0.940 | JOSBody | JOSBody | L: itemize 时间复杂度：清单生成 O(M log K) per node；Center 聚合 O(N · M · log K)；在 1000 节点规模下理论耗时< 50 ms（实测 8 节点 2.3 ms）。空间复杂度：O(N · W · M)，与节点数线性相关；1000 节点× 20 模式× 3 窗口× 64 B ≈ 3.7 MB，完全可承载于单机内存。通信复杂度：Agent → Center 增量同步 O(M) per node...<br>R: itemize 时间复杂度：清单生成 O(M log K) per node；Center 聚合 O(N · M · log K)；在 1000 节点规模下理论耗时 < 50 ms（实测 8 节点 2.3 ms）。 空间复杂度：O(N · W · M)，与节点数线性相关；1000 节点 × 20 模式 × 3 窗口 × 64 B approx 3.7 MB，完全可承载于单机内存。 通信复杂度：Agent to Center 增量同步 O... |
| Modified | 159 | 161 | 0.926 | JOSBody | JOSBody | L: 输入：时间窗口内网关流量日志集合 L={l1,…,lN}；定向策略 S（响应时延阈值 T、错误码集合 E）；历史窗口统计 ℋ。 输出：关注清单 A={(pi,wi)}，pi 为泛化 URL 模式，wi ∈ [0,1] 为动态关注度评分。<br>R: 输入：时间窗口内网关流量日志集合 L={l1,…,lN}；定向策略 S（响应时延阈值 T、错误码集合 E）；历史窗口统计 mathcalH。 输出：关注清单 A={(pi,wi)}，pi 为泛化 URL 模式，wi ∈ [0,1] 为动态关注度评分。 |
| Delete | 162 | - | - | JOSCode | - | Score(u,t)=α Freq(u,t)+β Err(u,t)+gamma Delay(u,t)+delta Trend(u,t)labeleq:dasm (1) |
| Delete | 163 | - | - | JOSBody | - | 其中 α+β+γ+δ=1（均非负），四个因子分别定义为： |
| Delete | 164 | - | - | JOSBody | - | itemize Freq(u,t)=count(u)/max v count(v)，归一化频次； Err(u,t)=err_count(u)/count(u)，错误率； Delay(u,t)=min(1, rt(u)/T)，延迟严重度； Trend(u,t)=σ\!((Freq t -Freq t-1 )/max(Freq t-1 ,ε))，热度趋势（σ 为 sigmoid 函数，将变化率映射到(0,1)）。 |
| Insert | - | 164 | - | - | JOSCode | Score(u,t)=α Freq(u,t)+β Err(u,t)+gamma Delay(u,t)+delta Trend(u,t) (1) |
| Insert | - | 165 | - | - | JOSBody | 其中 α+β+gamma+delta=1（均非负），四个因子分别定义为： itemize Freq(u,t)=count(u)/max_v count(v)，归一化频次； Err(u,t)=err_count(u)/count(u)，错误率； Delay(u,t)=minbigl(1, overlinert(u)/Tbigr)，延迟严重度； Trend(u,t)=sigma\!bigl((Freq_t-Freq_t-1)/max(Fre... |
| Delete | 166 | - | - | JOSCode | - | wdecay(u,t)=sumi wi·expbigl(-lambda (t-ti)bigr)labeleq:decay (2) |
| Delete | 167 | - | - | JOSBody | - | 其中 λ 为衰减系数（默认 0.1/s），ti 为第 i 个历史窗口的时间戳。 |
| Delete | 168 | - | - | JOSBody | - | 参数自适应。当全局错误率超过 10%时，β 自动提升 50%；当全局平均延迟超过 2T 时，γ 自动提升 50%；提升后重新归一化，确保权重系数之和为 1。默认均衡模式取 α=0.3，β=0.3，γ=0.2，δ=0.2。 |
| Insert | - | 167 | - | - | JOSCode | wdecay(u, t) = sumi wi·exp\!(-lambda (t-ti)) (2) |
| Insert | - | 168 | - | - | JOSBody | 其中 lambda 为衰减系数（默认 0.1/s），ti 为第 i 个历史窗口的时间戳。 |
| Insert | - | 169 | - | - | JOSBody | 参数自适应。当全局错误率超过 10% 时，β 自动提升 50%；当全局平均延迟超过 2T 时，gamma 自动提升 50%；提升后重新归一化，确保权重系数之和为 1。默认均衡模式取 α=0.3，β=0.3，gamma=0.2，delta=0.2。 |
| Delete | 170 | - | - | - | - | Input: 流量日志集合 L，策略 S=(T,E,K,TTL)，历史窗口 ℋ |
| Insert | - | 171 | - | - | - | Input: 流量日志集合 L，策略 S=(T,E,K,TTL)，历史窗口 mathcalH |
| Delete | 173 | - | - | - | - | )，历史窗口{H |
| Insert | - | 174 | - | - | - | H ← ∅; |
| Delete | 175 | - | - | - | - | $ |
| Insert | - | 176 | - | - | - | foreach l ∈ L do |
| Delete | 177 | - | - | - | - | H ← ∅ |
| Insert | - | 178 | - | - | - | if rt(l) > T or status(l) ∈ E then |
| Delete | 179 | - | - | - | - | foreach do |
| Insert | - | 180 | - | - | - | p ← Generalize(l.url); |
| Insert | - | 181 | - | - | - | // 数字→{id}，UUID→{uuid} |
| Delete | 181 | - | - | - | - | end |
| Insert | - | 183 | - | - | - | H[p].count ← H[p].count + 1; |
| Delete | 183 | - | - | - | - | { |
| Insert | - | 185 | - | - | - | 更新 H[p] 的错误计数、延迟累积; |
| Delete | 185 | - | - | - | - | if then |
| Insert | - | 187 | - | - | - | (α,β,gamma,delta) ← AdaptWeights(H, T); |
| Insert | - | 188 | - | - | - | // 负载自适应 |
| Delete | 187 | - | - | - | - | end |
| Insert | - | 190 | - | - | - | foreach p ∈ H do |
| Delete | 189 | - | - | - | - | (l) > T$ {or |
| Insert | - | 192 | - | - | - | wp ← Score(p,t;α,β,gamma,delta,mathcalH); |
| Insert | - | 193 | - | - | - | // 式 (1) |
| Delete | 191 | - | - | - | - | ${status |
| Insert | - | 195 | - | - | - | A ← TopK(H, K); |
| Insert | - | 196 | - | - | - | // 按 wp 降序取前 K 项 |
| Delete | 193 | - | - | - | - | (l) E$ |
| Insert | - | 198 | - | - | - | foreach (pi, wi) ∈ A do |
| Delete | 195 | - | - | - | - | { |
| Insert | - | 200 | - | - | - | 附加 TTL; |
| Delete | 197 | - | - | - | - | $p {Generalize |
| Delete | 198 | - | - | - | - | 14 |
| Delete | 199 | - | - | - | - | (l.{url |
| Delete | 200 | - | - | - | - | 15 |
| Delete | 201 | - | - | - | - | )$ {数字→{id}，UUID→{uuid\ |
| Delete | 202 | - | - | - | - | // 数字→{id}，UUID→{uuid} |
| Delete | 203 | - | - | - | - | 16 |
| Delete | 204 | - | - | - | - | $H[p].{count |
| Delete | 205 | - | - | - | - | 17 |
| Delete | 206 | - | - | - | - | H[p].{count |
| Delete | 207 | - | - | - | - | 18 |
| Delete | 208 | - | - | - | - | + 1$ |
| Delete | 209 | - | - | - | - | 19 |
| Delete | 210 | - | - | - | - | 更新 H[p] 的错误计数、延迟累积 |
| Delete | 211 | - | - | - | - | 20 |
| Delete | 212 | - | - | - | - | $(,,,) {AdaptWeights |

## 格式差异

| Left# | Right# | 文本 | 字段变化 |
| ---: | ---: | --- | --- |
| 113 | 114 | 72 vs 4388 条* | runs: `style=-;b=false;i=false;u=-;va=-;sz=15;font=Times New Roman/宋体 x1` -> `style=-;b=false;i=false;u=-;va=-;sz=15;font=Times New Roman/宋体 x1; style=-;b=false;i=false;u=-;va=superscript;sz=15;font=Times New Roman/宋体 x1` |
| 115 | 116 | 0.05% CPU* | runs: `style=-;b=false;i=false;u=-;va=-;sz=15;font=Times New Roman/宋体 x1` -> `style=-;b=false;i=false;u=-;va=-;sz=15;font=Times New Roman/宋体 x1; style=-;b=false;i=false;u=-;va=superscript;sz=15;font=Times New Roman/宋体 x1` |

## OOXML Hash

| Part | Equal | Left hash | Right hash |
| --- | --- | --- | --- |
| word/document.xml | false | 35a17db605b3e4dd | abef806ccb0811fd |
| word/styles.xml | false | 1f6c62f76a14a6ed | 381ee950c2ac8390 |
