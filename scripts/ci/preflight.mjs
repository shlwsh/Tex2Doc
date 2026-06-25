#!/usr/bin/env node
/**
 * Local pre-submit checks for development.
 *
 * GitHub CI intentionally runs only fast formatting/static checks. Keep the
 * full test suite here so regressions are caught before pushing. Use:
 *   npm run ci:preflight
 */
import { spawnSync } from 'node:child_process';

const isWindows = process.platform === 'win32';

function run(command, args, options = {}) {
  const printable = [command, ...args].join(' ');
  process.stdout.write(`\n[preflight] ${printable}\n`);
  const result = spawnSync(command, args, {
    cwd: options.cwd ?? process.cwd(),
    env: { ...process.env, ...options.env },
    shell: isWindows,
    stdio: 'inherit',
  });
  if (result.status !== 0) {
    process.stderr.write(`[preflight] failed: ${printable}\n`);
    process.exit(result.status ?? 1);
  }
}

function commandExists(command) {
  const probe = spawnSync(isWindows ? 'where' : 'command', isWindows ? [command] : ['-v', command], {
    shell: isWindows,
    stdio: 'ignore',
  });
  return probe.status === 0;
}

run('cargo', ['fmt', '--all', '--', '--check']);
run('cargo', ['clippy', '--workspace', '--all-targets', '--', '-D', 'warnings']);
run('cargo', ['test', '--workspace', '--all-targets', '--exclude', 'doc-server', '--', '--test-threads=1']);
run('cargo', ['test', '-p', 'doc-server', '--lib', '--bin', 'doc-server', '--', '--test-threads=1']);

if (process.env.DATABASE_URL) {
  run('cargo', ['test', '-p', 'doc-server', '--test', 'api', '--', '--test-threads=1']);
} else {
  process.stdout.write('\n[preflight] DATABASE_URL not set; skipping doc-server API integration tests.\n');
}

if (commandExists('flutter')) {
  run('flutter', ['pub', 'get'], { cwd: 'flutter_app' });
  run('flutter', ['analyze'], { cwd: 'flutter_app' });
  run('flutter', ['test'], { cwd: 'flutter_app' });
} else {
  process.stdout.write('\n[preflight] flutter not found; skipping Flutter analyze/test.\n');
}
