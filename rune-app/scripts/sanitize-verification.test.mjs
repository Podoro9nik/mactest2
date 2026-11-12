import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdir, mkdtemp, readFile, realpath, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
const script = join(dirname(fileURLToPath(import.meta.url)), 'sanitize-verification.mjs');
test('failure reports redact credentials and refuse unexpected binaries', async (t) => {
  const prefix = join(await realpath(tmpdir()), 'chiikawa-report-test-');
  const root = await mkdtemp(prefix);
  t.after(async () => { assert.ok(root.startsWith(prefix)); await rm(root, { recursive: true, force: true }); });
  const reports = join(root, 'reports');
  await mkdir(reports);
  const secret = 'fixture-build-secret';
  const token = 'ek_mcp_' + 'fixture_not_a_real_credential_123';
  const github = 'ghp_' + 'fixture_not_a_real_credential_123';
  await writeFile(join(reports, 'native-smoke.log'), `${secret}\n${token}\n${github}\nframe 10\n`);
  const run = path => spawnSync(process.execPath, [script, path], {
    env: { ...process.env, CHIIKAWA_EMBEDDED_MCP_KEY: secret }, encoding: 'utf8', windowsHide: true,
  });
  const result = run(reports);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(await readFile(join(reports, 'native-smoke.log'), 'utf8'), '[redacted]\n[redacted]\n[redacted]\nframe 10\n');
  assert.equal(JSON.parse(await readFile(join(reports, 'report-safety.json'), 'utf8')).sanitized, true);
  const unsafe = join(root, 'unsafe');
  await mkdir(unsafe);
  await writeFile(join(unsafe, 'program.exe'), secret);
  assert.notEqual(run(unsafe).status, 0);
  await assert.rejects(readFile(join(unsafe, 'report-safety.json')), { code: 'ENOENT' });
});
