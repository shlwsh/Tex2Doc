#!/usr/bin/env node
/**
 * Doc-engine Flutter Web 本地服务器
 *
 * 用途：serve `flutter_app/build/web` 目录，供 Playwright 集成测试访问。
 *
 * 特性：
 * - 零依赖（仅用 Node 内置 http / fs / path）
 * - 默认端口 2627（与项目本地服务端口段保持一致）
 * - 简单 MIME 推断
 * - CORS 头（开放，避免 Flutter web + WASM 在跨源场景被拦）
 * - 支持目录默认 index.html
 *
 * 用法：
 *   node scripts/serve_flutter_web.mjs
 *   PORT=2627 node scripts/serve_flutter_web.mjs
 */
import { createServer } from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import { resolve, join, dirname, extname } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '..', 'flutter_app', 'build', 'web');
const PORT = Number(process.env.PORT ?? 2627);
const HOST = process.env.HOST ?? '127.0.0.1';

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js':   'application/javascript; charset=utf-8',
  '.mjs':  'application/javascript; charset=utf-8',
  '.css':  'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.svg':  'image/svg+xml',
  '.png':  'image/png',
  '.jpg':  'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.gif':  'image/gif',
  '.webp': 'image/webp',
  '.ico':  'image/x-icon',
  '.wasm': 'application/wasm',
  '.txt':  'text/plain; charset=utf-8',
  '.map':  'application/json',
};

function mimeOf(p) {
  return MIME[extname(p).toLowerCase()] ?? 'application/octet-stream';
}

async function serveFile(res, absPath) {
  try {
    const s = await stat(absPath);
    if (s.isDirectory()) {
      return serveFile(res, join(absPath, 'index.html'));
    }
    const buf = await readFile(absPath);
    res.writeHead(200, {
      'content-type': mimeOf(absPath),
      'content-length': buf.length,
      'cache-control': 'no-store',
      'access-control-allow-origin': '*',
    });
    res.end(buf);
  } catch (e) {
    res.writeHead(404, { 'content-type': 'text/plain; charset=utf-8' });
    res.end(`404 Not Found: ${e.message ?? e}\n`);
  }
}

const server = createServer((req, res) => {
  // 简单路由：把 URL 转成 ROOT 下的相对路径
  let urlPath;
  try {
    urlPath = decodeURIComponent(new URL(req.url, `http://${req.headers.host}`).pathname);
  } catch (e) {
    res.writeHead(400);
    res.end('bad URL');
    return;
  }
  if (urlPath.includes('..')) {
    res.writeHead(400);
    res.end('bad path');
    return;
  }
  if (urlPath === '/' || urlPath === '') urlPath = '/index.html';
  const abs = join(ROOT, urlPath);
  serveFile(res, abs);
});

server.listen(PORT, HOST, () => {
  process.stdout.write(
    `[serve] Doc-engine Flutter Web 已就绪：http://${HOST}:${PORT}/\n` +
    `[serve] 静态根：${ROOT}\n` +
    `[serve] 按 Ctrl+C 停止。\n`
  );
});
