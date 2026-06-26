# Tex2Doc Slint 桌面端快捷助手与本地转换额度管理设计方案

> **更新日期 / Last Updated**: 2026-06-26
> **当前阶段 / Phase**: 技术方案规划与实现设计

---

## 一、 需求分析与模块设计

### 1.1 本地转换工作流完善 (配额管理与防丢失)
当前本地转换工作流通过 `convert_local_blocking` 直接编译出结果并写入用户指定的目标文件夹中，没有任何配额校验或消费逻辑。为了将其纳入计费和配额控制体系，需要进行如下完善：
1. **预校验额度**：在启动本地编译前，调用服务器接口 `/v1/local-conversions/check`，校验当前账户是否拥有可用额度。如果没有额度，转换动作被拦截并报配额不足错。
2. **生成临时结果**：本地编译正常进行，但编译出的 DOCX 文件不能直接写至指定的目标文件夹，而是写入一个安全的临时目录中，并保存好这个路径。
3. **确认并消费额度**：文件写入临时目录后，再次调用服务器接口 `/v1/local-conversions/consume` 扣减额度。扣减成功后记录消费账单。
4. **搬移至正式目录**：只有扣减成功后，客户端才会将临时目录中的正式 DOCX 文件移动并写入到用户指定的输出路径，同时移除临时文件。若扣减失败，整个流程报错且不把结果交给用户，防止未授权免费使用。

### 1.2 快捷助手模块 (Quick Assistant)
快捷助手是客户端新引入的零门槛入口模块。
- **免登录**：用户无须输入个人的邮箱密码注册登录，可以直接进行本地文档转换。
- **激活解锁**：界面提供一个兑换码输入框，输入兑换码激活后，即可使用快捷转换功能。
- **购买卡片链接**：兑换码输入框旁配置“购买卡片/在线购买”的按钮/链接，点击后自动用系统浏览器打开官方充值发卡页面（`https://pay.ldxp.cn/item/ns8i2g`）。

### 1.3 兑换码激活的临时账号复用逻辑
为了复用服务器原有的鉴权、配额管理和账单流水，兑换码激活采用**影子账号 (Shadow Account)** 方案：
- 当用户在快捷助手输入兑换码并点击“激活”时：
  1. **尝试登录**：系统以 `email = <兑换码>`，`password = <兑换码>` 发起登录请求。
  2. **自动注册**：如果登录因账号不存在失败，系统将以相同用户名和密码发起注册请求，然后再执行登录。
  3. **自动兑换**：登录成功并获得 `access_token` 后，系统通过 `client.redeem_code(code)` 接口将该兑换码兑换至此账号中，为其充值关联的额度。
  4. **凭据持久化**：系统将该兑换码（作为临时账号的账号/密码）保存在客户端凭据存储和设置中，后续重新启动应用时自动登录，维持激活状态。

### 1.4 双模式 Tab 切换与默认启动
客户端分为两种入口模式：
1. **快捷模式 (Quick Mode - Default)**：无须登录，以激活码机制进行本地文档的快捷转换。应用启动时默认进入此模式。
2. **会员中心 (User Mode)**：需登录账号，支持云端/本地转换、账单与配额查看、套餐订购、反馈处理等。
这两种模式通过窗口顶部的 **TAB 栏** 进行自由切换。

---

## 二、 服务端 API 设计 (`apps/rust-service`)

需要在服务端新增本地转换的配额校验与消费接口，以配合客户端的流程控制。

### 2.1 API 路由注册 (`apps/rust-service/src/routes.rs`)
在 `routes.rs` 中注册两个新端点（及 `/api/v1/` 别名）：
```rust
.route("/v1/local-conversions/check", post(check_local_conversion))
.route("/api/v1/local-conversions/check", post(check_local_conversion))
.route("/v1/local-conversions/consume", post(consume_local_conversion))
.route("/api/v1/local-conversions/consume", post(consume_local_conversion))
```

