# Tex2Doc 支付与账单系统技术方案
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



**日期**：2026-06-23
**作者**：研发
**输出目录**：`docs-zh/pay`
**关联**：`docs-zh/money/001_docdb_business_schema.sql`、`docs-zh/money/p6-p9-cloud-client-implementation-plan-20260623.md`、`docs-zh/money/commercialization-promotion-plan-20260622.md`

---

## 0. 决策摘要（本方案前提）

| 决策项 | 选择 | 影响 |
|---|---|---|
| 接入方式 | **先聚合后直连**：Beta 用聚合支付（如 Ping++ / 连连 / 拉卡拉）快速跑通，GA 后切支付宝开放平台 + 微信支付商户直连降费率 | 按可插拔 `PaymentProvider` 抽象设计，渠道适配器可热插拔 |
| 计费模式 | **一次性购买 + 次数包 / Credits**（不做自动续费代扣） | 无需代扣资质门槛；订单 + 额度发放为核心，订阅周期卡用「时长卡」一次性购买实现 |
| 收款渠道 | **支付宝、微信** | 双渠道统一抽象，统一对账 |
| 支付场景（默认，可调整） | **首期：PC 扫码（当面付 / Native）为主 + 移动 H5（wap / H5）为辅**；App、JSAPI/小程序作为预留扩展点 | 桌面端（Slint / Flutter Desktop）+ PC 官网展示二维码轮询为主路径 |
| 币种 | **CNY（分）** | 金额统一最小货币单位整数；既有 USD 套餐保留，新增 CNY 套餐与商品 |

> 「支付场景」一项在需求确认时未明确勾选，本方案按当前产品形态（桌面客户端 + PC 官网）取默认值。若首期需要 App 内支付或微信小程序/公众号场景，按第 7 节扩展点补充适配器即可，不影响主体架构。

---

## 1. 背景与现状盘点

Tex2Doc（Semantic TeX Engine）已具备 LaTeX→DOCX 转换核心、商业 API preview、桌面客户端与业务库 schema。支付是商业化闭环（推广计划 2.3 节「支付闭环」）的关键缺口。现状如下：

| 模块 | 现状 | 与支付相关的差距 |
|---|---|---|
| `crates/server`（Axum，`doc-server`） | 内存态 stub；`/v1/billing/checkout`、`/v1/billing/portal` 返回**伪造 URL**，币种 USD | 无真实下单、无异步通知、无验签、无订单/流水落库 |
| `crates/commercial-api-client` | Rust 客户端，`billing.rs` 为 Stripe 形态（`plans` / `create_checkout` / `create_billing_portal`），模型含 `success_url` / `cancel_url` / `BillingPortalRequest` | 支付宝/微信无 portal 概念；需替换为下单 + 轮询 + 通知模型 |
| 业务库 `docdb`（`001_..._business_schema.sql`） | 有 `billing_plans` / `subscriptions` / `invoices` / `usage_periods` / `usage_events`；金额 `*_cents`、币种默认 USD | **缺**支付订单、支付流水、退款、通知日志、Credit 钱包/账本、对账表 |
| 客户端（Slint / Flutter） | 复用 `commercial-api-client`，已有账号/用量/账单入口 preview | 需改为「选商品 → 拉起二维码/跳转 → 轮询订单状态 → 到账提示」 |

**结论**：现有 Stripe 形态的 billing 接口与国内支付模型不兼容，需新增一套支付域（payment domain），并与既有 `usage_periods` / `invoices` 对接，而非替换。

---

## 2. 总体架构

### 2.1 分层与模块

