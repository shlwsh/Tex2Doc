# Tex2Doc 支付与账单（docs-zh/pay）
> **版本 / Version**: v2.0
> **最后更新日期 / Last Updated**: 2026-06-26



本目录沉淀 Tex2Doc 接入「支付宝 / 微信」收款与账单功能的技术方案与落地资产。

## 目录

| 文件 | 内容 |
|---|---|
| [`payment-billing-technical-design-20260623.md`](./payment-billing-technical-design-20260623.md) | **主技术方案**：现状盘点、总体架构、`PaymentProvider` 抽象、计费/额度模型、API 契约、安全合规、对账、客户端改造、分阶段落地计划 |
| [`002_docdb_payment_schema.sql`](./002_docdb_payment_schema.sql) | 支付域数据库增量：订单 / 流水 / 退款 / 通知日志 / Credit 钱包账本 / 对账批次 / 商品目录 |

## 核心决策（已确认）

- **接入方式**：先聚合后直连（Beta 聚合快速跑通，GA 切支付宝/微信官方直连降费率）。
- **计费模式**：一次性购买 + 次数包 / Credits（不做自动续费代扣；Pro 用「月卡/年卡」时长卡实现）。
- **收款渠道**：支付宝、微信。
- **支付场景（默认，可调整）**：PC 扫码（当面付/Native）为主 + 移动 H5 为辅；App、JSAPI/小程序为预留扩展点。
- **币种**：CNY（金额统一最小单位「分」）。

## 快速上手

```bash
# 1. 业务库已初始化（见 docs-zh/money/001_docdb_business_schema.sql）后，叠加支付域表：
psql -U postgres -d docdb -f docs-zh/pay/002_docdb_payment_schema.sql
```

后续工程落地：新增 `crates/payment`（`doc-payment`），`doc-server` 增加 `/v1/payments/*` 路由，`commercial-api-client` 替换 Stripe 形态 billing 方法。详见主技术方案第 2、6、7 节。

## 关联文档

- `docs-zh/money/001_docdb_business_schema.sql` — 业务库基线（用户/套餐/订阅/账单/用量）
- `docs-zh/money/p6-p9-cloud-client-implementation-plan-20260623.md` — 云端与客户端对接方案
- `docs-zh/money/commercialization-promotion-plan-20260622.md` — 商业化推广规划（2.3 节「支付闭环」）
