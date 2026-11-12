// Offline checks exercise the shipped binary without generating or purchasing images.
import { spawnSync } from 'node:child_process';
import { mkdir, readFile, readdir, stat, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';

const [binaryArgument, outputArgument] = process.argv.slice(2);
if (!binaryArgument || !outputArgument) throw new Error('Use verify-package.mjs EXECUTABLE OUTPUT_DIRECTORY');
const binary = resolve(binaryArgument), output = resolve(outputArgument);
const env = { ...process.env, CHIIKAWIZATOR_OFFLINE: '1' };
await mkdir(output, { recursive: true });
function redact(text) {
  let result = text;
  if (env.CHIIKAWA_EMBEDDED_MCP_KEY) result = result.split(env.CHIIKAWA_EMBEDDED_MCP_KEY).join('[redacted]');
  return result.replace(/(?:ek_mcp_|ek_live_)[A-Za-z0-9_-]{16,}/g, '[redacted]');
}
async function run(name, args, timeout = 60_000) {
  const result = spawnSync(binary, args, { env, timeout, encoding: 'utf8', windowsHide: true, maxBuffer: 16 * 1024 * 1024 });
  await writeFile(join(output, `${name}.log`), redact(`${result.stdout || ''}\n${result.stderr || ''}`));
  if (result.error || result.status !== 0) {
    throw new Error(`${name} failed (${result.error?.code || result.status}); see the sanitized verification log.`);
  }
  return result.stdout;
}
const selfTestText = await run('self-test', ['--self-test']);
const selfTest = JSON.parse(selfTestText.trim().replace(/^\uFEFF/, ''));
if (selfTest.ok !== true) throw new Error('Application self-test did not report ok:true.');
if (selfTest.hasEmbeddedKey !== true) throw new Error('The personal installer does not contain its configured access.');
if (selfTest.renderer !== 'Rune' || selfTest.version !== '4.0.0') throw new Error('Unexpected application renderer or version.');
const characterCount = Array.isArray(selfTest.characters) ? selfTest.characters.length : selfTest.characters;
if (characterCount !== 8) throw new Error('The application does not include all eight characters.');
await writeFile(join(output, 'self-test.json'), redact(JSON.stringify(selfTest, null, 2)) + '\n');
for (const language of ['ru', 'en', 'ja']) {
  const scene = `home-${language}`;
  const snap = join(output, `${scene}.png`), tree = join(output, `${scene}.json`);
  await run(scene, ['--scene', scene, '--snap', snap, '--tree', tree, '--size', '1200x800', '--scale', '1', '--at', '0.5', '--realtime']);
  const bytes = await readFile(snap);
  if (bytes.length < 1024 || !bytes.subarray(0, 8).equals(Buffer.from([137,80,78,71,13,10,26,10]))) {
    throw new Error(`Invalid image for ${scene}.`);
  }
  JSON.parse(await readFile(tree, 'utf8'));
}
const tracePath = join(output, 'native-smoke.jsonl');
await run('native-smoke', ['--native', '--scene', 'home-ru', '--trace', tracePath, '--frames', '10']);
const trace = (await readFile(tracePath, 'utf8')).trim().split(/\r?\n/).map(line => JSON.parse(line));
if (trace[0]?.clock !== 'wall' || trace.length < 11) throw new Error('Native smoke test did not present ten frames.');
// App-generated text is inspected as well as captured stdout before public CI upload.
for (const entry of await readdir(output)) {
  if (!/\.(?:json|jsonl|log)$/.test(entry)) continue;
  const path = join(output, entry), text = await readFile(path, 'utf8');
  if (redact(text) !== text) {
    await writeFile(path, redact(text));
    throw new Error('Credential content was removed from a verification artifact; inspect the application diagnostics.');
  }
}
await writeFile(join(output, 'verification.json'), JSON.stringify({
  ok: true, offline: true, nativeFrames: trace.length - 1,
  scenes: ['home-ru', 'home-en', 'home-ja'], executableBytes: (await stat(binary)).size,
}, null, 2) + '\n');
console.log('Offline self-test, three language snapshots and native window smoke test passed.');