```
                         ┌─────────────────────────────────────────────┐
   客户端                │  Slint Desktop / Flutter / PC 官网 (Web)      │
   (commercial-api-      │  选商品 → 创建订单 → 展示二维码/跳转 → 轮询    │
    client / HTTP)       └───────────────┬─────────────────────────────┘
                                         │ HTTPS (Bearer access_token)
                         ┌───────────────▼─────────────────────────────┐
   doc-server           │  payment routes:                              │
   (Axum)               │   POST /v1/payments/orders        下单        │
                        │   GET  /v1/payments/orders/:id    查询/轮询   │
                        │   POST /v1/payments/orders/:id/cancel 关单    │
                        │   POST /v1/payments/refunds       退款(内部)  │
                        │   POST /v1/payments/notify/alipay 异步通知    │
                        │   POST /v1/payments/notify/wechat 异步通知    │
                        │   GET  /v1/billing/products       商品目录    │
                        │   GET  /v1/billing/wallet         额度余额    │
                        │   GET  /v1/billing/invoices       账单列表    │
                        └───────────────┬─────────────────────────────┘
                                         │
                  ┌──────────────────────┼───────────────────────────┐
                  ▼                      ▼                           ▼
        ┌──────────────────┐  ┌────────────────────┐   ┌─────────────────────┐
        │ PaymentService    │  │ PaymentProvider     │   │ BillingService       │
        │ 订单状态机/幂等   │  │ trait (可插拔)      │   │ 额度发放/钱包/账单   │
        │ 金额校验/防重放   │  │  - AggregatorProvider│   │ usage_periods 对接   │
        └────────┬─────────┘  │  - AlipayDirect      │   │ credit_wallets/ledger│
                 │            │  - WechatDirect      │   └──────────┬──────────┘
                 │            └─────────┬───────────┘              │
                 ▼                      ▼                          ▼
        ┌──────────────────────────────────────────────────────────────────┐
        │ PostgreSQL docdb: payment_orders / payment_transactions /          │
        │  payment_refunds / payment_notify_logs / credit_wallets /          │
        │  credit_ledger / payment_products / payment_reconciliation_runs    │
        └──────────────────────────────────────────────────────────────────┘
                 ▲
                 │ 定时任务(后台 worker)
        ┌────────┴─────────────────────────────────┐
        │ 关单/补偿查询(order query)、对账(reconcile)、 │
        │ Credit 过期(可选)                          │
        └──────────────────────────────────────────┘
```

### 2.2 新增 crate：`doc-payment`

建议在 workspace 新增 `crates/payment`（包名 `doc-payment`），与 `doc-server`、`commercial-api-client` 解耦：

- `provider/mod.rs`：`PaymentProvider` trait + 工厂（按 `provider` 字符串选择实现）。
- `provider/aggregator.rs`：聚合支付适配（首期）。
- `provider/alipay.rs`、`provider/wechat.rs`：官方直连适配（GA 期）。
- `service.rs`：`PaymentService`（下单、查询、关单、退款、通知处理；不含 HTTP）。
- `billing.rs`：`BillingService`（额度发放、钱包、账单、与 `usage_periods` 对接）。
- `signing.rs`：验签 / 加签工具（RSA2 / SHA256withRSA、微信 v3 平台证书）。
- `model.rs`：领域模型（与 DB 行解耦的 DTO）。

`doc-server` 仅做 HTTP 适配（提取参数、鉴权、调用 `PaymentService`/`BillingService`、序列化响应）。

### 2.3 PaymentProvider 抽象

```rust
#[async_trait]
pub trait PaymentProvider: Send + Sync {
    /// 渠道标识，如 "alipay" / "wechat"
    fn channel(&self) -> Channel;
    /// provider 标识，如 "pingxx" / "alipay_direct"
    fn provider_id(&self) -> &str;

    /// 预下单：返回二维码内容 / 跳转 URL / App 拉起参数
    async fn create_charge(&self, req: &CreateChargeRequest) -> Result<ChargeHandle, PayError>;

    /// 主动查询订单（补偿用，应对通知丢失）
    async fn query_order(&self, out_trade_no: &str) -> Result<ChargeStatus, PayError>;

    /// 关闭/撤销订单
    async fn close_order(&self, out_trade_no: &str) -> Result<(), PayError>;

    /// 退款
    async fn refund(&self, req: &RefundRequest) -> Result<RefundHandle, PayError>;

    /// 验签并解析异步通知，返回归一化事件（验签失败返回 Err）
    fn parse_notify(&self, headers: &NotifyHeaders, body: &[u8]) -> Result<NotifyEvent, PayError>;

    /// 通知应答体（支付宝返回 "success"，微信 v3 返回 JSON {code,message}）
    fn notify_ack(&self, ok: bool) -> NotifyAck;
}
```

