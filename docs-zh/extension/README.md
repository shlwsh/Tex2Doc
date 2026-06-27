# Tex2Doc 浏览器扩展部署及使用手册

## 目录

- [快速部署](#快速部署)
- [功能介绍](#功能介绍)
- [使用指南](#使用指南)
- [开发构建](#开发构建)
- [故障排除](#故障排除)

---

## 快速部署

### 支持的浏览器

| 浏览器 | 清单版本 | 构建命令 | 输出目录 |
|--------|----------|----------|----------|
| **Chrome** | MV3 | `npm run build:chrome` | `.output/chrome-mv3/` |
| **Edge** | MV3 | `npm run build:edge` | `.output/chrome-mv3-edge/` |
| **Safari** | MV2 | `npm run build:safari` | `.output/safari-mv2/` |

### 步骤 1：构建扩展

```bash
cd apps/browser-extension
npm install
# 构建所有浏览器版本
npm run build:chrome
npm run build:edge
npm run build:safari
```

### 步骤 2：加载到 Chrome

1. 打开 Chrome 浏览器
2. 访问 `chrome://extensions/`
3. 开启右上角的 **开发者模式**
4. 点击 **加载已解压的扩展程序**
5. 选择 `.output/chrome-mv3/` 文件夹

### 步骤 3：加载到 Edge

1. 打开 Edge 浏览器
2. 访问 `edge://extensions/`
3. 开启右上角的 **开发者模式**
4. 点击 **加载已解压的扩展程序**
5. 选择 `.output/chrome-mv3-edge/` 文件夹

### 步骤 4：加载到 Safari

> **注意**：Safari 扩展需要使用 Xcode 或 Safari Developer 工具加载。

1. 打开 Xcode
2. 选择 **File** → **Open**
3. 打开 `apps/browser-extension` 文件夹
4. 选择 Safari 扩展的 scheme 并运行
5. 在 Safari 中启用扩展：**Safari 设置** → **扩展** → 开启 Tex2Doc

### 步骤 5：固定扩展图标（可选）

1. 在浏览器扩展页面找到 Tex2Doc 扩展
2. 点击扩展图标旁的固定图标按钮
3. 扩展将显示在地址栏右侧

---

## 功能介绍

### 核心功能

| 功能 | 说明 |
|------|------|
| **LaTeX 转 Word** | 将 LaTeX 文档转换为 Word (.docx) 格式 |
| **本地转换** | 使用 WebAssembly 在本地完成转换，文件不离开设备 |
| **云端转换** | 上传至 Tex2Doc 云服务处理复杂文档 |
| **Overleaf 集成** | 一键转换 Overleaf 项目 |
| **arXiv 支持** | 下载并转换 arXiv 论文 |
| **账户管理** | 登录账户追踪用量、配额管理和计费 |

### 权限说明

| 权限 | 用途 |
|------|------|
| `storage` | 存储扩展设置和会话信息 |
| `downloads` | 下载转换后的 Word 文档 |
| `contextMenus` | 右键菜单 |
| `notifications` | 转换完成通知 |
| `sidePanel` | 侧边面板功能（Chrome/Edge） |
| `host_permissions` | 仅允许访问 `api.tex2doc.cn` |

### 浏览器差异

| 功能 | Chrome | Edge | Safari |
|------|--------|------|--------|
| 侧边面板 | ✅ 支持 | ✅ 支持 | ❌ 不支持 |
| 清单版本 | MV3 | MV3 | MV2 |
| WebAssembly | ✅ 支持 | ✅ 支持 | ✅ 支持 |

---

## 使用指南

### 打开扩展

1. 点击浏览器工具栏中的 Tex2Doc 图标
2. 或右键点击页面，选择 "Open Tex2Doc"

### 基本转换流程

1. **准备 LaTeX 文件**
   - 确保 `.tex` 文件完整（包含主文件和依赖）
   - 整理好文件结构

2. **选择转换模式**
   - **本地转换**：适合简单文档，无需网络
   - **云端转换**：适合复杂文档，自动处理依赖

3. **开始转换**
   - 上传文件或粘贴 LaTeX 代码
   - 选择输出格式和质量
   - 点击转换按钮

4. **下载结果**
   - 转换完成后自动通知
   - 点击下载按钮获取 Word 文档

### Overleaf 集成

1. 打开 Overleaf 项目页面
2. 点击 Tex2Doc 扩展图标
3. 选择 "Convert Overleaf Project"
4. 等待转换完成并下载

### arXiv 论文转换

1. 打开 arXiv 论文页面
2. 点击 Tex2Doc 扩展图标
3. 选择 "Download & Convert"
4. 等待转换完成

### 账户管理

1. **登录**：点击扩展图标 → 登录
2. **查看用量**：登录后查看已用配额
3. **订阅管理**：点击设置 → 账户管理

---

## 开发构建

### 项目结构

```
apps/browser-extension/
├── src/
│   ├── entrypoints/       # 扩展入口点
│   │   ├── background.ts  # 后台服务脚本
│   │   ├── popup/         # 弹出窗口
│   │   ├── options/       # 设置页面
│   │   ├── sidepanel/     # 侧边面板
│   │   └── content/       # 内容脚本
│   │       ├── arxiv.content.ts    # arXiv 支持
│   │       ├── overleaf.content.ts # Overleaf 支持
│   │       └── generic.content.ts  # 通用页面支持
│   ├── api/               # API 客户端
│   │   ├── api-client.ts
│   │   ├── auth.ts
│   │   ├── conversions.ts
│   │   ├── billing.ts
│   │   └── usage.ts
│   ├── browser/           # 浏览器兼容性层
│   ├── conversion/        # 转换逻辑
│   │   ├── cloud-conversion.ts  # 云端转换
│   │   ├── local-wasm.ts       # 本地 WASM 转换
│   │   └── project-zip.ts      # 项目 ZIP 处理
│   ├── state/             # 状态管理 (Zustand)
│   │   ├── session-store.ts
│   │   ├── settings-store.ts
│   │   ├── job-store.ts
│   │   └── quota-store.ts
│   ├── ui/                # UI 组件
│   │   ├── components/
│   │   ├── theme/
│   │   └── i18n/
│   └── workers/           # Web Workers
├── public/
│   └── icons/             # 扩展图标
├── wxt.config.ts          # WXT 配置
├── tailwind.config.ts     # Tailwind CSS 配置
└── package.json
```

### 构建命令

| 命令 | 说明 | 输出目录 |
|------|------|----------|
| `npm run dev` | 开发模式（热重载） | `.output/chrome-mv3-dev/` |
| `npm run build` | 构建所有浏览器版本 | 根据浏览器不同 |
| `npm run build:chrome` | 仅构建 Chrome | `.output/chrome-mv3/` |
| `npm run build:edge` | 仅构建 Edge | `.output/chrome-mv3-edge/` |
| `npm run build:safari` | 仅构建 Safari | `.output/safari-mv2/` |
| `npm run build:firefox` | 仅构建 Firefox | `.output/firefox-mv2/` |
| `npm run zip` | 打包为 ZIP 分发 | `.output/*.zip` |

### WXT 配置说明

扩展使用 WXT 框架构建，关键配置在 `wxt.config.ts`：

```typescript
export default defineConfig({
  srcDir,
  outBaseDir: '.output',
  outDirTemplate: "{{browser}}-mv{{manifestVersion}}{{modeSuffix}}",
  entrypointsDir: path.join(srcDir, 'entrypoints'),
  publicDir: path.join(rootDir, 'public'),
  alias: {
    '@': srcDir,
    '@api': path.join(srcDir, 'api'),
    // ...
  },
  manifest: ({ mode }) => ({
    name: 'Tex2Doc - LaTeX to Word',
    permissions: ['storage', 'downloads', 'contextMenus', 'notifications'],
    // ...
  }),
});
```

### 技术架构

```
┌─────────────────────────────────────────────────────────────┐
│                      Browser                                │
├─────────────┬─────────────┬─────────────┬─────────────────┤
│   Popup     │   Options   │  SidePanel  │ Content Scripts │
└──────┬──────┴──────┬──────┴──────┬──────┴────────┬────────┘
       │              │             │               │
       └──────────────┴──────┬──────┴───────────────┘
                             │
                    ┌───────┴───────┐
                    │   Background   │
                    │ Service Worker │
                    └───────┬───────┘
                            │
       ┌────────────────────┼────────────────────┐
       │                    │                    │
┌──────▼──────┐     ┌──────▼──────┐    ┌──────▼──────┐
│  IndexedDB   │     │  WASM       │    │  API Client │
│  (Jobs)     │     │  (Local)    │    │  (Cloud)    │
└─────────────┘     └─────────────┘    └──────┬──────┘
                                               │
                                        ┌──────▼──────┐
                                        │  Tex2Doc    │
                                        │  API Server │
                                        └─────────────┘
```

---

## 故障排除

### 扩展无法加载

**问题**：浏览器提示 "Manifest file is missing or unreadable"

**解决**：
1. 确认选择的是正确的输出文件夹
   - Chrome: `.output/chrome-mv3/`
   - Edge: `.output/chrome-mv3-edge/`
   - Safari: `.output/safari-mv2/`
2. 检查是否包含 `manifest.json`
3. 重新运行对应浏览器的构建命令

### 转换失败

**问题**：转换过程中报错或卡住

**解决**：
1. 检查网络连接
2. 确认 LaTeX 语法正确
3. 尝试使用本地转换模式
4. 查看扩展后台日志（右键 → 检查 → Console）

### 图标不显示

**问题**：扩展图标显示为灰色方块

**解决**：
1. 确认 `public/icons/` 目录存在
2. 确认图标尺寸正确：16x16, 32x32, 48x48, 128x128
3. 重新构建扩展

### 路径别名解析错误

**问题**：构建时报错找不到模块

**解决**：
1. 确保 `wxt.config.ts` 中正确配置 `srcDir`
2. 确保 `alias` 配置使用绝对路径
3. 运行 `npx wxt prepare` 重新生成类型定义

### 后台脚本错误

**问题**：`defineBackgroundScript is not defined`

**解决**：
1. 使用正确的函数名 `defineBackground`
2. 确保在 `wxt.config.ts` 中设置 `srcDir`
3. WXT 0.18+ 使用 `defineBackground` 而非 `defineBackgroundScript`

### Safari 侧边面板不工作

**问题**：Safari 上找不到侧边面板功能

**解决**：这是正常现象。Safari 不支持 Manifest V3 的 sidePanel API，该功能在 Safari 上不可用。

---

## 更新日志

### v0.1.0 (2026-06-27)
- 初始版本发布
- 支持 LaTeX 到 Word 转换
- 支持本地和云端转换
- 支持 Overleaf 和 arXiv 集成
- 账户管理和用量追踪
- 多浏览器支持（Chrome、Edge、Safari）
