import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { cp, mkdir, mkdtemp, readFile, readdir, realpath, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
const script = join(dirname(fileURLToPath(import.meta.url)), 'stage-source.mjs');
const fakeCredential = 'ek_mcp_' + 'fixture_not_a_real_credential_123';
async function fixture(t) {
  // macOS exposes /var as /private/var; Node resolves a script's symlinked entry path.
  const prefix = join(await realpath(tmpdir()), 'chiikawa-stage-test-');
  const root = await mkdtemp(prefix);
  t.after(async () => {
    assert.ok(root.startsWith(prefix));
    await rm(root, { recursive: true, force: true });
  });
  async function put(rel, text) {
    const file = join(root, rel);
    await mkdir(dirname(file), { recursive: true });
    await writeFile(file, text);
  }
  await put('.github/workflows/rune-build.yml', 'name: fixture\n');
  await put('rune-app/src/main.rs', 'fn main() {}\n');
  await mkdir(join(root, 'rune-app/scripts'), { recursive: true });
  const stagedScript = join(root, 'rune-app/scripts/stage-source.mjs');
  await cp(script, stagedScript);
  const run = (target = join(root, 'build/staging')) => spawnSync(process.execPath, [stagedScript, target], {
    encoding: 'utf8', windowsHide: true,
    env: { ...process.env, CHIIKAWA_EMBEDDED_MCP_KEY: '' },
  });
  return { root, put, run, target: join(root, 'build/staging') };
}
test('source staging excludes local credentials, framework and executable outputs', async (t) => {
  const { root, put, run, target } = await fixture(t);
  for (const rel of ['vendor/rune/private.txt', 'target/program.exe', 'dist/setup.exe', '.env', 'local.secret.json'])
    await put(`rune-app/${rel}`, fakeCredential);
  const result = run();
  assert.equal(result.status, 0, result.stderr);
  assert.equal(await readFile(join(target, 'rune-app/src/main.rs'), 'utf8'), 'fn main() {}\n');
  const manifest = JSON.parse(await readFile(join(target, 'rune-source-manifest.json'), 'utf8'));
  assert.deepEqual(manifest.files.sort(), ['.github/workflows/rune-build.yml', 'README.md', 'rune-app/scripts/stage-source.mjs', 'rune-app/src/main.rs']);
  assert.ok((await readFile(join(target, 'README.md'), 'utf8')).includes('rune-app/README.md'));
  assert.equal(run(join(root, 'outside-staging')).status, 1);
});
test('credential scan rejects the entire selection before copying source files', async (t) => {
  const { put, run, target } = await fixture(t);
  await put('rune-app/src/leak.rs', fakeCredential);
  const result = run();
  assert.notEqual(result.status, 0);
  assert.ok(result.stderr.includes('Credential found in source'));
  assert.ok(!result.stderr.includes(fakeCredential));
  assert.deepEqual(await readdir(target), []);
});
test('nested destination junctions cannot write outside the staging checkout', async (t) => {
  const { root, run, target } = await fixture(t);
  const outside = join(root, 'unrelated-files');
  await mkdir(outside);
  await mkdir(target, { recursive: true });
  await symlink(outside, join(target, 'rune-app'), process.platform === 'win32' ? 'junction' : 'dir');
  const result = run();
  assert.notEqual(result.status, 0);
  assert.ok(result.stderr.includes('Unsafe staging directory entry'));
  assert.deepEqual(await readdir(outside), []);
});