归一化结构（`ChargeStatus` / `NotifyEvent`）屏蔽渠道差异，上层 `PaymentService` 只处理统一语义：**订单是否支付成功、金额、渠道交易号、买家标识、发生时间**。

---

## 3. 计费与额度模型

### 3.1 商品与套餐的关系

- 既有 `billing_plans`：定义「套餐等级」（Preview / Pro）与每月额度、存储、功能。**保留**，新增 `pro_cny`（CNY 计价）。
- 新增 `payment_products`：定义「可购买商品」。两类：
  - `credit_pack`（次数包）：购买后向 `credit_wallets` 发放 `grant_conversions` 次云转换额度，**不过期或按策略过期**。
  - `plan_period`（时长卡）：购买后授予 `grant_plan_id` 套餐 `grant_period_days` 天，写入/延长 `subscriptions`（`provider='order'`，无自动续费）。

> 「不做自动续费」即用一次性「Pro 月卡 / 年卡」替代订阅扣款：到期不自动续，靠到期前提醒用户复购。这规避了支付宝周期扣款 / 微信委托代扣的资质与闭环复杂度。

### 3.2 云转换额度扣减优先级

云转换计费在既有 `usage_periods` / `usage_events` 之上叠加 Credit 钱包。扣减顺序（在 `BillingService::consume_one` 内，单事务）：

1. 当前订阅周期 `usage_periods` 还有余量 → 记 `usage_events`（云转换）。
2. 周期额度用尽 → 扣 `credit_wallets.balance_conversions`，记 `credit_ledger(reason='consume', delta=-1)`。
3. 两者都为 0 → 返回 `quota_exhausted`，前端引导购买次数包。

转换任务失败按策略退额（记 `credit_ledger(reason='refund')` 或回滚 `usage_events`），与 `conversion_jobs` 状态机联动。

### 3.3 账单（invoices）联动

每笔订单支付成功后，生成一条 `invoices` 记录（`status='paid'`、`amount_paid_cents`、`currency='CNY'`、`hosted_invoice_url` 可空），供客户端「账单」页展示。`invoices.subscription_id` 对时长卡可关联，次数包置空。

---

## 4. 数据库设计

详见同目录 `002_docdb_payment_schema.sql`。新增表一览：

| 表 | 作用 | 关键约束 |
|---|---|---|
| `payment_products` | 可购买商品目录（次数包 / 时长卡） | `kind ∈ {credit_pack, plan_period}` |
| `payment_orders` | 业务订单，商户订单号 `out_trade_no` 唯一 | 状态机；`(user_id, idempotency_key)` 幂等唯一 |
| `payment_transactions` | 渠道真实支付流水 | `(provider, provider_transaction_id)` 唯一去重 |
| `payment_refunds` | 退款单，`out_refund_no` 唯一 | 状态机 |
| `payment_notify_logs` | 异步通知原始报文（验签前先落库） | 按 `out_trade_no` / 交易号检索 |
| `credit_wallets` | 次数包额度当前余额 | `balance >= 0` |
| `credit_ledger` | 额度流水账（append-only） | `idempotency_key` 唯一，保证一单只发放一次 |
| `payment_reconciliation_runs` | 每日对账批次结果 | `(provider, bill_date)` 唯一 |

**金额一律整数最小单位（分）**，与既有 `*_cents` 习惯一致；`currency` 列区分 CNY/USD。

### 4.1 订单状态机