### 2.2 数据库存储层扩展 (`apps/rust-service/src/db_store.rs`)
在 `DbStore` 实现中，添加 `consume_local_conversion` 函数，原子扣减本地转换额度并记账：
```rust
pub async fn consume_local_conversion(&self, user_id: &str) -> Result<u64, u64> {
    let user_uuid = parse_uuid(user_id).map_err(|_| 0_u64)?;
    let mut tx = self.pool.begin().await.map_err(|_| 0_u64)?;
    
    // 1. 获取配额
    let entitlement = sqlx::query(
        r#"
        INSERT INTO commercial_entitlements (user_id)
        VALUES ($1)
        ON CONFLICT (user_id) DO UPDATE SET user_id = EXCLUDED.user_id
        RETURNING count_balance, valid_until
        "#,
    )
    .bind(user_uuid)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| 0_u64)?;

    let count_balance = entitlement.get::<i64, _>("count_balance");
    let valid_until_active = entitlement
        .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("valid_until")
        .ok()
        .flatten()
        .is_some_and(|value| value >= chrono::Utc::now());
    let used = preview_conversions_used_tx(&mut tx, user_uuid)
        .await
        .map_err(|_| 0_u64)?;

    // 2. 依次校验 时间授权、按次额度、测试额度
    if valid_until_active {
        insert_usage_ledger(
            &mut tx,
            user_uuid,
            None,
            "reserve",
            0,
            None,
            "date_entitlement",
            Some("local conversion using date entitlement"),
        )
        .await
        .map_err(|_| used)?;
        tx.commit().await.map_err(|_| used)?;
        return Ok(used);
    }

    if count_balance > 0 {
        let new_balance = count_balance - 1;
        sqlx::query(
            "UPDATE commercial_entitlements SET count_balance = $2, updated_at = now() WHERE user_id = $1",
        )
        .bind(user_uuid)
        .bind(new_balance)
        .execute(&mut *tx)
        .await
        .map_err(|_| used)?;

        insert_usage_ledger(
            &mut tx,
            user_uuid,
            None,
            "reserve",
            1,
            Some(new_balance),
            "entitlement",
            Some("count entitlement consumed for local conversion"),
        )
        .await
        .map_err(|_| used)?;
        tx.commit().await.map_err(|_| used)?;
        return Ok(used);
    }

    if used >= PREVIEW_CLOUD_CONVERSION_LIMIT {
        tx.rollback().await.ok();
        return Err(used);
    }

    // 3. 扣减预览额度
    insert_usage_ledger(
        &mut tx,
        user_uuid,
        None,
        "reserve",
        1,
        None,
        "preview",
        Some("preview local conversion consumed"),
    )
    .await
    .map_err(|_| used)?;
    tx.commit().await.map_err(|_| used)?;
    Ok(used + 1)
}
```

### 2.3 路由处理器实现 (`apps/rust-service/src/routes.rs`)
- `check_local_conversion`：校验是否允许本次转换：
  - 检查会话并获取 `session.id`。
  - 获取用户的 `entitlement` 状态及 `used` 计数。
  - 若 `valid_until_active || count_balance > 0 || used < PREVIEW_CLOUD_CONVERSION_LIMIT` 则允许。
- `consume_local_conversion`：执行实际扣费：
  - 校验当前 session。
  - 调用 `state.db.consume_local_conversion(&session.id)`。
  - 若超出配额上限返回 `ApiError::PaymentRequired` (402)。

---

## 三、 客户端 API 交互层 (`crates/commercial-api-client`)

扩展 API 客户端以适应新增的额度处理逻辑。

### 3.1 增加模型定义 (`src/models.rs`)
```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalQuotaCheckResponse {
    pub allowed: bool,
    pub valid_until_active: bool,
    pub count_balance: u32,
    pub used: u32,
    pub limit: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LocalQuotaConsumeResponse {
    pub consumed: bool,
    pub balance: u32,
}
```

### 3.2 增加 API 方法 (`src/conversions.rs`)
```rust
impl ApiClient {
    pub async fn check_local_conversion(&self) -> Result<LocalQuotaCheckResponse, ApiError> {
        self.post("local-conversions/check", &()).await
    }

    pub async fn consume_local_conversion(&self) -> Result<LocalQuotaConsumeResponse, ApiError> {
        self.post("local-conversions/consume", &()).await
    }
}
```

---

## 四、 桌面端本地转换逻辑优化 (`apps/slint-user/src/cloud_convert.rs`)

`convert_local_blocking` 需要接入 API 客户端执行远程配额操作，新流程设计如下：

1. **构造 API Client**：通过传入的 `base_url` 与 `access_token` 生成验证网络通道。
2. **预校验额度**：
   - 调用 `client.check_local_conversion().await`。
   - 若 `allowed == false`，则抛出 `CloudConvertError::QuotaExceeded` 错误，终止流程。
3. **输出至临时路径**：
   - 提取 ZIP 项目至 `tempfile::TempDir`。
   - 在此临时目录下，设置一个 `.docx` 生成临时路径，如 `temp_docx_path = temp_dir.path().join("temp_result.docx")`。
   - 调用 `SemanticTexEngine` 编译转换，将 DOCX 字节流写入 `temp_docx_path`。
4. **扣除额度**：
   - 调用 `client.consume_local_conversion().await`，如果失败，抛出错误，转换失败，且不导出成品。
