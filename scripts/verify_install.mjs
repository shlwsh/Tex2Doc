#!/usr/bin/env node
/**
 * 自动安装 Playwright Chromium 浏览器。
 *
 * 在首次跑 verify 之前调用一次：
 *     node scripts/verify_install.mjs
 *
 * 国内/CI 环境可设置 PLAYWRIGHT_DOWNLOAD_HOST 走镜像。
 */
import { spawnSync } from "node:child_process";

const r = spawnSync(
  "npx",
  ["--yes", "playwright", "install", "--with-deps", "chromium"],
  { stdio: "inherit", env: process.env }
);
process.exit(r.status ?? 1);