```
created ──下单成功──▶ pending ──通知/查询=成功──▶ paid ──发起退款──▶ refunding ──▶ refunded
   │                    │                                              
   │                    └──超时未支付(close_order)──▶ closed           
   └──下单渠道报错──▶ failed
```

幂等要点：

- 通知/查询到达成功时，用 `UPDATE payment_orders SET status='paid' WHERE id=? AND status IN ('created','pending')` 的**条件更新**保证只迁移一次；受影响行数为 0 则说明已处理，直接幂等返回。
- 额度发放绑定 `credit_ledger.idempotency_key = 'order:' || out_trade_no`，唯一约束兜底，杜绝重复到账。

---

## 5. 核心支付时序（PC 扫码 / 当面付为例）

```
客户端            doc-server                Provider(渠道)         DB
  │  POST /payments/orders {product_id,channel,scene=native}      │
  │─────────────▶│                                                │
  │              │ 校验登录/商品/金额; 生成 out_trade_no          │
  │              │ INSERT payment_orders(status=created) ─────────▶│
  │              │ provider.create_charge() ───▶│                 │
  │              │              二维码内容 qr_code ◀──│            │
  │              │ UPDATE order(status=pending,qr_code) ──────────▶│
  │◀── {order_id,qr_code,expires_at} ─────│                       │
  │ 展示二维码                                                     │
  │ 轮询 GET /payments/orders/:id ────────▶│ 读 order ◀───────────│
  │              │ (status=pending) ───────────────────────────── │
  │                                                               │
  │            用户扫码支付 ──────────────▶ 渠道                  │
  │                          渠道异步通知 POST /payments/notify/* │
  │              ◀──────────────────────────────│                │
  │              │ 落 notify_logs; 验签; 校验金额/订单            │
  │              │ INSERT payment_transactions(success)（唯一去重）│
  │              │ 条件更新 order→paid; BillingService 发放额度   │
  │              │ INSERT credit_ledger(idem=order:xxx)           │
  │              │ 生成 invoices(paid)                            │
  │              │ 应答渠道 "success"/{code:SUCCESS} ────────────▶│
  │ 轮询 GET /payments/orders/:id ────────▶│                      │
  │◀── {status=paid, granted:100} ────────│                      │
  │ 提示到账, 刷新额度                                            │
```

**补偿**：后台 worker 周期性对 `pending` 且接近超时的订单调用 `provider.query_order()`，应对通知丢失；超时仍未支付则 `close_order` 并置 `closed`。

---

## 6. API 契约（doc-server 新增）

所有用户态接口需 `Authorization: Bearer <access_token>`；通知接口免鉴权但**必须验签**。同时注册 `/v1/...` 与 `/api/v1/...` 两套前缀（与现有路由风格一致）。

### 6.1 商品目录

```
GET /v1/billing/products
200 → { "products": [
  { "id":"credits_100","name":"云转换 100 次包","kind":"credit_pack",
    "currency":"CNY","price_cents":990,"grant_conversions":100 },
  { "id":"pro_month_cny","name":"Pro 月卡","kind":"plan_period",
    "currency":"CNY","price_cents":19900,"grant_plan_id":"pro_cny","grant_period_days":30 }
] }
```

### 6.2 创建订单

```
POST /v1/payments/orders
Header: Idempotency-Key: <client-uuid>      # 可选，重复提交去重
Body: { "product_id":"credits_100", "channel":"alipay", "scene":"native",
        "return_url":"https://tex2doc.cn/pay/return" }   # return_url 仅 web/wap 需要
200 → {
  "order_id":"<uuid>", "out_trade_no":"T2D20260623...", "status":"pending",
  "amount_cents":990, "currency":"CNY",
  "scene":"native", "qr_code":"https://qr.alipay.com/...",   # native: 二维码内容
  "pay_url":null,                                            # web/wap: 跳转URL
  "expires_at":"2026-06-23T10:15:00Z"
}
```