5. **文件转移**：
   - 消费额度成功后，将 `temp_docx_path` 复制写入到用户真正的 `output_docx` 路径。
   - 将报告写入用户制定的 `report_path`。

---

## 五、 UI 界面与交互重构 (`apps/slint-user/src`)

### 5.1 Tab 面板布局 (`src/ui/main.slint`)
引入 `is-quick-mode` 全局状态。重构窗口主区域：
```slint
// 顶层导航栏
VerticalBox {
    padding: 0px;
    spacing: 0px;
    
    // 快捷模式与会员中心切换 Tab
    Rectangle {
        height: 50px;
        background: color-surface-alt;
        border-color: color-border;
        border-width: 1px;
        HorizontalBox {
            padding-left: 20px;
            padding-right: 20px;
            spacing: 16px;
            
            // Tab 1: 快捷助手
            Rectangle {
                width: 130px;
                background: is-quick-mode ? color-surface : #00000000;
                border-radius: 6px;
                border-color: is-quick-mode ? color-border : #00000000;
                border-width: is-quick-mode ? 1px : 0px;
                TouchArea { clicked => { is-quick-mode = true; } }
                Text {
                    text: "快捷助手 (Quick)";
                    font-weight: is-quick-mode ? 700 : 500;
                    color: is-quick-mode ? color-accent : color-text-secondary;
                    horizontal-alignment: center;
                    vertical-alignment: center;
                }
            }
            
            // Tab 2: 会员中心
            Rectangle {
                width: 130px;
                background: !is-quick-mode ? color-surface : #00000000;
                border-radius: 6px;
                border-color: !is-quick-mode ? color-border : #00000000;
                border-width: !is-quick-mode ? 1px : 0px;
                TouchArea { clicked => { is-quick-mode = false; } }
                Text {
                    text: "会员中心 (Member)";
                    font-weight: !is-quick-mode ? 700 : 500;
                    color: !is-quick-mode ? color-accent : color-text-secondary;
                    horizontal-alignment: center;
                    vertical-alignment: center;
                }
            }
            
            Rectangle { horizontal-stretch: 1; }
        }
    }

    // 各面板展示
    if is-quick-mode : Rectangle {
        // 快捷模式主工作台
        VerticalBox {
            padding: 20px;
            spacing: 12px;
            
            // 兑换码激活区块
            Rectangle {
                border-radius: 8px;
                border-color: color-border;
                background: color-surface;
                VerticalBox {
                    padding: 16px;
                    spacing: 8px;
                    Text { text: "兑换码激活 (Redeem Activation)"; font-size: 14px; font-weight: 700; color: color-text-primary; }
                    HorizontalBox {
                        spacing: 10px;
                        LineEdit { placeholder-text: "请输入充值兑换码/激活码 (Enter Redeem Code)..."; text <=> redeem-code; }
                        Button { text: "激活当前模式 (Activate)"; primary: true; clicked => { quick-activate-clicked(redeem-code); } }
                        Button { text: "购买卡片 (Buy Code)"; clicked => { purchase-redeem-code-clicked(); } }
                    }
                    Text { text: quick-activation-status; font-size: 12px; color: color-accent; }
                }
            }

            // 转换文件选择与操作区块
            Rectangle {
                border-radius: 8px;
                border-color: color-border;
                background: color-surface;
                // 复用本地转换表单: 输入路径、MainTeX、输出路径、Profile、Quality、转换按钮、Progress
            }
        }
    }
    
    if !is-quick-mode : Rectangle {
        // 原会员登录与工作区
        if !is-signed-in: Rectangle { /* ... 原登录模块 ... */ }
        if is-signed-in: HorizontalBox { /* ... 原会员中心侧边栏页面布局 ... */ }
    }
}
```

### 5.2 交互逻辑绑定 (`src/ui_bindings/` 及 `main.rs`)
- **兑换码激活**：
  - 在后台线程中：
    - 调用 `login(code, code)`，若返回 401，则调用 `register(code, code)`，注册成功后再次执行 `login(code, code)`。
    - 取得 Token 后，将 Token 和 API 地址注入 `ApiClient` 实例。
    - 调用 `client.redeem_code(code)` 激活兑换码，若报 "已兑换" 则视作登录同步完成不影响状态。
    - 之后调用 `client.usage()` 获取该激活临时账号的当前额度，并将状态存入 `settings.json`（持久化）。
    - 回传 UI 刷新激活额度、隐藏激活输入框或显示已激活状态。
- **本地转换调用**：
  - 会员模式的 "Convert" 以及 快捷助手的 "Convert" 均将调用带 Token 与 API 地址的本地转换程序。
  - 临时生成目录放置在系统的 `TempDir` 级别。
  - 处理完毕更新 UI 的额度数字。
