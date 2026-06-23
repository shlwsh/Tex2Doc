# Doc-engine Chrome 扩展（Manifest V3）

> 状态：M1 预留骨架。

## 计划

- M11-M12：Popup 360px、Content Script 上下文菜单、Service Worker 调 WASM。

## 临时结构

```
extension/
├── manifest.json     # MV3 配置
├── background.js     # Service Worker
├── popup/            # 弹窗
│   ├── popup.html
│   └── popup.js
└── content/          # 上下文菜单
    └── content.js
```

详细规范见 `docs/Doc-engine_LaTeX-to-DOCX_技术方案_v2.0_20260614.md` §5.2。