按 `scene` 返回不同载荷：`native`→`qr_code`；`web`/`wap`→`pay_url`；`app`→渠道 SDK 拉起参数（预留）；`jsapi`→`pay_params`（预留）。

### 6.3 查询订单（轮询）

```
GET /v1/payments/orders/:id
200 → { "order_id":..., "status":"paid|pending|closed|failed|refunded",
        "paid_at":..., "granted":{"conversions":100} }
```

建议客户端轮询间隔 2–3s、最长 5 分钟，配合 SSE/WebSocket 可选优化（首期轮询足够）。

### 6.4 关单 / 退款

```
POST /v1/payments/orders/:id/cancel        # 用户主动放弃, pending→closed
POST /v1/payments/refunds                   # 内部/客服态, 需权限
  Body: { "order_id":..., "amount_cents":990, "reason":"用户申请" }
```

### 6.5 异步通知

```
POST /v1/payments/notify/alipay     # application/x-www-form-urlencoded, RSA2 验签
POST /v1/payments/notify/wechat     # application/json, 微信 v3 证书验签 + AES-GCM 解密
处理成功：支付宝返回纯文本 "success"；微信返回 200 + {"code":"SUCCESS"}
处理失败/验签失败：返回非成功码使渠道重试（微信 5xx / {"code":"FAIL"}）
```

### 6.6 钱包与账单

```
GET /v1/billing/wallet    → { "balance_conversions":100, "updated_at":... }
GET /v1/billing/invoices  → { "invoices":[ {amount_paid_cents,currency,status,paid_at,...} ] }
```

### 6.7 客户端 `commercial-api-client` 改造

`billing.rs` 中 Stripe 形态方法做如下调整（保留旧方法标 `#[deprecated]` 过渡）：

- 删除/弃用 `create_billing_portal`（国内无 portal）。
- 新增 `list_products()`、`create_payment_order(req)`、`get_payment_order(id)`、`cancel_payment_order(id)`、`get_wallet()`、`list_invoices()`。
- `models.rs` 新增 `PaymentProductSummary` / `CreateOrderRequest` / `PaymentOrder` / `WalletSummary`；`CheckoutRequest`（含 `success_url/cancel_url`）弃用。

> 改动前按 CLAUDE.md 要求对受影响符号跑 `impact`：`create_checkout` / `create_billing_portal` / `CheckoutRequest` / `BillingSession`，并核对 Slint、Flutter 两端调用点。

---

## 7. 渠道适配与「先聚合后直连」

### 7.1 首期：聚合支付适配器

一套 `AggregatorProvider` 同时覆盖支付宝/微信，下单参数中带 `channel`。优点：一次接入、统一对账、统一退款；缺点：抽成、依赖第三方可用性、部分服务商要求企业资质。

- 下单：调用聚合方 `create_charge`，拿到二维码/跳转 URL。
- 通知：聚合方统一回调到 `/v1/payments/notify/aggregator`（或复用 `/alipay` `/wechat`），用聚合方提供的签名机制验签。
- 抽象层用同一 `PaymentProvider`，仅 `provider_id` 与验签实现不同。

### 7.2 GA 期：官方直连适配器

| 渠道 | 产品（按 scene） | 下单接口 | 验签 |
|---|---|---|---|
| 支付宝 | native→`alipay.trade.precreate`（当面付）；web→`alipay.trade.page.pay`；wap→`alipay.trade.wap.pay`；app→`alipay.trade.app.pay` | 开放平台网关 | RSA2（SHA256withRSA），应答需验支付宝公钥；推荐公钥证书模式 |
| 微信支付 v3 | native→Native 下单；H5→H5 下单；JSAPI→JSAPI 下单；app→APP 下单 | `api.mch.weixin.qq.com/v3/pay/transactions/*` | 请求用商户 API 私钥加签；通知用微信平台证书验签 + `AEAD_AES_256_GCM` 解密 resource |

直连需准备：营业执照与支付宝/微信商户号、应用 AppID、API 私钥/证书、`notify_url` 公网回调、IP 白名单（部分接口）。

