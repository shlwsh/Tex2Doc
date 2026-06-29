# Tex2Doc React 管理端登录与注册说明

> 更新日期：2026-06-29
> 适用入口：`http://127.0.0.1:2630/admin-react`
> 启动脚本：`scripts/run_react.ps1`

## 1. 本地默认管理员账号

使用 `scripts/run_react.ps1` 启动 React 前端和 Rust service 时，脚本会给 Rust service 注入本地 bootstrap 管理员环境变量：

```text
TEX2DOC_BOOTSTRAP_ADMIN_EMAIL=demo@example.com
TEX2DOC_BOOTSTRAP_ADMIN_PASSWORD=demo
```

因此本地默认管理员登录信息为：

| 字段 | 值 |
|---|---|
| API Base URL | `http://127.0.0.1:2630/v1/` 或保持页面默认值 |
| 邮箱 | `demo@example.com` |
| 密码 | `demo` |

说明：

1. React dev server 监听 `2630`。
2. Rust service 监听 `2624`。
3. Vite 已将 `/v1/*`、`/api/*`、`/admin/v1/*` 代理到 `http://127.0.0.1:2624`。
4. 所以管理端页面里 API Base URL 使用同源 `http://127.0.0.1:2630/v1/` 即可。

## 2. 启动方式

从项目根目录执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\run_react.ps1
```

脚本会自动：

1. 检查并清理 `2624` 和 `2630` 端口占用。
2. 停止旧的 `apps/react-web` Vite 进程。
3. 启动 Rust `doc-server`。
4. 启动 React Vite dev server。
5. 验证 Rust health、React 页面和 Vite API proxy。

启动成功后访问：

```text
http://127.0.0.1:2630/admin-react
```

## 3. 登录流程

1. 打开 `http://127.0.0.1:2630/admin-react`。
2. 保持 `登录` Tab。
3. API Base URL 保持默认，或填写：

   ```text
   http://127.0.0.1:2630/v1/
   ```

4. 邮箱填写：

   ```text
   demo@example.com
   ```

5. 密码填写：

   ```text
   demo
   ```

6. 点击登录。

登录后前端会继续调用：

```text
GET /admin/v1/me
```

只有后端返回管理员角色时才会进入后台工作台。

## 4. 注册说明

管理端登录页目前保留了 `注册` Tab，但它不等同于创建管理员。

普通注册流程调用：

```text
POST /v1/auth/register
```

该接口创建的是普通用户，默认不是管理员角色。普通用户即使注册成功，也会在管理端门禁校验阶段被拒绝，并提示：

```text
Admin role required.
```

因此：

| 场景 | 是否可进入管理端 |
|---|---|
| 使用 `demo@example.com / demo` 登录 | 可以 |
| 在管理端页面自行注册新账号 | 不可以，默认普通用户 |
| 使用后端已设置为 `admin`、`operator` 或 `support` 的账号登录 | 可以 |

## 5. 修改默认管理员

如果需要改成本地自己的管理员账号，可以修改启动脚本中的 bootstrap 变量：

```powershell
$env:TEX2DOC_BOOTSTRAP_ADMIN_EMAIL = "your-admin@example.com"
$env:TEX2DOC_BOOTSTRAP_ADMIN_PASSWORD = "your-password"
```

对应位置：

```text
scripts/run_react.ps1
```

后端启动时会执行 upsert：

1. 如果邮箱不存在，则创建管理员账号。
2. 如果邮箱已存在，则更新密码、角色和状态。
3. 角色会被设置为 `admin`。
4. 状态会被设置为 `active`。

## 6. 常见问题

### 6.1 为什么填写 `http://127.0.0.1:2624/v1/` 也能登录？

可以。那是直接访问 Rust service。

但本地 React 开发推荐使用：

```text
http://127.0.0.1:2630/v1/
```

这样前端和 API 是同源请求，由 Vite proxy 转发到 Rust service，更接近正式部署路径。

### 6.2 为什么注册后仍然进不了管理端？

注册接口只创建普通用户。管理端要求后端返回管理员角色，当前认可的角色包括：

```text
admin
operator
support
```

自助注册账号默认不具备这些角色。

### 6.3 忘记本地管理员密码怎么办？

重新运行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\run_react.ps1
```

默认脚本会再次将 `demo@example.com` 的密码设置为 `demo`，并确保该账号是 `admin` 和 `active`。
