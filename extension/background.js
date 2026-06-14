// Doc-engine MV3 Service Worker（占位）。
// M11-M12 实现：
// 1. 监听 contextMenus.create('doc-engine-convert', ...)
// 2. onClicked → 调用 wasm-bindgen 转换选区 → 写入剪贴板

self.addEventListener('install', () => {
  // 占位：M11-M12 接入 wasm-pack build 产物
});

self.addEventListener('activate', () => {
  // 占位
});