### 7.3 切换策略

- 配置驱动：`payment_orders.provider` 记录每单实际 provider；新单的默认 provider 由配置/灰度开关决定，存量订单按各自 provider 处理通知与退款，平滑切换。
- 验签密钥、商户号等通过环境变量/密钥管理注入（见 `.env.mygit.example` 既有风格），**严禁入库明文或提交仓库**。

---

## 8. 安全与合规

| 项 | 措施 |
|---|---|
| 验签 | 通知先落 `payment_notify_logs`（验签前原文），再验签；验签失败一律拒绝并告警，绝不发放额度 |
| 金额校验 | 通知金额必须等于 `payment_orders.amount_cents`，币种一致；不一致记异常、不发货 |
| 幂等 / 防重放 | 订单条件更新 + `payment_transactions` 唯一约束 + `credit_ledger.idempotency_key`；通知重复到达天然幂等 |
| 订单归属 | 通知中的 `out_trade_no` 必须能在本地找到对应订单且属于发起用户 |
| 金额来源 | 金额只由服务端按 `payment_products.price_cents` 计算，**绝不信任客户端传入金额** |
| 密钥管理 | 私钥/证书走环境变量或 KMS；最小权限；定期轮换；日志脱敏（buyer_id、openid 仅存哈希/截断） |
| 传输安全 | 全链路 HTTPS；`notify_url` 仅暴露 POST，做来源 IP/证书校验（直连） |
| 越权 | 退款接口需管理员/客服权限；用户接口只能操作本人订单 |
| 审计 | 订单、流水、退款、通知、额度账本均可追溯；保留期符合财务要求 |
| 资质合规 | 直连需主体资质与商户号；定价、退款政策、发票需符合相关法规；隐私政策声明支付数据处理 |

---

## 9. 对账与运维

- **每日对账**（`payment_reconciliation_runs`）：T+1 拉取渠道账单文件，与本地 `payment_transactions` 逐笔比对，标记 `matched / mismatched / missing_local / missing_remote`，差异告警人工处理。
- **补偿查询**：后台 worker 对 `pending` 订单定时 `query_order`，弥补通知丢失；超时关单。
- **可观测性**：埋点指标——下单数、成功率、平均到账时延、通知验签失败率、对账差异数、退款率；接入现有 tracing/告警。
- **告警**：验签失败突增、对账长期不平、通知积压、关单失败。

---

## 10. 客户端改造（Slint / Flutter / Web）

| 端 | 改动 |
|---|---|
| Slint Desktop | 账单页新增「购买次数包 / Pro 卡」→ 调 `create_payment_order(scene=native)` → 内嵌二维码控件 + 轮询订单 → 到账刷新额度。复用 `commercial-api-client` 新方法 |
| Flutter | 桌面/Web 同上展示二维码；移动端可走 `scene=wap` 拉起浏览器/客户端；App 内支付为后续扩展（`scene=app` + SDK） |
| PC 官网 | 落地页「升级 Pro / 买次数包」→ 同一下单 API → 展示二维码或跳转；`return_url` 回跳后以订单查询为准（不以前端回跳判定支付成功） |

UI 状态：待支付（二维码/倒计时）、支付中、已到账（额度刷新）、已超时（重新下单）、失败重试。

---

## 11. 分阶段落地计划

