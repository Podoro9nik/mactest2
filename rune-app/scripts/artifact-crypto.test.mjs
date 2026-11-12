import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtemp, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
const script = join(dirname(fileURLToPath(import.meta.url)), 'artifact-crypto.mjs');
const fixtureKey = 'offline-unit-test-build-secret';
function run(operation, input, output, key = fixtureKey) {
  return spawnSync(process.execPath, [script, operation, input, output], {
    env: { ...process.env, CHIIKAWA_EMBEDDED_MCP_KEY: key }, encoding: 'utf8', windowsHide: true,
  });
}
test('personal artifact roundtrip, authentication and overwrite protection', async (t) => {
  const prefix = join(tmpdir(), 'chiikawa-artifact-test-');
  const dir = await mkdtemp(prefix);
  t.after(async () => {
    assert.ok(dir.startsWith(prefix));
    await rm(dir, { recursive: true, force: true });
  });
  const input = join(dir, 'installer'), encrypted = join(dir, 'installer.enc'), restored = join(dir, 'restored');
  const original = Buffer.from('Offline installer fixture\0\xff', 'latin1');
  await writeFile(input, original);
  assert.equal(run('encrypt', input, encrypted).status, 0);
  assert.equal(run('decrypt', encrypted, restored).status, 0);
  assert.deepEqual(await readFile(restored), original);
  assert.notEqual(run('encrypt', input, encrypted).status, 0);
  assert.notEqual(run('encrypt', input, input).status, 0);
  assert.deepEqual(await readFile(input), original);
  const wrong = join(dir, 'wrong-key');
  assert.notEqual(run('decrypt', encrypted, wrong, 'incorrect-build-key').status, 0);
  assert.ok(!(await readdir(dir)).includes('wrong-key'));
  const bytes = await readFile(encrypted);
  bytes[40] ^= 0x40;
  const tampered = join(dir, 'tampered.enc'), rejected = join(dir, 'tampered-output');
  await writeFile(tampered, bytes);
  assert.notEqual(run('decrypt', tampered, rejected).status, 0);
  assert.ok(!(await readdir(dir)).includes('tampered-output'));
});
