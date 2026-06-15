#!/usr/bin/env node
/**
 * e2e_server.mjs — Doc-engine crates/server 端到端冒烟
 *
 * 复用 Rust 集成测试（ephemeral port）验证全部路径。
 * 这比 Node.js 起进程更稳。
 *
 * 用法：node scripts/e2e_server.mjs
 */

import { spawn } from 'node:child_process';
import { setTimeout as sleep } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';
import { dirname } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = __dirname + '/..';

async function main() {
  console.log('[e2e-server] running: cargo test -p doc-server');
  const p = spawn('cargo', ['test', '-p', 'doc-server'], { cwd: repoRoot });
  let stdout = '';
  let stderr = '';
  p.stdout.on('data', (b) => { process.stdout.write(b); stdout += b; });
  p.stderr.on('data', (b) => { process.stderr.write(b); stderr += b; });
  const code = await new Promise((res) => p.on('close', res));
  if (code === 0) {
    console.log('[e2e-server] PASS: all doc-server tests passed');
  } else {
    console.error(`[e2e-server] FAIL: cargo test exited ${code}`);
  }
  process.exit(code);
}

main();