| 阶段 | 范围 | 验收标准 |
|---|---|---|
| **M1 数据与契约** | 执行 `002_docdb_payment_schema.sql`；定义 `doc-payment` crate 骨架与 `PaymentProvider` trait；冻结 API 契约 | schema 建表通过；trait/DTO 编译通过；契约评审通过 |
| **M2 聚合打通（沙箱）** | `AggregatorProvider` 下单/通知/查询/退款；`PaymentService` 状态机 + 幂等；额度发放 + 钱包/账本 | 沙箱完成「支付宝/微信扫码 → 通知 → 到账 → 额度可用」闭环；重复通知不重复发放 |
| **M3 客户端接入** | `commercial-api-client` 新方法；Slint/Flutter 购买流；PC 官网下单页 | 桌面端可买次数包并立即云转换；轮询到账正确 |
| **M4 对账与补偿** | 每日对账批次；补偿查询/关单 worker；监控告警 | 沙箱对账无差异；杀掉一次通知后补偿查询仍能到账 |
| **M5 生产灰度（聚合）** | 真实商户号、生产密钥、灰度放量 | 小流量真实收款成功；退款流程跑通 |
| **M6 直连切换（GA）** | `AlipayDirect` / `WechatDirect` 适配器；配置灰度切换 | 新单走直连成功；存量聚合单正常处理；费率下降 |
| **附加** | 对公转账/发票（Team/Enterprise）、Credit 过期策略、自动续费（如后续需要） | 按需排期 |

---

## 12. 风险与开放问题

| 项 | 说明 | 处理 |
|---|---|---|
| 聚合服务商选型与资质 | Ping++/连连/拉卡拉/payjs 等费率、结算周期、资质要求不同 | M1 前完成选型对比与签约；抽象层已隔离差异 |
| 直连商户资质周期 | 支付宝/微信商户号申请与审核耗时 | M5 前并行启动申请，不阻塞聚合上线 |
| 桌面端二维码体验 | 桌面应用内渲染二维码与轮询时延 | 首期轮询 2–3s；后续可选 SSE/WebSocket 推送 |
| Credit 是否过期 | 影响收入确认与用户预期 | 默认不过期；如需过期，加 `credit_ledger(reason='expire')` 定时任务，需先明确政策 |
| 发票/税务 | 数电发票开具与对接 | 列入附加阶段，Team/Enterprise 优先 |
| 既有 USD 套餐 | `billing_plans` 默认 USD | 保留，新增 CNY 套餐/商品；客户端按 `currency` 展示 |
| 自动续费需求回潮 | 若后续要订阅扣款 | 在 `PaymentProvider` 增加 `agreement_*`（支付宝周期扣款 / 微信委托代扣），需相应资质 |

---

## 附录 A：环境变量约定（示例，实际值走密钥管理）

```
# 通用
PAYMENT_PROVIDER_DEFAULT=aggregator      # aggregator | alipay_direct | wechat_direct
PAYMENT_NOTIFY_BASE_URL=https://api.tex2doc.cn

# 聚合（示例）
AGG_API_KEY=...
AGG_API_SECRET=...
AGG_WEBHOOK_SECRET=...

# 支付宝直连
ALIPAY_APP_ID=...
ALIPAY_APP_PRIVATE_KEY=...           # PEM, 走密钥管理
ALIPAY_PUBLIC_KEY_CERT=...           # 公钥证书模式
ALIPAY_NOTIFY_URL=${PAYMENT_NOTIFY_BASE_URL}/v1/payments/notify/alipay

# 微信支付 v3 直连
WECHAT_MCH_ID=...
WECHAT_APP_ID=...
WECHAT_API_V3_KEY=...
WECHAT_MCH_PRIVATE_KEY=...            # 商户 API 私钥, PEM
WECHAT_MCH_SERIAL_NO=...
WECHAT_NOTIFY_URL=${PAYMENT_NOTIFY_BASE_URL}/v1/payments/notify/wechat
```

## 附录 B：关键不变量（实现自测清单）

1. 服务端金额永远来自 `payment_products`，从不采信客户端金额。
2. 一个 `out_trade_no` 最多产生一次成功 `credit_ledger` 发放（唯一键）。
3. 通知验签失败 → 不改订单、不发额度、记录并告警。
4. 订单状态迁移用条件更新，受影响行数为 0 视为已处理（幂等返回）。
5. 退款金额 ≤ 已支付金额；退款成功按策略回收已发放额度。
6. 所有支付接口 HTTPS；通知接口必验签；密钥不入库不入仓。
